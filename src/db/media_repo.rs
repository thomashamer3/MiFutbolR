//! Persistencia de referencias multimedia (IMP-21).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::media::{MediaItem, TipoMedia};

/// Lista referencias, opcionalmente por tipo.
pub fn list(conn: &Connection, tipo: Option<&str>) -> Result<Vec<MediaItem>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, partido_id, tipo, titulo, url, descripcion, fecha
           FROM media
          WHERE (?1 = '' OR tipo = ?1)
          ORDER BY fecha DESC, id DESC",
    )?;
    let filas = stmt.query_map(params![tipo.unwrap_or("")], |row| {
        Ok(MediaItem {
            id: row.get(0)?,
            partido_id: row.get(1)?,
            tipo: TipoMedia::from_codigo(&row.get::<_, String>(2)?),
            titulo: row.get(3)?,
            url: row.get(4)?,
            descripcion: row.get(5)?,
            fecha: row.get(6)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene una referencia.
pub fn get(conn: &Connection, id: i64) -> Result<MediaItem, DbError> {
    conn.query_row(
        "SELECT id, partido_id, tipo, titulo, url, descripcion, fecha
           FROM media WHERE id = ?1",
        params![id],
        |row| {
            Ok(MediaItem {
                id: row.get(0)?,
                partido_id: row.get(1)?,
                tipo: TipoMedia::from_codigo(&row.get::<_, String>(2)?),
                titulo: row.get(3)?,
                url: row.get(4)?,
                descripcion: row.get(5)?,
                fecha: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "media",
        id,
    })
}

/// Inserta una referencia.
pub fn insert(conn: &Connection, item: &MediaItem) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO media (partido_id, tipo, titulo, url, descripcion)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            item.partido_id,
            item.tipo.as_str(),
            item.titulo,
            item.url,
            item.descripcion
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una referencia.
pub fn update(conn: &Connection, item: &MediaItem) -> Result<(), DbError> {
    conn.execute(
        "UPDATE media SET partido_id = ?1, tipo = ?2, titulo = ?3, url = ?4,
                descripcion = ?5 WHERE id = ?6",
        params![
            item.partido_id,
            item.tipo.as_str(),
            item.titulo,
            item.url,
            item.descripcion,
            item.id
        ],
    )?;
    Ok(())
}

/// Borra una referencia.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM media WHERE id = ?1", params![id])?;
    Ok(())
}
