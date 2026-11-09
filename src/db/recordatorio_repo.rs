//! Persistencia de recordatorios (IMP-18).

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::recordatorios::{EventoAgenda, Periodicidad, Recordatorio};

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<Recordatorio> {
    let fecha: String = row.get(3)?;
    Ok(Recordatorio {
        id: row.get(0)?,
        titulo: row.get(1)?,
        nota: row.get(2)?,
        fecha: dates::parse_date(&fecha).unwrap_or_else(dates::today),
        hora: row.get(4)?,
        periodicidad: Periodicidad::from_i64(row.get(5)?),
        tematica: row.get(6)?,
        completado: row.get::<_, i64>(7)? != 0,
        ultima_gen: row.get(8)?,
    })
}

/// Lista recordatorios.
pub fn list(conn: &Connection) -> Result<Vec<Recordatorio>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, titulo, nota, fecha, hora, periodicidad, tematica, completado, ultima_gen
           FROM recordatorio
          ORDER BY completado, fecha, hora, id",
    )?;
    let filas = stmt.query_map([], map)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Recordatorios pendientes recurrentes.
pub fn recurrentes(conn: &Connection) -> Result<Vec<Recordatorio>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, titulo, nota, fecha, hora, periodicidad, tematica, completado, ultima_gen
           FROM recordatorio
          WHERE completado = 0 AND periodicidad > 0
          ORDER BY fecha",
    )?;
    let filas = stmt.query_map([], map)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene un recordatorio.
pub fn get(conn: &Connection, id: i64) -> Result<Recordatorio, DbError> {
    conn.query_row(
        "SELECT id, titulo, nota, fecha, hora, periodicidad, tematica, completado, ultima_gen
           FROM recordatorio WHERE id = ?1",
        params![id],
        map,
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "recordatorio",
        id,
    })
}

/// Inserta un recordatorio.
pub fn insert(conn: &Connection, item: &Recordatorio) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO recordatorio (titulo, nota, fecha, hora, periodicidad, tematica, completado)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            item.titulo,
            item.nota,
            item.fecha.format("%Y-%m-%d").to_string(),
            item.hora,
            item.periodicidad.as_i64(),
            item.tematica,
            i64::from(item.completado)
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza un recordatorio.
pub fn update(conn: &Connection, item: &Recordatorio) -> Result<(), DbError> {
    conn.execute(
        "UPDATE recordatorio SET titulo = ?1, nota = ?2, fecha = ?3, hora = ?4,
                periodicidad = ?5, tematica = ?6, completado = ?7
          WHERE id = ?8",
        params![
            item.titulo,
            item.nota,
            item.fecha.format("%Y-%m-%d").to_string(),
            item.hora,
            item.periodicidad.as_i64(),
            item.tematica,
            i64::from(item.completado),
            item.id
        ],
    )?;
    Ok(())
}

/// Borra un recordatorio.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM recordatorio WHERE id = ?1", params![id])?;
    Ok(())
}

/// Avanza la fecha de una ocurrencia recurrente.
pub fn avanzar_fecha(
    conn: &Connection,
    id: i64,
    fecha: NaiveDate,
    hoy: NaiveDate,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE recordatorio SET fecha = ?1, ultima_gen = ?2 WHERE id = ?3",
        params![
            fecha.format("%Y-%m-%d").to_string(),
            hoy.format("%Y-%m-%d").to_string(),
            id
        ],
    )?;
    Ok(())
}

/// Agenda fusionada de recordatorios pendientes y partidos.
pub fn agenda(conn: &Connection) -> Result<Vec<EventoAgenda>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT fecha, hora, titulo, tipo FROM (
             SELECT fecha AS fecha, hora AS hora, titulo AS titulo, 'Recordatorio' AS tipo
               FROM recordatorio WHERE completado = 0
             UNION ALL
             SELECT substr(fecha_hora, 1, 10) AS fecha,
                    substr(fecha_hora, 12, 5) AS hora,
                    'Partido vs ' || CASE WHEN rival_nombre = '' THEN 'rival' ELSE rival_nombre END AS titulo,
                    'Partido' AS tipo
               FROM partido
         )
         ORDER BY fecha, hora",
    )?;
    let filas = stmt.query_map([], |row| {
        let fecha: String = row.get(0)?;
        Ok((
            fecha,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut eventos = Vec::new();
    for fila in filas {
        let (fecha, hora, titulo, tipo) = fila?;
        let Some(fecha) = dates::parse_date(&fecha) else {
            continue;
        };
        eventos.push(EventoAgenda {
            fecha,
            hora,
            titulo,
            tipo: if tipo == "Partido" {
                "Partido"
            } else {
                "Recordatorio"
            },
        });
    }
    Ok(eventos)
}
