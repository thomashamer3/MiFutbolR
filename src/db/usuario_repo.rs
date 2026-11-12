//! Persistencia de cuentas locales.

use rusqlite::{Connection, OptionalExtension, params};

use super::error::DbError;
use crate::domain::usuario::{Sesion, Usuario};

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<Usuario> {
    Ok(Usuario {
        id: row.get(0)?,
        nombre: row.get(1)?,
        con_password: row.get::<_, i64>(2)? != 0,
        creado_en: row.get(3)?,
        ultimo_acceso: row.get(4)?,
    })
}

/// Lista cuentas.
pub fn list(conn: &Connection) -> Result<Vec<Usuario>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, nombre, con_password, creado_en, ultimo_acceso
           FROM cuenta ORDER BY nombre",
    )?;
    let filas = stmt.query_map([], map)?;
    Ok(filas.collect::<Result<Vec<_>, _>>()?)
}

/// Obtiene una cuenta.
pub fn get(conn: &Connection, id: i64) -> Result<Usuario, DbError> {
    conn.query_row(
        "SELECT id, nombre, con_password, creado_en, ultimo_acceso
           FROM cuenta WHERE id = ?1",
        params![id],
        map,
    )
    .optional()?
    .ok_or(DbError::NotFound {
        entidad: "cuenta",
        id,
    })
}

/// Busca por nombre (sin distinguir mayúsculas).
pub fn get_by_nombre(conn: &Connection, nombre: &str) -> Result<Option<Usuario>, DbError> {
    Ok(conn
        .query_row(
            "SELECT id, nombre, con_password, creado_en, ultimo_acceso
               FROM cuenta WHERE nombre = ?1 COLLATE NOCASE",
            params![nombre],
            map,
        )
        .optional()?)
}

/// Hash almacenado de una cuenta.
pub fn password_hash(conn: &Connection, nombre: &str) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row(
            "SELECT password_hash FROM cuenta WHERE nombre = ?1 COLLATE NOCASE",
            params![nombre],
            |row| row.get(0),
        )
        .optional()?)
}

/// Crea una cuenta.
pub fn insert(
    conn: &Connection,
    nombre: &str,
    hash: &str,
    con_password: bool,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO cuenta (nombre, password_hash, con_password) VALUES (?1, ?2, ?3)",
        params![nombre, hash, i64::from(con_password)],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Cambia la contraseña.
pub fn update_password(
    conn: &Connection,
    id: i64,
    hash: &str,
    con_password: bool,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE cuenta SET password_hash = ?1, con_password = ?2 WHERE id = ?3",
        params![hash, i64::from(con_password), id],
    )?;
    Ok(())
}

/// Elimina una cuenta.
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM cuenta WHERE id = ?1", params![id])?;
    Ok(())
}

/// Marca el último acceso.
pub fn touch(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE cuenta SET ultimo_acceso = datetime('now','localtime') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Estado de la sesión.
pub fn sesion(conn: &Connection) -> Result<Sesion, DbError> {
    let (usuario_id, vista): (Option<i64>, i64) = conn.query_row(
        "SELECT usuario_id, bienvenida_vista FROM cuenta_sesion WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let usuario = match usuario_id {
        Some(id) => get(conn, id).ok(),
        None => None,
    };
    Ok(Sesion {
        usuario,
        bienvenida_vista: vista != 0,
    })
}

/// Inicia sesión con una cuenta.
pub fn set_sesion(conn: &Connection, usuario_id: Option<i64>) -> Result<(), DbError> {
    conn.execute(
        "UPDATE cuenta_sesion SET usuario_id = ?1 WHERE id = 1",
        params![usuario_id],
    )?;
    Ok(())
}

/// Marca la bienvenida como vista.
pub fn marcar_bienvenida(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "UPDATE cuenta_sesion SET bienvenida_vista = 1 WHERE id = 1",
        [],
    )?;
    Ok(())
}
