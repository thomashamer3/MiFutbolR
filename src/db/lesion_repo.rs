//! Repositorio de lesiones (§13.3).

use rusqlite::{Connection, Row, params};

use super::error::DbError;
use crate::domain::dates;
use crate::domain::models::{EstadoLesion, Lesion, LesionFilter, LesionRow, LesionStats};

/// Lista lesiones con camiseta y partido resueltos.
pub fn list(conn: &Connection, filter: &LesionFilter) -> Result<Vec<LesionRow>, DbError> {
    let sql = format!(
        "SELECT l.id, l.jugador, l.tipo, l.fecha, l.estado,
                COALESCE(c.nombre, ''),
                CASE WHEN p.id IS NULL THEN ''
                     ELSE p.id || ' · ' || p.fecha_hora END
           FROM lesion l
           LEFT JOIN camiseta c ON c.id = l.camiseta_id
           LEFT JOIN partido p ON p.id = l.partido_id
          WHERE (?1 = ''
                 OR l.jugador LIKE '%' || ?1 || '%' COLLATE NOCASE
                 OR l.tipo LIKE '%' || ?1 || '%' COLLATE NOCASE
                 OR l.descripcion LIKE '%' || ?1 || '%' COLLATE NOCASE)
            AND (?2 = '' OR l.estado = ?2)
          ORDER BY {}",
        filter.orden.sql_order()
    );
    let estado = filter
        .estado
        .map(|estado| estado.as_str().to_string())
        .unwrap_or_default();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![filter.texto.trim(), estado], |row| {
            let fecha_texto: String = row.get(3)?;
            let fecha = dates::parse_datetime(&fecha_texto).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    3,
                    "fecha".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;
            Ok(LesionRow {
                id: row.get(0)?,
                jugador: row.get(1)?,
                tipo: row.get(2)?,
                fecha,
                estado: EstadoLesion::from_str(&row.get::<_, String>(4)?)
                    .unwrap_or(EstadoLesion::Activa),
                camiseta: row.get(5)?,
                partido: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Obtiene una lesión.
pub fn get(conn: &Connection, id: i64) -> Result<Lesion, DbError> {
    conn.query_row(
        "SELECT id, jugador, tipo, descripcion, fecha, camiseta_id, estado, partido_id
           FROM lesion WHERE id = ?1",
        [id],
        map_lesion,
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => DbError::NotFound {
            entidad: "Lesion",
            id,
        },
        other => other.into(),
    })
}

/// Inserta una lesión.
pub fn insert(conn: &Connection, lesion: &Lesion) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO lesion
            (jugador, tipo, descripcion, fecha, camiseta_id, estado, partido_id, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now','localtime'))",
        params![
            lesion.jugador,
            lesion.tipo,
            lesion.descripcion,
            dates::to_iso_datetime(lesion.fecha),
            lesion.camiseta_id,
            lesion.estado.as_str(),
            lesion.partido_id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una lesión.
pub fn update(conn: &Connection, lesion: &Lesion) -> Result<(), DbError> {
    let updated = conn.execute(
        "UPDATE lesion
            SET jugador = ?1, tipo = ?2, descripcion = ?3, fecha = ?4,
                camiseta_id = ?5, estado = ?6, partido_id = ?7,
                updated_at = datetime('now','localtime')
          WHERE id = ?8",
        params![
            lesion.jugador,
            lesion.tipo,
            lesion.descripcion,
            dates::to_iso_datetime(lesion.fecha),
            lesion.camiseta_id,
            lesion.estado.as_str(),
            lesion.partido_id,
            lesion.id,
        ],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound {
            entidad: "Lesion",
            id: lesion.id,
        });
    }
    Ok(())
}

/// Elimina una lesión.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    let deleted = conn.execute("DELETE FROM lesion WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(DbError::NotFound {
            entidad: "Lesion",
            id,
        });
    }
    Ok(())
}

/// Estadísticas agregadas.
pub fn stats(conn: &Connection) -> Result<LesionStats, DbError> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM lesion", [], |row| row.get(0))?;
    let activas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM lesion WHERE estado = 'ACTIVA'",
        [],
        |row| row.get(0),
    )?;
    let recuperadas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM lesion WHERE estado = 'RECUPERADO'",
        [],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare("SELECT fecha FROM lesion ORDER BY fecha ASC")?;
    let fechas = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut dias = Vec::new();
    let mut anterior: Option<chrono::NaiveDateTime> = None;
    for texto in fechas {
        let Some(fecha) = dates::parse_datetime(&texto) else {
            continue;
        };
        if let Some(prev) = anterior {
            let diferencia = fecha.signed_duration_since(prev).num_days();
            if diferencia >= 0 {
                dias.push(diferencia as f64);
            }
        }
        anterior = Some(fecha);
    }
    let promedio_dias = if dias.is_empty() {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        Some(dias.iter().sum::<f64>() / dias.len() as f64)
    };

    let mut stmt = conn.prepare(
        "SELECT tipo, COUNT(*) FROM lesion GROUP BY tipo ORDER BY COUNT(*) DESC, tipo ASC",
    )?;
    let por_tipo = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', fecha), COUNT(*) FROM lesion
          GROUP BY strftime('%Y-%m', fecha) ORDER BY 1 ASC",
    )?;
    let por_mes = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LesionStats {
        total,
        activas,
        recuperadas,
        promedio_dias,
        por_tipo,
        por_mes,
    })
}

fn map_lesion(row: &Row<'_>) -> rusqlite::Result<Lesion> {
    let fecha_texto: String = row.get(4)?;
    let fecha = dates::parse_datetime(&fecha_texto).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(4, "fecha".to_string(), rusqlite::types::Type::Text)
    })?;
    Ok(Lesion {
        id: row.get(0)?,
        jugador: row.get(1)?,
        tipo: row.get(2)?,
        descripcion: row.get(3)?,
        fecha,
        camiseta_id: row.get(5)?,
        estado: EstadoLesion::from_str(&row.get::<_, String>(6)?).unwrap_or(EstadoLesion::Activa),
        partido_id: row.get(7)?,
    })
}
