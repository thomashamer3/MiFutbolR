//! Persistencia de química entre jugadores (IMP-24).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::quimica::{Dupla, MejorDupla};

/// Jugadores disponibles (id, nombre con equipo).
pub fn jugadores(conn: &Connection) -> Result<Vec<(i64, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT j.id, j.nombre || ' · ' || e.nombre
           FROM jugador j JOIN equipo e ON e.id = j.equipo_id
          ORDER BY e.nombre, j.nombre",
    )?;
    let filas = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Lista duplas.
pub fn list(conn: &Connection) -> Result<Vec<Dupla>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT q.id, q.jugador1_id, q.jugador2_id, j1.nombre, j2.nombre,
                q.partidos, q.victorias, q.goles, q.asistencias, q.comentario
           FROM quimica_jugador_estadistica q
           JOIN jugador j1 ON j1.id = q.jugador1_id
           JOIN jugador j2 ON j2.id = q.jugador2_id
          ORDER BY (CASE WHEN q.partidos = 0 THEN 0.0
                         ELSE q.victorias * 1.0 / q.partidos END) DESC,
                   q.partidos DESC, q.id",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(Dupla {
            id: row.get(0)?,
            jugador1_id: row.get(1)?,
            jugador2_id: row.get(2)?,
            jugador1: row.get(3)?,
            jugador2: row.get(4)?,
            partidos: row.get(5)?,
            victorias: row.get(6)?,
            goles: row.get(7)?,
            asistencias: row.get(8)?,
            comentario: row.get(9)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene una dupla.
pub fn get(conn: &Connection, id: i64) -> Result<Dupla, DbError> {
    conn.query_row(
        "SELECT q.id, q.jugador1_id, q.jugador2_id, j1.nombre, j2.nombre,
                q.partidos, q.victorias, q.goles, q.asistencias, q.comentario
           FROM quimica_jugador_estadistica q
           JOIN jugador j1 ON j1.id = q.jugador1_id
           JOIN jugador j2 ON j2.id = q.jugador2_id
          WHERE q.id = ?1",
        params![id],
        |row| {
            Ok(Dupla {
                id: row.get(0)?,
                jugador1_id: row.get(1)?,
                jugador2_id: row.get(2)?,
                jugador1: row.get(3)?,
                jugador2: row.get(4)?,
                partidos: row.get(5)?,
                victorias: row.get(6)?,
                goles: row.get(7)?,
                asistencias: row.get(8)?,
                comentario: row.get(9)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "quimica_jugador_estadistica",
        id,
    })
}

/// Inserta una dupla.
pub fn insert(conn: &Connection, dupla: &Dupla) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO quimica_jugador_estadistica
            (jugador1_id, jugador2_id, partidos, victorias, goles, asistencias, comentario)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            dupla.jugador1_id,
            dupla.jugador2_id,
            dupla.partidos,
            dupla.victorias,
            dupla.goles,
            dupla.asistencias,
            dupla.comentario
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una dupla.
pub fn update(conn: &Connection, dupla: &Dupla) -> Result<(), DbError> {
    conn.execute(
        "UPDATE quimica_jugador_estadistica
            SET jugador1_id = ?1, jugador2_id = ?2, partidos = ?3, victorias = ?4,
                goles = ?5, asistencias = ?6, comentario = ?7,
                updated_at = datetime('now','localtime')
          WHERE id = ?8",
        params![
            dupla.jugador1_id,
            dupla.jugador2_id,
            dupla.partidos,
            dupla.victorias,
            dupla.goles,
            dupla.asistencias,
            dupla.comentario,
            dupla.id
        ],
    )?;
    Ok(())
}

/// Borra una dupla.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM quimica_jugador_estadistica WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Mejor dupla según los equipos asignados a partidos (CTE `duplas_auto` de C):
/// mayor winrate sin empates, luego más victorias y más partidos.
pub fn mejor_combinacion(conn: &Connection) -> Result<Option<MejorDupla>, DbError> {
    Ok(conn
        .query_row(
            "WITH resumen AS (
                SELECT j1.nombre AS a, j2.nombre AS b,
                       SUM(p.resultado = 1) AS v, SUM(p.resultado = 3) AS d,
                       SUM(p.resultado = 2) AS e
                  FROM equipo eq
                  JOIN partido p ON p.id = eq.partido_id
                  JOIN jugador j1 ON j1.equipo_id = eq.id
                  JOIN jugador j2 ON j2.equipo_id = eq.id AND j1.id < j2.id
                 GROUP BY j1.id, j2.id)
             SELECT a, b, v, d, e FROM resumen
              ORDER BY CASE WHEN v + d > 0 THEN v * 100.0 / (v + d) ELSE 0 END DESC,
                       v DESC, v + d + e DESC
              LIMIT 1",
            [],
            |row| {
                Ok(MejorDupla {
                    jugador_a: row.get(0)?,
                    jugador_b: row.get(1)?,
                    victorias: row.get(2)?,
                    derrotas: row.get(3)?,
                    empates: row.get(4)?,
                })
            },
        )
        .optional()?)
}
