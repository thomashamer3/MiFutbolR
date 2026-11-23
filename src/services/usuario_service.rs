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
        let password = password.unwrap_or("");
        if !verify_password(password, &hash) {
            return Err(ServiceError::Message("Contraseña incorrecta".to_string()));
        }
        // Hash importado de MiFutbolC: se reemplaza por el formato propio.
        if hash.starts_with(PREFIJO_C) {
            usuario_repo::update_password(conn, usuario.id, &hash_password(password), true)?;
        }
    }
    usuario_repo::set_sesion(conn, Some(usuario.id))?;
    usuario_repo::touch(conn, usuario.id)?;
    Ok(usuario_repo::get(conn, usuario.id)?)
}

/// Registra una cuenta importada con su hash ya calculado (no cambia la sesión).
pub fn importar_cuenta(
    conn: &Connection,
    nombre: &str,
    hash_almacenado: &str,
    con_password: bool,
) -> Result<Usuario, ServiceError> {
    let nombre = validar_nombre(nombre)?;
    if usuario_repo::get_by_nombre(conn, &nombre)?.is_some() {
        return Err(ServiceError::Message(format!(
            "Ya existe una cuenta llamada {nombre}"
        )));
    }
    let id = usuario_repo::insert(conn, &nombre, hash_almacenado, con_password)?;
    Ok(usuario_repo::get(conn, id)?)
}

/// Formato almacenado de un hash de MiFutbolC: `fnv1$<salt>$<hash>`.
#[must_use]
pub fn hash_mifutbolc(salt: &str, hash: &str) -> String {
    format!("{PREFIJO_C}{salt}${hash}")
}

/// Cambia o quita la contraseña de una cuenta; exige la actual si la tiene.
pub fn cambiar_password(
    conn: &Connection,
    id: i64,
    actual: Option<&str>,
    password: Option<&str>,
) -> Result<(), ServiceError> {
    verificar_actual(conn, id, actual)?;
    let password = password.map(str::trim).filter(|valor| !valor.is_empty());
    let (hash, con_password) = match password {
        Some(password) => (hash_password(password), true),
        None => (String::new(), false),
    };
    usuario_repo::update_password(conn, id, &hash, con_password)?;
    Ok(())
}

/// Elimina una cuenta; exige su contraseña actual si la tiene.
pub fn eliminar(conn: &Connection, id: i64, actual: Option<&str>) -> Result<(), ServiceError> {
    verificar_actual(conn, id, actual)?;
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

/// Como `auth_confirm_current_password` de MiFutbolC: sin contraseña no se pide nada.
fn verificar_actual(conn: &Connection, id: i64, actual: Option<&str>) -> Result<(), ServiceError> {
    let usuario = usuario_repo::get(conn, id)?;
    if !usuario.con_password {
        return Ok(());
    }
    let hash = usuario_repo::password_hash(conn, &usuario.nombre)?.unwrap_or_default();
    if verify_password(actual.unwrap_or(""), &hash) {
        Ok(())
    } else {
        Err(ServiceError::Message(
            "Contraseña actual incorrecta".to_string(),
        ))
    }
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

/// Prefijo de los hashes importados de MiFutbolC.
const PREFIJO_C: &str = "fnv1$";

/// FNV-1a de 64 bits (el que usa MiFutbolC en `utils.c`).
fn fnv1a64(texto: &str) -> u64 {
    texto
        .bytes()
        .fold(14_695_981_039_346_656_037, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        })
}

/// `auth_build_password_hash` de MiFutbolC: dos rondas de FNV-1a con salt.
fn hash_c(password: &str, salt: &str) -> String {
    let h1 = fnv1a64(&format!("{salt}:{password}"));
    format!("{:016x}", fnv1a64(&format!("{password}:{h1:016x}:{salt}")))
}

fn verify_password(password: &str, stored: &str) -> bool {
    if let Some(resto) = stored.strip_prefix(PREFIJO_C) {
        // shortcut: FNV no es un hash de contraseñas seguro; solo se acepta para
        // migrar cuentas de MiFutbolC y se reemplaza en el primer login correcto.
        return resto
            .split_once('$')
            .is_some_and(|(salt, esperado)| hash_c(password, salt) == esperado);
    }
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
    fn fnv_matches_reference_vectors() {
        // Vectores oficiales de FNV-1a 64.
        assert_eq!(fnv1a64(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64("foobar"), 0x8594_4171_f739_67e8);
        // Composición de MiFutbolC: h2 = fnv("pw:<h1>:salt"), h1 = fnv("salt:pw").
        let h1 = fnv1a64("s:pw");
        assert_eq!(
            hash_c("pw", "s"),
            format!("{:016x}", fnv1a64(&format!("pw:{h1:016x}:s")))
        );
        // Salida de auth_build_password_hash compilado desde utils.c de MiFutbolC.
        assert_eq!(hash_c("clave", "abc123"), "3cb57b64f0155065");
        assert_eq!(hash_c("pw", "s"), "0effe4c9e18d4d1e");
    }

    #[test]
    fn imported_c_hash_logs_in_and_upgrades() {
        let db = Database::open_in_memory().unwrap();
        let hash = hash_mifutbolc("abc123", &hash_c("clave", "abc123"));
        let cuenta = importar_cuenta(db.conn(), "Beto", &hash, true).unwrap();
        assert!(
            sesion(db.conn()).unwrap().usuario.is_none(),
            "importar no inicia sesión"
        );
        assert!(importar_cuenta(db.conn(), "beto", "", false).is_err());

        assert!(autenticar(db.conn(), "Beto", Some("mala")).is_err());
        assert_eq!(
            autenticar(db.conn(), "Beto", Some("clave")).unwrap().id,
            cuenta.id
        );
        let nuevo = usuario_repo::password_hash(db.conn(), "Beto")
            .unwrap()
            .unwrap();
        assert!(!nuevo.starts_with(PREFIJO_C), "se migró al formato propio");
        assert!(autenticar(db.conn(), "Beto", Some("clave")).is_ok());
    }

    #[test]
    fn validates_duplicates_and_changes_password() {
        let db = Database::open_in_memory().unwrap();
        crear(db.conn(), "Ana", None).unwrap();
        assert!(crear(db.conn(), "ana", Some("x")).is_err());
        let usuario = listar(db.conn()).unwrap().remove(0);
        // Sin contraseña previa no se pide la actual.
        cambiar_password(db.conn(), usuario.id, None, Some("clave")).unwrap();
        assert!(autenticar(db.conn(), "Ana", None).is_err());
        assert!(autenticar(db.conn(), "Ana", Some("clave")).is_ok());
        cerrar_sesion(db.conn()).unwrap();
        assert!(sesion(db.conn()).unwrap().usuario.is_none());
    }

    #[test]
    fn change_and_delete_require_current_password() {
        let db = Database::open_in_memory().unwrap();
        let beto = crear(db.conn(), "Beto", Some("clave")).unwrap();
        assert!(cambiar_password(db.conn(), beto.id, None, None).is_err());
        assert!(cambiar_password(db.conn(), beto.id, Some("mala"), Some("otra")).is_err());
        assert!(eliminar(db.conn(), beto.id, Some("mala")).is_err());
        assert!(
            autenticar(db.conn(), "Beto", Some("clave")).is_ok(),
            "nada cambió"
        );

        cambiar_password(db.conn(), beto.id, Some("clave"), Some("otra")).unwrap();
        assert!(autenticar(db.conn(), "Beto", Some("otra")).is_ok());
        assert!(eliminar(db.conn(), beto.id, Some("clave")).is_err());
        eliminar(db.conn(), beto.id, Some("otra")).unwrap();
        assert!(listar(db.conn()).unwrap().is_empty());

        // Hash heredado de MiFutbolC: también vale como contraseña actual.
        let hash = hash_mifutbolc("abc123", &hash_c("clave", "abc123"));
        let vieja = importar_cuenta(db.conn(), "Vieja", &hash, true).unwrap();
        assert!(eliminar(db.conn(), vieja.id, None).is_err());
        eliminar(db.conn(), vieja.id, Some("clave")).unwrap();

        let ana = crear(db.conn(), "Ana", None).unwrap();
        eliminar(db.conn(), ana.id, None).unwrap();
    }
}
