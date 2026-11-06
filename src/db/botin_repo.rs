//! Repositorio de botines (§10.3).

use rusqlite::{Connection, Row, params};

use super::error::DbError;
use crate::domain::models::{Botin, BotinDependencias, BotinFilter, BotinRow};

const COLUMNS: &str = "id, nombre, imagen_ruta, activa, sorteada";

/// Lista botines con uso y marca de predeterminado.
pub fn list(conn: &Connection, filter: &BotinFilter) -> Result<Vec<BotinRow>, DbError> {
    let sql = format!(
        "SELECT b.id, b.nombre, b.activa, b.sorteada,
                COUNT(p.id) AS partidos,
                (b.id = (SELECT botin_predeterminado FROM settings WHERE id = 1)) AS predeterminado
           FROM botin b
           LEFT JOIN partido p ON p.botin_id = b.id
          WHERE (?1 = 0 OR b.activa = CASE ?1 WHEN 1 THEN 1 ELSE 0 END)
            AND (?2 = '' OR b.nombre LIKE '%' || ?2 || '%' COLLATE NOCASE)
          GROUP BY b.id
          ORDER BY {}",
        filter.orden.sql_order()
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(
            params![filter.estado.sql_code(), filter.texto.trim()],
            |row| {
                Ok(BotinRow {
                    id: row.get(0)?,
                    nombre: row.get(1)?,
                    activa: row.get::<_, i64>(2)? != 0,
                    sorteada: row.get::<_, i64>(3)? != 0,
                    partidos: row.get(4)?,
                    predeterminado: row.get::<_, i64>(5)? != 0,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene un botín por id.
pub fn get(conn: &Connection, id: i64) -> Result<Botin, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM botin WHERE id = ?1");
    conn.query_row(&sql, [id], map_botin)
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
                entidad: "Botin",
                id,
            },
            other => other.into(),
        })
}

/// Inserta un botín.
pub fn insert(conn: &Connection, botin: &Botin) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO botin (nombre, imagen_ruta, activa, sorteada)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            botin.nombre,
            botin.imagen_ruta,
            i64::from(botin.activa),
            i64::from(botin.sorteada),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un botín.
pub fn update(conn: &Connection, botin: &Botin) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE botin SET nombre = ?1, imagen_ruta = ?2, activa = ?3, sorteada = ?4 WHERE id = ?5",
        params![
            botin.nombre,
            botin.imagen_ruta,
            i64::from(botin.activa),
            i64::from(botin.sorteada),
            botin.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Botin",
            id: botin.id,
        });
    }
    Ok(())
}

/// Activa/desactiva un botín.
pub fn set_active(conn: &Connection, id: i64, active: bool) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE botin SET activa = ?1 WHERE id = ?2",
        params![i64::from(active), id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Botin",
            id,
        });
    }
    Ok(())
}

/// Elimina un botín y limpia el predeterminado si era el elegido.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM botin WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Botin",
            id,
        });
    }
    conn.execute(
        "UPDATE settings SET botin_predeterminado = 0 WHERE id = 1 AND botin_predeterminado = ?1",
        [id],
    )?;
    Ok(())
}

/// Dependencias de un botín.
pub fn dependencias(conn: &Connection, id: i64) -> Result<BotinDependencias, DbError> {
    let partidos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido WHERE botin_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(BotinDependencias { partidos })
}

/// Comprueba si ya existe ese nombre (sin distinguir mayúsculas).
pub fn existe_nombre(
    conn: &Connection,
    nombre: &str,
    exclude_id: Option<i64>,
) -> Result<bool, DbError> {
    let exclude = exclude_id.unwrap_or(0);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM botin
             WHERE lower(nombre) = lower(?1)
               AND (?2 = 0 OR id <> ?2)
        )",
        params![nombre.trim(), exclude],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Botines activos pendientes de sorteo.
pub fn draw_pending(conn: &Connection) -> Result<Vec<Botin>, DbError> {
    let sql = format!("SELECT {COLUMNS} FROM botin WHERE activa = 1 AND sorteada = 0 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], map_botin)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Marca un botín como sorteado.
pub fn mark_drawn(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("UPDATE botin SET sorteada = 1 WHERE id = ?1", [id])?;
    Ok(())
}

/// Reinicia el ciclo de sorteo.
pub fn reset_draw(conn: &Connection) -> Result<usize, DbError> {
    let updated = conn.execute("UPDATE botin SET sorteada = 0 WHERE activa = 1", [])?;
    Ok(updated)
}

/// Actualiza el botín predeterminado en `settings`.
pub fn set_default(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE settings SET botin_predeterminado = ?1 WHERE id = 1",
        [id],
    )?;
    Ok(())
}

fn map_botin(row: &Row<'_>) -> rusqlite::Result<Botin> {
    Ok(Botin {
        id: row.get(0)?,
        nombre: row.get(1)?,
        imagen_ruta: row.get(2)?,
        activa: row.get::<_, i64>(3)? != 0,
        sorteada: row.get::<_, i64>(4)? != 0,
    })
}
