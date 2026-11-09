//! Repositorio de equipos y jugadores (§11.3).

use rusqlite::{Connection, Row, Transaction, params};

use super::error::DbError;
use crate::domain::models::{
    Equipo, EquipoDependencias, EquipoFilter, EquipoRow, Jugador, Posicion, TipoEquipo, TipoFutbol,
};

const COLUMNS: &str =
    "id, nombre, tipo, tipo_futbol, num_jugadores, partido_id, activa, imagen_ruta";

/// Lista equipos con el conteo de jugadores.
pub fn list(conn: &Connection, filter: &EquipoFilter) -> Result<Vec<EquipoRow>, DbError> {
    let sql = format!(
        "SELECT e.id, e.nombre, e.tipo, e.tipo_futbol, e.num_jugadores, e.activa,
                (SELECT COUNT(*) FROM jugador j WHERE j.equipo_id = e.id) AS jugadores,
                e.imagen_ruta
           FROM equipo e
          WHERE (?1 = 0 OR e.activa = CASE ?1 WHEN 1 THEN 1 ELSE 0 END)
            AND (?2 = '' OR e.nombre LIKE '%' || ?2 || '%' COLLATE NOCASE)
          ORDER BY {}",
        filter.orden.sql_order()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![filter.estado.sql_code(), filter.texto.trim()],
            |row| {
                Ok(EquipoRow {
                    id: row.get(0)?,
                    nombre: row.get(1)?,
                    tipo: TipoEquipo::try_from(row.get::<_, i64>(2)?).unwrap_or(TipoEquipo::Fijo),
                    tipo_futbol: TipoFutbol::try_from(row.get::<_, i64>(3)?)
                        .unwrap_or(TipoFutbol::F5),
                    num_jugadores: row.get(4)?,
                    activa: row.get::<_, i64>(5)? != 0,
                    jugadores_count: row.get(6)?,
                    imagen_ruta: row.get(7)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene un equipo con sus jugadores.
pub fn get(conn: &Connection, id: i64) -> Result<Equipo, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM equipo WHERE id = ?1");
    let mut equipo: Equipo =
        conn.query_row(&sql, [id], map_equipo)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                    entidad: "Equipo",
                    id,
                },
                other => other.into(),
            })?;
    equipo.jugadores = list_jugadores(conn, id)?;
    Ok(equipo)
}

/// Jugadores de un equipo ordenados por dorsal.
pub fn list_jugadores(conn: &Connection, equipo_id: i64) -> Result<Vec<Jugador>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, equipo_id, nombre, numero, posicion, es_capitan
           FROM jugador WHERE equipo_id = ?1 ORDER BY numero",
    )?;
    let rows = stmt
        .query_map([equipo_id], |row| {
            Ok(Jugador {
                id: row.get(0)?,
                equipo_id: row.get(1)?,
                nombre: row.get(2)?,
                numero: row.get(3)?,
                posicion: Posicion::try_from(row.get::<_, i64>(4)?).unwrap_or(Posicion::Arquero),
                es_capitan: row.get::<_, i64>(5)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Inserta un equipo y sus jugadores en una transacción.
pub fn insert(conn: &mut Connection, equipo: &Equipo) -> Result<i64, DbError> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO equipo
            (nombre, tipo, tipo_futbol, num_jugadores, partido_id, activa, imagen_ruta, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now','localtime'))",
        params![
            equipo.nombre,
            equipo.tipo.as_i64(),
            equipo.tipo_futbol.as_i64(),
            equipo.num_jugadores,
            equipo.partido_id,
            i64::from(equipo.activa),
            equipo.imagen_ruta,
        ],
    )?;
    let id = tx.last_insert_rowid();
    insert_jugadores(&tx, id, &equipo.jugadores)?;
    tx.commit()?;
    Ok(id)
}

/// Actualiza la cabecera de un equipo (los jugadores se gestionan aparte).
pub fn update(conn: &Connection, equipo: &Equipo) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE equipo
            SET nombre = ?1, tipo = ?2, tipo_futbol = ?3, num_jugadores = ?4,
                partido_id = ?5, activa = ?6, imagen_ruta = ?7,
                updated_at = datetime('now','localtime')
          WHERE id = ?8",
        params![
            equipo.nombre,
            equipo.tipo.as_i64(),
            equipo.tipo_futbol.as_i64(),
            equipo.num_jugadores,
            equipo.partido_id,
            i64::from(equipo.activa),
            equipo.imagen_ruta,
            equipo.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Equipo",
            id: equipo.id,
        });
    }
    Ok(())
}

/// Activa/desactiva un equipo.
pub fn set_active(conn: &Connection, id: i64, active: bool) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE equipo SET activa = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![i64::from(active), id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Equipo",
            id,
        });
    }
    Ok(())
}

/// Elimina un equipo (los jugadores caen por CASCADE).
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM equipo WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Equipo",
            id,
        });
    }
    Ok(())
}

/// Dependencias de torneo/historial.
pub fn dependencias(conn: &Connection, id: i64) -> Result<EquipoDependencias, DbError> {
    let partido_torneo: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido_torneo WHERE equipo1_id = ?1 OR equipo2_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let equipo_historial: i64 = conn.query_row(
        "SELECT COUNT(*) FROM equipo_historial WHERE equipo_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let torneos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM torneo WHERE equipo_fijo_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(EquipoDependencias {
        partido_torneo,
        equipo_historial,
        torneos,
    })
}

/// Inserta un jugador.
pub fn insert_jugador(conn: &Connection, jugador: &Jugador) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO jugador (equipo_id, nombre, numero, posicion, es_capitan)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            jugador.equipo_id,
            jugador.nombre,
            jugador.numero,
            jugador.posicion.as_i64(),
            i64::from(jugador.es_capitan),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un jugador.
pub fn update_jugador(conn: &Connection, jugador: &Jugador) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE jugador
            SET nombre = ?1, numero = ?2, posicion = ?3, es_capitan = ?4
          WHERE id = ?5",
        params![
            jugador.nombre,
            jugador.numero,
            jugador.posicion.as_i64(),
            i64::from(jugador.es_capitan),
            jugador.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Jugador",
            id: jugador.id,
        });
    }
    Ok(())
}

/// Elimina un jugador.
pub fn delete_jugador(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM jugador WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Jugador",
            id,
        });
    }
    Ok(())
}

/// Quita la capitanía a todos los jugadores del equipo.
pub fn clear_capitan(conn: &Connection, equipo_id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE jugador SET es_capitan = 0 WHERE equipo_id = ?1",
        [equipo_id],
    )?;
    Ok(())
}

/// Comprueba si el dorsal ya existe en el equipo.
pub fn existe_dorsal(
    conn: &Connection,
    equipo_id: i64,
    numero: i64,
    exclude_id: Option<i64>,
) -> Result<bool, DbError> {
    let exclude = exclude_id.unwrap_or(0);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM jugador
             WHERE equipo_id = ?1 AND numero = ?2 AND (?3 = 0 OR id <> ?3)
        )",
        params![equipo_id, numero, exclude],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn insert_jugadores(
    tx: &Transaction<'_>,
    equipo_id: i64,
    jugadores: &[Jugador],
) -> rusqlite::Result<()> {
    for jugador in jugadores {
        tx.execute(
            "INSERT INTO jugador (equipo_id, nombre, numero, posicion, es_capitan)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                equipo_id,
                jugador.nombre,
                jugador.numero,
                jugador.posicion.as_i64(),
                i64::from(jugador.es_capitan),
            ],
        )?;
    }
    Ok(())
}

fn map_equipo(row: &Row<'_>) -> rusqlite::Result<Equipo> {
    Ok(Equipo {
        id: row.get(0)?,
        nombre: row.get(1)?,
        tipo: TipoEquipo::try_from(row.get::<_, i64>(2)?).unwrap_or(TipoEquipo::Fijo),
        tipo_futbol: TipoFutbol::try_from(row.get::<_, i64>(3)?).unwrap_or(TipoFutbol::F5),
        num_jugadores: row.get(4)?,
        partido_id: row.get(5)?,
        activa: row.get::<_, i64>(6)? != 0,
        imagen_ruta: row.get(7)?,
        jugadores: Vec::new(),
    })
}
