//! Tiendas (IMP-20).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::tienda_repo;
use crate::domain::tiendas::Tienda;

/// Error del servicio.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Persistencia.
    #[error(transparent)]
    Db(#[from] DbError),
    /// Validación.
    #[error("{0}")]
    Message(String),
}

/// Lista tiendas.
pub fn listar(conn: &Connection) -> Result<Vec<Tienda>, ServiceError> {
    Ok(tienda_repo::list(conn)?)
}

/// Crea una tienda.
pub fn crear(conn: &Connection, tienda: &Tienda) -> Result<i64, ServiceError> {
    validar(tienda)?;
    Ok(tienda_repo::insert(conn, tienda)?)
}

/// Actualiza una tienda.
pub fn actualizar(conn: &Connection, tienda: &Tienda) -> Result<(), ServiceError> {
    validar(tienda)?;
    tienda_repo::update(conn, tienda)?;
    Ok(())
}

/// Elimina una tienda.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    tienda_repo::delete(conn, id)?;
    Ok(())
}

fn validar(tienda: &Tienda) -> Result<(), ServiceError> {
    if tienda.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "El nombre de la tienda es obligatorio".to_string(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn tienda() -> Tienda {
        Tienda {
            id: 0,
            nombre: "Fútbol Total".to_string(),
            tipo: "física".to_string(),
            url: String::new(),
            telefono: "123".to_string(),
            instagram: String::new(),
            productos: "botines".to_string(),
            rango_precio: "$$".to_string(),
            tiene_envio: true,
            favorito: true,
            notas: String::new(),
        }
    }

    #[test]
    fn crud_tiendas() {
        let db = Database::open_in_memory().unwrap();
        let id = crear(db.conn(), &tienda()).unwrap();
        let lista = listar(db.conn()).unwrap();
        assert_eq!(lista.len(), 1);
        assert!(lista[0].favorito);
        let mut editada = lista[0].clone();
        editada.nombre = "Otra".to_string();
        actualizar(db.conn(), &editada).unwrap();
        assert_eq!(listar(db.conn()).unwrap()[0].nombre, "Otra");
        eliminar(db.conn(), id).unwrap();
        assert!(listar(db.conn()).unwrap().is_empty());
    }
}
