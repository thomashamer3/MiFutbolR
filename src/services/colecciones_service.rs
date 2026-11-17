//! Colecciones e inventario (paridad con MiFutbolC, `colecciones.c`).

use rusqlite::Connection;
use thiserror::Error;

use crate::db::DbError;
use crate::db::coleccion_repo;
use crate::domain::colecciones::{Coleccion, InventarioItem};

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

/// Inventario completo.
pub fn listar_items(conn: &Connection) -> Result<Vec<InventarioItem>, ServiceError> {
    Ok(coleccion_repo::list_items(conn)?)
}

/// Ítems de una colección.
pub fn items_de(conn: &Connection, coleccion_id: i64) -> Result<Vec<InventarioItem>, ServiceError> {
    Ok(coleccion_repo::items_de(conn, coleccion_id)?)
}

/// Crea o actualiza un ítem. Una camiseta vinculada toma su nombre de la camiseta.
pub fn guardar_item(conn: &Connection, item: &InventarioItem) -> Result<i64, ServiceError> {
    let mut item = item.clone();
    if item.tipo != 1 {
        item.camiseta_id = None;
    }
    if item.camiseta_id.is_some() {
        item.nombre.clear();
    } else if item.nombre.trim().is_empty() {
        return Err(ServiceError::Message(
            "Indicá un nombre o vinculá una camiseta".to_string(),
        ));
    }
    if !item.valor.is_finite() || item.valor < 0.0 {
        return Err(ServiceError::Message(
            "El valor no puede ser negativo".to_string(),
        ));
    }
    coleccion_repo::save_item(conn, &item).map_err(|error| match error {
        DbError::Sqlite(ref inner) if inner.to_string().contains("UNIQUE") => {
            ServiceError::Message("Esa camiseta ya está en el inventario".to_string())
        }
        otro => otro.into(),
    })
}

/// Elimina un ítem.
pub fn eliminar_item(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    coleccion_repo::delete_item(conn, id)?;
    Ok(())
}

/// Agrega al inventario las camisetas que faltan; devuelve cuántas.
pub fn sincronizar_camisetas(conn: &Connection) -> Result<usize, ServiceError> {
    Ok(coleccion_repo::sincronizar_camisetas(conn)?)
}

/// Colecciones.
pub fn listar_colecciones(conn: &Connection) -> Result<Vec<Coleccion>, ServiceError> {
    Ok(coleccion_repo::list_colecciones(conn)?)
}

/// Crea (`id == 0`) o actualiza una colección.
pub fn guardar_coleccion(
    conn: &Connection,
    id: i64,
    nombre: &str,
    descripcion: &str,
) -> Result<i64, ServiceError> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(ServiceError::Message(
            "El nombre de la colección es obligatorio".to_string(),
        ));
    }
    coleccion_repo::save_coleccion(conn, id, nombre, descripcion.trim()).map_err(
        |error| match error {
            DbError::Sqlite(ref inner) if inner.to_string().contains("UNIQUE") => {
                ServiceError::Message("Ya existe una colección con ese nombre".to_string())
            }
            otro => otro.into(),
        },
    )
}

/// Elimina una colección (los ítems siguen en el inventario).
pub fn eliminar_coleccion(conn: &Connection, id: i64) -> Result<(), ServiceError> {
    coleccion_repo::delete_coleccion(conn, id)?;
    Ok(())
}

/// Agrega un ítem a una colección; `false` si ya estaba.
pub fn agregar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<bool, ServiceError> {
    Ok(coleccion_repo::agregar(conn, coleccion_id, item_id)?)
}

/// Quita un ítem de una colección.
pub fn quitar(conn: &Connection, coleccion_id: i64, item_id: i64) -> Result<(), ServiceError> {
    coleccion_repo::quitar(conn, coleccion_id, item_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn item(tipo: i64, nombre: &str, camiseta_id: Option<i64>) -> InventarioItem {
        InventarioItem {
            id: 0,
            tipo,
            nombre: nombre.to_string(),
            nombre_visible: String::new(),
            estado: 0,
            valor: 100.0,
            fecha_compra: String::new(),
            camiseta_id,
            colecciones: 0,
        }
    }

    #[test]
    fn seeds_sync_and_membership() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        // Las tres colecciones por defecto de MiFutbolC.
        assert_eq!(listar_colecciones(conn).unwrap().len(), 3);

        conn.execute(
            "INSERT INTO camiseta (nombre) VALUES ('Titular'), ('Suplente')",
            [],
        )
        .unwrap();
        assert_eq!(sincronizar_camisetas(conn).unwrap(), 2);
        assert_eq!(sincronizar_camisetas(conn).unwrap(), 0, "idempotente");
        let items = listar_items(conn).unwrap();
        assert_eq!(items[0].nombre_visible, "Titular");

        let botin = guardar_item(conn, &item(2, "Predator", None)).unwrap();
        assert!(guardar_item(conn, &item(3, "  ", None)).is_err());
        // Una camiseta ya inventariada no se puede repetir.
        assert!(guardar_item(conn, &item(1, "", items[0].camiseta_id)).is_err());

        let actual = listar_colecciones(conn).unwrap()[0].id;
        assert!(agregar(conn, actual, botin).unwrap());
        assert!(!agregar(conn, actual, botin).unwrap());
        assert_eq!(items_de(conn, actual).unwrap().len(), 1);
        assert_eq!(listar_colecciones(conn).unwrap()[0].items, 1);
        quitar(conn, actual, botin).unwrap();
        assert!(items_de(conn, actual).unwrap().is_empty());

        // Borrar la camiseta deja el ítem desvinculado con nombre de reemplazo.
        conn.execute("DELETE FROM camiseta WHERE nombre = 'Suplente'", [])
            .unwrap();
        assert!(
            listar_items(conn)
                .unwrap()
                .iter()
                .any(|i| i.nombre_visible == "(Camiseta eliminada)")
        );
    }

    #[test]
    fn coleccion_names_are_unique() {
        let db = Database::open_in_memory().unwrap();
        let id = guardar_coleccion(db.conn(), 0, "Retro", "").unwrap();
        assert!(guardar_coleccion(db.conn(), 0, "Retro", "").is_err());
        assert!(guardar_coleccion(db.conn(), 0, " ", "").is_err());
        eliminar_coleccion(db.conn(), id).unwrap();
        assert_eq!(listar_colecciones(db.conn()).unwrap().len(), 3);
    }
}
