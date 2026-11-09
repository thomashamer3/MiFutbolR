//! Referencias multimedia (IMP-21).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::media_repo;
use crate::domain::media::MediaItem;

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

/// Lista referencias.
pub fn listar(conn: &Connection, tipo: Option<&str>) -> Result<Vec<MediaItem>, ServiceError> {
    Ok(media_repo::list(conn, tipo)?)
}

/// Crea una referencia.
pub fn crear(conn: &Connection, item: &MediaItem) -> Result<i64, ServiceError> {
    validar(item)?;
    Ok(media_repo::insert(conn, item)?)
}

/// Actualiza una referencia.
pub fn actualizar(conn: &Connection, item: &MediaItem) -> Result<(), ServiceError> {
    validar(item)?;
    media_repo::update(conn, item)?;
    Ok(())
}

/// Elimina una referencia.
pub fn eliminar(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    media_repo::delete(conn, id)?;
    Ok(())
}

fn validar(item: &MediaItem) -> Result<(), ServiceError> {
    if item.titulo.trim().is_empty() {
        return Err(ServiceError::Message(
            "El título de la referencia es obligatorio".to_string(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::domain::media::TipoMedia;

    fn item(tipo: TipoMedia, titulo: &str) -> MediaItem {
        MediaItem {
            id: 0,
            partido_id: None,
            tipo,
            titulo: titulo.to_string(),
            url: String::new(),
            descripcion: String::new(),
            fecha: String::new(),
        }
    }

    #[test]
    fn crud_y_filtro() {
        let db = Database::open_in_memory().unwrap();
        crear(db.conn(), &item(TipoMedia::Video, "Resumen")).unwrap();
        crear(db.conn(), &item(TipoMedia::Foto, "Foto")).unwrap();
        assert_eq!(listar(db.conn(), None).unwrap().len(), 2);
        assert_eq!(listar(db.conn(), Some("foto")).unwrap().len(), 1);
        let mut primero = listar(db.conn(), None).unwrap()[0].clone();
        primero.titulo = "Editado".to_string();
        actualizar(db.conn(), &primero).unwrap();
        eliminar(db.conn(), primero.id).unwrap();
        assert_eq!(listar(db.conn(), None).unwrap().len(), 1);
    }
}
