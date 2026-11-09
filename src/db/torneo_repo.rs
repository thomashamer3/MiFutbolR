//! Persistencia de torneos (IMP-11).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::torneos::{PartidoTorneoRow, TipoTorneo, Torneo, TorneoEquipo};

/// Lista los torneos.
pub fn list(conn: &Connection) -> Result<Vec<Torneo>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo_torneo, cantidad_equipos, fase_actual
           FROM torneo ORDER BY id DESC",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(Torneo {
            id: row.get(0)?,
            nombre: row.get(1)?,
            tipo: TipoTorneo::from_i64(row.get(2)?),
            cantidad_equipos: row.get(3)?,
            fase_actual: row.get(4)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene un torneo.
pub fn get(conn: &Connection, id: i64) -> Result<Torneo, DbError> {
    conn.query_row(
        "SELECT id, nombre, tipo_torneo, cantidad_equipos, fase_actual
           FROM torneo WHERE id = ?1",
        params![id],
        |row| {
            Ok(Torneo {
                id: row.get(0)?,
                nombre: row.get(1)?,
                tipo: TipoTorneo::from_i64(row.get(2)?),
                cantidad_equipos: row.get(3)?,
                fase_actual: row.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "torneo",
        id,
    })
}

/// Crea un torneo y devuelve su id.
pub fn insert(conn: &Connection, nombre: &str, tipo: TipoTorneo) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO torneo (nombre, tipo_torneo) VALUES (?1, ?2)",
        params![nombre, tipo.as_i64()],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza nombre y tipo.
pub fn update(conn: &Connection, id: i64, nombre: &str, tipo: TipoTorneo) -> Result<(), DbError> {
    conn.execute(
        "UPDATE torneo SET nombre = ?1, tipo_torneo = ?2 WHERE id = ?3",
        params![nombre, tipo.as_i64(), id],
    )?;
    Ok(())
}

/// Borra un torneo (el fixture y las asociaciones caen por cascada).
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM torneo WHERE id = ?1", params![id])?;
    Ok(())
}

/// Equipos asociados.
pub fn equipos(conn: &Connection, torneo_id: i64) -> Result<Vec<TorneoEquipo>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.nombre
           FROM equipo_torneo et JOIN equipo e ON e.id = et.equipo_id
          WHERE et.torneo_id = ?1
          ORDER BY e.nombre",
    )?;
    let filas = stmt.query_map(params![torneo_id], |row| {
        Ok(TorneoEquipo {
            id: row.get(0)?,
            nombre: row.get(1)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Asocia todos los equipos activos que aún no estén en el torneo.
pub fn asociar_todos(conn: &Connection, torneo_id: i64) -> Result<usize, DbError> {
    let nuevos = conn.execute(
        "INSERT OR IGNORE INTO equipo_torneo (torneo_id, equipo_id)
         SELECT ?1, id FROM equipo WHERE activa = 1",
        params![torneo_id],
    )?;
    actualizar_cantidad(conn, torneo_id)?;
    Ok(nuevos)
}

/// Recalcula la cantidad de equipos del torneo.
pub fn actualizar_cantidad(conn: &Connection, torneo_id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE torneo
            SET cantidad_equipos = (SELECT COUNT(*) FROM equipo_torneo WHERE torneo_id = ?1)
          WHERE id = ?1",
        params![torneo_id],
    )?;
    Ok(())
}

/// Fixture del torneo.
pub fn partidos(conn: &Connection, torneo_id: i64) -> Result<Vec<PartidoTorneoRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT pt.id, pt.equipo1_id, e1.nombre, pt.equipo2_id, e2.nombre,
                pt.goles_equipo1, pt.goles_equipo2,
                IFNULL(pt.estado, 'Pendiente'), pt.fase
           FROM partido_torneo pt
           JOIN equipo e1 ON e1.id = pt.equipo1_id
           JOIN equipo e2 ON e2.id = pt.equipo2_id
          WHERE pt.torneo_id = ?1
          ORDER BY pt.id",
    )?;
    let filas = stmt.query_map(params![torneo_id], |row| {
        Ok(PartidoTorneoRow {
            id: row.get(0)?,
            local_id: row.get(1)?,
            local: row.get(2)?,
            visitante_id: row.get(3)?,
            visitante: row.get(4)?,
            goles_local: row.get(5)?,
            goles_visitante: row.get(6)?,
            estado: row.get(7)?,
            fase: row.get(8)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Borra el fixture actual.
pub fn borrar_fixture(conn: &Connection, torneo_id: i64) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM partido_torneo WHERE torneo_id = ?1",
        params![torneo_id],
    )?;
    Ok(())
}

/// Inserta el fixture en una transacción.
pub fn insertar_fixture(
    conn: &Connection,
    torneo_id: i64,
    partidos: &[(i64, i64)],
) -> Result<usize, DbError> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO partido_torneo (torneo_id, equipo1_id, equipo2_id, estado, fase)
             VALUES (?1, ?2, ?3, 'Pendiente', 'Fase de Grupos')",
        )?;
        for (local, visitante) in partidos {
            stmt.execute(params![torneo_id, local, visitante])?;
        }
    }
    tx.commit()?;
    Ok(partidos.len())
}

/// Registra el resultado de un partido del fixture.
pub fn registrar_resultado(
    conn: &Connection,
    partido_id: i64,
    goles_local: i64,
    goles_visitante: i64,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE partido_torneo
            SET goles_equipo1 = ?1, goles_equipo2 = ?2, estado = 'Jugado'
          WHERE id = ?3",
        params![goles_local, goles_visitante, partido_id],
    )?;
    Ok(())
}

/// Obtiene un partido del fixture.
pub fn get_partido(conn: &Connection, id: i64) -> Result<(String, String, i64, i64), DbError> {
    conn.query_row(
        "SELECT e1.nombre, e2.nombre, pt.goles_equipo1, pt.goles_equipo2
           FROM partido_torneo pt
           JOIN equipo e1 ON e1.id = pt.equipo1_id
           JOIN equipo e2 ON e2.id = pt.equipo2_id
          WHERE pt.id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "partido_torneo",
        id,
    })
}
