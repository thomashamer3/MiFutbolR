//! Persistencia de temporadas (IMP-12).

use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::temporadas::{Temporada, TemporadaFase, TemporadaResumen};

/// Lista las temporadas.
pub fn list(conn: &Connection) -> Result<Vec<Temporada>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, anio, fecha_inicio, fecha_fin, estado, descripcion
           FROM temporada ORDER BY anio DESC, id DESC",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    filas
        .map(|fila| {
            let (id, nombre, anio, inicio, fin, estado, descripcion) = fila?;
            Ok(Temporada {
                id,
                nombre,
                anio,
                fecha_inicio: dates::parse_date(&inicio)
                    .ok_or_else(|| DbError::InvalidData(format!("fecha inválida: {inicio}")))?,
                fecha_fin: dates::parse_date(&fin)
                    .ok_or_else(|| DbError::InvalidData(format!("fecha inválida: {fin}")))?,
                estado,
                descripcion,
            })
        })
        .collect()
}

/// Obtiene una temporada.
pub fn get(conn: &Connection, id: i64) -> Result<Temporada, DbError> {
    conn.query_row(
        "SELECT id, nombre, anio, fecha_inicio, fecha_fin, estado, descripcion
           FROM temporada WHERE id = ?1",
        params![id],
        |row| {
            let inicio: String = row.get(3)?;
            let fin: String = row.get(4)?;
            Ok(Temporada {
                id: row.get(0)?,
                nombre: row.get(1)?,
                anio: row.get(2)?,
                fecha_inicio: dates::parse_date(&inicio).unwrap_or_else(dates::today),
                fecha_fin: dates::parse_date(&fin).unwrap_or_else(dates::today),
                estado: row.get(5)?,
                descripcion: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "temporada",
        id,
    })
}

/// Crea una temporada.
pub fn insert(
    conn: &Connection,
    nombre: &str,
    anio: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
    descripcion: &str,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO temporada (nombre, anio, fecha_inicio, fecha_fin, estado, descripcion)
         VALUES (?1, ?2, ?3, ?4, 'Activa', ?5)",
        params![
            nombre,
            anio,
            inicio.format("%Y-%m-%d").to_string(),
            fin.format("%Y-%m-%d").to_string(),
            descripcion
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una temporada.
pub fn update(
    conn: &Connection,
    id: i64,
    nombre: &str,
    anio: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
    descripcion: &str,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE temporada
            SET nombre = ?1, anio = ?2, fecha_inicio = ?3, fecha_fin = ?4, descripcion = ?5
          WHERE id = ?6",
        params![
            nombre,
            anio,
            inicio.format("%Y-%m-%d").to_string(),
            fin.format("%Y-%m-%d").to_string(),
            descripcion,
            id
        ],
    )?;
    Ok(())
}

/// Borra una temporada.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM temporada WHERE id = ?1", params![id])?;
    Ok(())
}

/// Fases de la temporada.
pub fn fases(conn: &Connection, temporada_id: i64) -> Result<Vec<TemporadaFase>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo, fecha_inicio, fecha_fin
           FROM temporada_fase WHERE temporada_id = ?1 ORDER BY id",
    )?;
    let filas = stmt.query_map(params![temporada_id], |row| {
        Ok(TemporadaFase {
            id: row.get(0)?,
            nombre: row.get(1)?,
            tipo: row.get(2)?,
            fecha_inicio: row.get(3)?,
            fecha_fin: row.get(4)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Inserta las fases por defecto.
pub fn insertar_fases_default(
    conn: &Connection,
    temporada_id: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> Result<(), DbError> {
    for (nombre, tipo, desde, hasta) in [
        ("Pretemporada", "pretemporada", inicio, inicio),
        ("Temporada regular", "regular", inicio, fin),
        ("Postemporada", "postemporada", fin, fin),
    ] {
        conn.execute(
            "INSERT INTO temporada_fase (temporada_id, nombre, tipo, fecha_inicio, fecha_fin)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                temporada_id,
                nombre,
                tipo,
                desde.format("%Y-%m-%d").to_string(),
                hasta.format("%Y-%m-%d").to_string()
            ],
        )?;
    }
    Ok(())
}

/// Asocia todos los torneos existentes.
pub fn asociar_todos_torneos(conn: &Connection, temporada_id: i64) -> Result<usize, DbError> {
    Ok(conn.execute(
        "INSERT OR IGNORE INTO torneo_temporada (temporada_id, torneo_id)
         SELECT ?1, id FROM torneo",
        params![temporada_id],
    )?)
}

/// Nombres de torneos asociados.
pub fn torneos(conn: &Connection, temporada_id: i64) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT t.nombre
           FROM torneo_temporada tt JOIN torneo t ON t.id = tt.torneo_id
          WHERE tt.temporada_id = ?1 ORDER BY t.id",
    )?;
    let filas = stmt.query_map(params![temporada_id], |row| row.get::<_, String>(0))?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Resumen de partidos y lesiones en el rango.
pub fn resumen(
    conn: &Connection,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> Result<TemporadaResumen, DbError> {
    let (partidos, goles, asistencias, victorias, empates, derrotas, rendimiento) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(goles), 0), COALESCE(SUM(asistencias), 0),
                COALESCE(SUM(CASE WHEN resultado = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 2 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN resultado = 3 THEN 1 ELSE 0 END), 0),
                COALESCE(AVG(rendimiento_general), 0.0)
           FROM partido
          WHERE fecha_hora >= ?1 || ' 00:00:00'
            AND fecha_hora <= ?2 || ' 23:59:59'",
            params![
                inicio.format("%Y-%m-%d").to_string(),
                fin.format("%Y-%m-%d").to_string()
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )?;
    let lesiones: i64 = conn.query_row(
        "SELECT COUNT(*) FROM lesion WHERE fecha >= ?1 AND fecha <= ?2",
        params![
            inicio.format("%Y-%m-%d").to_string(),
            fin.format("%Y-%m-%d").to_string()
        ],
        |row| row.get(0),
    )?;
    let winrate = if partidos > 0 {
        victorias as f64 * 100.0 / partidos as f64
    } else {
        0.0
    };
    Ok(TemporadaResumen {
        partidos,
        goles,
        asistencias,
        victorias,
        empates,
        derrotas,
        rendimiento,
        winrate,
        lesiones,
    })
}

/// Fatiga: partidos en los últimos 14 días del cierre (o de hoy si sigue abierta).
pub fn fatiga(conn: &Connection, fin: NaiveDate) -> Result<f64, DbError> {
    let hoy = dates::today();
    let referencia = if fin <= hoy { fin } else { hoy };
    let desde = referencia - chrono::Duration::days(14);
    let partidos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partido
          WHERE fecha_hora >= ?1 || ' 00:00:00'
            AND fecha_hora <= ?2 || ' 23:59:59'",
        params![
            desde.format("%Y-%m-%d").to_string(),
            referencia.format("%Y-%m-%d").to_string()
        ],
        |row| row.get(0),
    )?;
    Ok((partidos as f64 * 10.0).min(100.0))
}

/// Año de una fecha.
#[must_use]
pub fn anio(fecha: NaiveDate) -> i64 {
    i64::from(fecha.year())
}
