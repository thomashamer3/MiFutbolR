//! Cuentas locales, autenticación y bienvenida.

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::db::DbError;
use crate::db::usuario_repo;
use crate::domain::usuario::{Sesion, Usuario};

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Validación o credenciales.
    #[error("{0}")]
    Message(String),
}

/// Lista cuentas.
pub fn listar(conn: &Connection) -> Result<Vec<Usuario>, ServiceError> {
    Ok(usuario_repo::list(conn)?)
}

/// Crea una cuenta y la deja como sesión actual.
pub fn crear(
    conn: &Connection,
    nombre: &str,
    password: Option<&str>,
) -> Result<Usuario, ServiceError> {
    let nombre = validar_nombre(nombre)?;
    if usuario_repo::get_by_nombre(conn, &nombre)?.is_some() {
        return Err(ServiceError::Message(
            "Ya existe una cuenta con ese nombre".to_string(),
        ));
    }
    let password = password.map(str::trim).filter(|valor| !valor.is_empty());
    let (hash, con_password) = match password {
        Some(password) => (hash_password(password), true),
        None => (String::new(), false),
    };
    let id = usuario_repo::insert(conn, &nombre, &hash, con_password)?;
    usuario_repo::set_sesion(conn, Some(id))?;
    usuario_repo::touch(conn, id)?;
    Ok(usuario_repo::get(conn, id)?)
}

/// Autentica una cuenta y la deja como sesión actual.
pub fn autenticar(
    conn: &Connection,
    nombre: &str,
    password: Option<&str>,
) -> Result<Usuario, ServiceError> {
    let usuario = usuario_repo::get_by_nombre(conn, nombre.trim())?
        .ok_or_else(|| ServiceError::Message("No existe esa cuenta".to_string()))?;
    if usuario.con_password {
        let hash = usuario_repo::password_hash(conn, &usuario.nombre)?.unwrap_or_default();
        if !verify_password(password.unwrap_or(""), &hash) {
            return Err(ServiceError::Message("Contraseña incorrecta".to_string()));
        }
    }
    usuario_repo::set_sesion(conn, Some(usuario.id))?;
    usuario_repo::touch(conn, usuario.id)?;
    Ok(usuario_repo::get(conn, usuario.id)?)
}

/// Cambia o quita la contraseña de una cuenta.
pub fn cambiar_password(
    conn: &Connection,
    id: i64,
    password: Option<&str>,
) -> Result<(), ServiceError> {
    let password = password.map(str::trim).filter(|valor| !valor.is_empty());
    let (hash, con_password) = match password {
        Some(password) => (hash_password(password), true),
        None => (String::new(), false),
    };
    usuario_repo::update_password(conn, id, &hash, con_password)?;
    Ok(())
}

/// Elimina una cuenta.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    let sesion = usuario_repo::sesion(conn)?;
    usuario_repo::delete(conn, id)?;
    if sesion
        .usuario
        .as_ref()
        .is_some_and(|usuario| usuario.id == id)
    {
        usuario_repo::set_sesion(conn, None)?;
    }
    Ok(())
}

/// Sesión actual.
pub fn sesion(conn: &Connection) -> Result<Sesion, ServiceError> {
    Ok(usuario_repo::sesion(conn)?)
}

/// Cierra la sesión.
pub fn cerrar_sesion(conn: &Connection) -> Result<(), ServiceError> {
    usuario_repo::set_sesion(conn, None)?;
    Ok(())
}

/// Marca la pantalla de bienvenida como vista.
pub fn marcar_bienvenida(conn: &Connection) -> Result<(), ServiceError> {
    usuario_repo::marcar_bienvenida(conn)?;
    Ok(())
}

fn validar_nombre(nombre: &str) -> Result<String, ServiceError> {
    let nombre = nombre.trim();
    if nombre.chars().count() < 2 {
        return Err(ServiceError::Message(
            "El nombre debe tener al menos 2 caracteres".to_string(),
        ));
    }
    if nombre.chars().count() > 40 {
        return Err(ServiceError::Message(
            "El nombre no puede superar los 40 caracteres".to_string(),
        ));
    }
    if nombre.chars().any(char::is_control) {
        return Err(ServiceError::Message(
            "El nombre contiene caracteres no válidos".to_string(),
        ));
    }
    Ok(nombre.to_string())
}

fn hash_password(password: &str) -> String {
    let salt = format!("{:032x}", rand::random::<u128>());
    let digest = Sha256::digest(format!("{salt}${password}").as_bytes());
    format!("{salt}${}", hex(&digest))
}

fn verify_password(password: &str, stored: &str) -> bool {
    let Some((salt, expected)) = stored.split_once('$') else {
        return false;
    };
    let actual = hex(&Sha256::digest(format!("{salt}${password}").as_bytes()));
    actual == expected
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut salida = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(salida, "{byte:02x}");
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn creates_with_and_without_password() {
        let db = Database::open_in_memory().unwrap();
        let sin = crear(db.conn(), "Ana", None).unwrap();
        assert!(!sin.con_password);
        assert_eq!(sesion(db.conn()).unwrap().usuario.unwrap().nombre, "Ana");

        let con = crear(db.conn(), "Beto", Some("secreta")).unwrap();
        assert!(con.con_password);
        assert!(autenticar(db.conn(), "beto", Some("mala")).is_err());
        assert_eq!(
            autenticar(db.conn(), "Beto", Some("secreta")).unwrap().id,
            con.id
        );
    }

    #[test]
    fn validates_duplicates_and_changes_password() {
        let db = Database::open_in_memory().unwrap();
        crear(db.conn(), "Ana", None).unwrap();
        assert!(crear(db.conn(), "ana", Some("x")).is_err());
        let usuario = listar(db.conn()).unwrap().remove(0);
        cambiar_password(db.conn(), usuario.id, Some("clave")).unwrap();
        assert!(autenticar(db.conn(), "Ana", None).is_err());
        assert!(autenticar(db.conn(), "Ana", Some("clave")).is_ok());
        cerrar_sesion(db.conn()).unwrap();
        assert!(sesion(db.conn()).unwrap().usuario.is_none());
    }
}
