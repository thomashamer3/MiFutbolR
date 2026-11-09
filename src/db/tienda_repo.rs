//! Persistencia de tiendas (IMP-20).

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::tiendas::Tienda;

/// Lista tiendas (favoritas primero).
pub fn list(conn: &Connection) -> Result<Vec<Tienda>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, tipo, url, telefono, instagram, productos, rango_precio,
                tiene_envio, favorito, notas
           FROM tienda ORDER BY favorito DESC, nombre",
    )?;
    let filas = stmt.query_map([], |row| {
        Ok(Tienda {
            id: row.get(0)?,
            nombre: row.get(1)?,
            tipo: row.get(2)?,
            url: row.get(3)?,
            telefono: row.get(4)?,
            instagram: row.get(5)?,
            productos: row.get(6)?,
            rango_precio: row.get(7)?,
            tiene_envio: row.get::<_, i64>(8)? != 0,
            favorito: row.get::<_, i64>(9)? != 0,
            notas: row.get(10)?,
        })
    })?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene una tienda.
pub fn get(conn: &Connection, id: i64) -> Result<Tienda, DbError> {
    conn.query_row(
        "SELECT id, nombre, tipo, url, telefono, instagram, productos, rango_precio,
                tiene_envio, favorito, notas
           FROM tienda WHERE id = ?1",
        params![id],
        |row| {
            Ok(Tienda {
                id: row.get(0)?,
                nombre: row.get(1)?,
                tipo: row.get(2)?,
                url: row.get(3)?,
                telefono: row.get(4)?,
                instagram: row.get(5)?,
                productos: row.get(6)?,
                rango_precio: row.get(7)?,
                tiene_envio: row.get::<_, i64>(8)? != 0,
                favorito: row.get::<_, i64>(9)? != 0,
                notas: row.get(10)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "tienda",
        id,
    })
}

/// Inserta una tienda.
pub fn insert(conn: &Connection, tienda: &Tienda) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO tienda (nombre, tipo, url, telefono, instagram, productos,
                             rango_precio, tiene_envio, favorito, notas)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            tienda.nombre,
            tienda.tipo,
            tienda.url,
            tienda.telefono,
            tienda.instagram,
            tienda.productos,
            tienda.rango_precio,
            i64::from(tienda.tiene_envio),
            i64::from(tienda.favorito),
            tienda.notas
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Actualiza una tienda.
pub fn update(conn: &Connection, tienda: &Tienda) -> Result<(), DbError> {
    conn.execute(
        "UPDATE tienda SET nombre = ?1, tipo = ?2, url = ?3, telefono = ?4,
                instagram = ?5, productos = ?6, rango_precio = ?7,
                tiene_envio = ?8, favorito = ?9, notas = ?10
          WHERE id = ?11",
        params![
            tienda.nombre,
            tienda.tipo,
            tienda.url,
            tienda.telefono,
            tienda.instagram,
            tienda.productos,
            tienda.rango_precio,
            i64::from(tienda.tiene_envio),
            i64::from(tienda.favorito),
            tienda.notas,
            tienda.id
        ],
    )?;
    Ok(())
}

/// Borra una tienda.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM tienda WHERE id = ?1", params![id])?;
    Ok(())
}
