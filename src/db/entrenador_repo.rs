//! Lecturas y configuración del Entrenador IA.

use rusqlite::{Connection, params};

use super::error::DbError;
use crate::domain::entrenador::{Agregados, PartidoIa};

/// Últimos partidos, más recientes primero.
pub fn ultimos_partidos(conn: &Connection, limite: i64) -> Result<Vec<PartidoIa>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT fecha_hora, rendimiento_general, cansancio, estado_animo, resultado
           FROM partido ORDER BY fecha_hora DESC, id DESC LIMIT ?1",
    )?;
    let filas = stmt.query_map(params![limite], |row| {
        Ok(PartidoIa {
            fecha_hora: row.get(0)?,
            rendimiento: row.get(1)?,
            cansancio: row.get(2)?,
            animo: row.get(3)?,
            resultado: row.get(4)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Agregados de todos los partidos y lesiones.
pub fn agregados(conn: &Connection) -> Result<Agregados, DbError> {
    Ok(conn.query_row(
        "SELECT COUNT(*), COALESCE(AVG(goles), 0.0), COALESCE(AVG(asistencias), 0.0),
                COALESCE(MAX(rendimiento_general), 0), COALESCE(MIN(rendimiento_general), 0),
                (SELECT COUNT(*) FROM lesion)
           FROM partido",
        [],
        |row| {
            Ok(Agregados {
                partidos: row.get(0)?,
                goles: row.get(1)?,
                asistencias: row.get(2)?,
                rendimiento_max: row.get(3)?,
                rendimiento_min: row.get(4)?,
                lesiones: row.get(5)?,
            })
        },
    )?)
}

/// Nivel de intervención (0-3).
pub fn nivel(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE((SELECT nivel_intervencion FROM ia_config WHERE id = 1), 2)",
        [],
        |row| row.get(0),
    )?)
}

/// Cambia el nivel de intervención.
pub fn set_nivel(conn: &Connection, nivel: i64) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO ia_config (id, nivel_intervencion) VALUES (1, ?1)
         ON CONFLICT(id) DO UPDATE SET nivel_intervencion = excluded.nivel_intervencion",
        params![nivel],
    )?;
    Ok(())
}

/// (ultima_alerta unix, ultimo_riesgo) del último aviso previo a un partido.
pub fn ultima_alerta(conn: &Connection) -> Result<(i64, f64), DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE((SELECT ultima_alerta FROM ia_config WHERE id = 1), 0),
                COALESCE((SELECT ultimo_riesgo FROM ia_config WHERE id = 1), 0.0)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

/// Guarda el momento y el riesgo del aviso mostrado.
pub fn registrar_alerta(conn: &Connection, cuando: i64, riesgo: f64) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO ia_config (id, ultima_alerta, ultimo_riesgo) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET ultima_alerta = excluded.ultima_alerta,
                                       ultimo_riesgo = excluded.ultimo_riesgo",
        params![cuando, riesgo],
    )?;
    Ok(())
}

/// Registra si se siguió un consejo.
pub fn registrar_consejo(conn: &Connection, consejo: &str, seguido: bool) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO consejos_historial (fecha, consejo, seguido)
         VALUES (CAST(strftime('%s', 'now') AS INTEGER), ?1, ?2)",
        params![consejo, i64::from(seguido)],
    )?;
    Ok(())
}

/// (seguidos, ignorados) del historial.
pub fn historial(conn: &Connection) -> Result<(i64, i64), DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(seguido = 1), 0), COALESCE(SUM(seguido = 0), 0)
           FROM consejos_historial",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}
