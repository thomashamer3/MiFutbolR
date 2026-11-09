//! Récords y rankings (IMP-05).

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::records_repo;
use crate::domain::records::RecordItem;

/// Devuelve los 16 reportes.
pub fn listar(conn: &Connection) -> Result<Vec<RecordItem>, DbError> {
    records_repo::todos(conn)
}
