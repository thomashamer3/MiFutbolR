//! Agregados para el comparador (IMP-14).

use rusqlite::{Connection, params};

use super::error::DbError;
use crate::domain::comparador::{AgregadoComparacion, OpcionComparador};

/// Camisetas disponibles.
pub fn camisetas(conn: &Connection) -> Result<Vec<OpcionComparador>, DbError> {
    let mut stmt = conn.prepare("SELECT id, nombre FROM camiseta ORDER BY nombre")?;
    let filas = stmt.query_map([], |row| {
        Ok(OpcionComparador {
            id: row.get(0)?,
            nombre: row.get(1)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Canchas disponibles.
pub fn canchas(conn: &Connection) -> Result<Vec<OpcionComparador>, DbError> {
    let mut stmt = conn.prepare("SELECT id, nombre FROM cancha ORDER BY nombre")?;
    let filas = stmt.query_map([], |row| {
        Ok(OpcionComparador {
            id: row.get(0)?,
            nombre: row.get(1)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Agregado de una camiseta.
pub fn agregado_camiseta(conn: &Connection, id: i64) -> Result<AgregadoComparacion, DbError> {
    conn.query_row(
        "SELECT c.nombre, COUNT(p.id), COALESCE(SUM(p.goles), 0),
                COALESCE(SUM(p.asistencias), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM camiseta c LEFT JOIN partido p ON p.camiseta_id = c.id
          WHERE c.id = ?1
          GROUP BY c.id",
        params![id],
        |row| {
            Ok(AgregadoComparacion {
                nombre: row.get(0)?,
                partidos: row.get(1)?,
                goles: row.get(2)?,
                asistencias: row.get(3)?,
                victorias: row.get(4)?,
                empates: row.get(5)?,
                derrotas: row.get(6)?,
                rendimiento: row.get(7)?,
            })
        },
    )
    .map_err(DbError::from)
}

/// Agregado de una cancha.
pub fn agregado_cancha(conn: &Connection, id: i64) -> Result<AgregadoComparacion, DbError> {
    conn.query_row(
        "SELECT c.nombre, COUNT(p.id), COALESCE(SUM(p.goles), 0),
                COALESCE(SUM(p.asistencias), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(AVG(p.rendimiento_general), 0.0)
           FROM cancha c LEFT JOIN partido p ON p.cancha_id = c.id
          WHERE c.id = ?1
          GROUP BY c.id",
        params![id],
        |row| {
            Ok(AgregadoComparacion {
                nombre: row.get(0)?,
                partidos: row.get(1)?,
                goles: row.get(2)?,
                asistencias: row.get(3)?,
                victorias: row.get(4)?,
                empates: row.get(5)?,
                derrotas: row.get(6)?,
                rendimiento: row.get(7)?,
            })
        },
    )
    .map_err(DbError::from)
}
