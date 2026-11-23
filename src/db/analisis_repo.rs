//! Serie de partidos para el módulo Análisis (`analisis.c`).

use rusqlite::Connection;

use super::error::DbError;
use crate::domain::analisis::PartidoAnalisis;

/// Todos los partidos en orden cronológico.
pub fn serie(conn: &Connection) -> Result<Vec<PartidoAnalisis>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, fecha_hora, goles, asistencias, resultado, rendimiento_general,
                cansancio, estado_animo
           FROM partido ORDER BY fecha_hora, id",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(PartidoAnalisis {
            id: row.get(0)?,
            fecha_hora: row.get(1)?,
            goles: row.get(2)?,
            asistencias: row.get(3)?,
            resultado: row.get(4)?,
            rendimiento: row.get(5)?,
            cansancio: row.get(6)?,
            animo: row.get(7)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}
