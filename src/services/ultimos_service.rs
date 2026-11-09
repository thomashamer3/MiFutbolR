//! Últimos partidos (IMP-16).

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::ultimos_repo;
use crate::domain::ultimos::UltimoItem;

/// Lista los cinco "últimos".
pub fn listar(conn: &Connection) -> Result<Vec<UltimoItem>, DbError> {
    ultimos_repo::listar(conn)
}
