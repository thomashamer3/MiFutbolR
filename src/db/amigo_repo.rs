//! Persistencia de amigos y ranking (IMP-22).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::amigos::{Amigo, RankingAmigo};

/// Lista amigos.
pub fn list(conn: &Connection) -> Result<Vec<Amigo>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, posicion, goles, asistencias, partidos, mejor_rendimiento, notas
           FROM amigo ORDER BY goles DESC, asistencias DESC, nombre",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(Amigo {
            id: row.get(0)?,
            nombre: row.get(1)?,
            posicion: row.get(2)?,
            goles: row.get(3)?,
            asistencias: row.get(4)?,
            partidos: row.get(5)?,
            mejor_rendimiento: row.get(6)?,
            notas: row.get(7)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene un amigo.
pub fn get(conn: &Connection, id: i64) -> Result<Amigo, DbError> {
    conn.query_row(
        "SELECT id, nombre, posicion, goles, asistencias, partidos, mejor_rendimiento, notas
           FROM amigo WHERE id = ?1",
        params![id],
        |row| {
            Ok(Amigo {
                id: row.get(0)?,
                nombre: row.get(1)?,
                posicion: row.get(2)?,
                goles: row.get(3)?,
                asistencias: row.get(4)?,
                partidos: row.get(5)?,
                mejor_rendimiento: row.get(6)?,
                notas: row.get(7)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "amigo",
        id,
    })
}

/// Inserta un amigo.
pub fn insert(conn: &Connection, amigo: &Amigo) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO amigo (nombre, posicion, goles, asistencias, partidos, mejor_rendimiento, notas)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            amigo.nombre,
            amigo.posicion,
            amigo.goles,
            amigo.asistencias,
            amigo.partidos,
            amigo.mejor_rendimiento,
            amigo.notas
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un amigo.
pub fn update(conn: &Connection, amigo: &Amigo) -> Result<(), DbError> {
    conn.execute(
        "UPDATE amigo SET nombre = ?1, posicion = ?2, goles = ?3, asistencias = ?4,
                partidos = ?5, mejor_rendimiento = ?6, notas = ?7
          WHERE id = ?8",
        params![
            amigo.nombre,
            amigo.posicion,
            amigo.goles,
            amigo.asistencias,
            amigo.partidos,
            amigo.mejor_rendimiento,
            amigo.notas,
            amigo.id
        ],
    )?;
    Ok(())
}

/// Borra un amigo.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM amigo WHERE id = ?1", params![id])?;
    Ok(())
}

/// Estadísticas locales del usuario.
pub fn locales(conn: &Connection) -> Result<RankingAmigo, DbError> {
    let (partidos, goles, asistencias, mejor) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(goles), 0), COALESCE(SUM(asistencias), 0),
                COALESCE(MAX(rendimiento_general), 0)
           FROM partido",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    Ok(RankingAmigo {
        nombre: "Yo".to_string(),
        goles,
        asistencias,
        partidos,
        mejor_rendimiento: mejor,
        local: true,
    })
}
