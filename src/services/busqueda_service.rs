//! Búsqueda global (IMP-03).

use rusqlite::Connection;

use crate::db::busqueda_repo;
use crate::db::error::DbError;
use crate::domain::busqueda::ResultadoBusqueda;

/// Busca en todas las entidades.
pub fn buscar(
    conn: &Connection,
    texto: &str,
    limite: usize,
) -> Result<Vec<ResultadoBusqueda>, DbError> {
    busqueda_repo::buscar(conn, texto, limite)
}
