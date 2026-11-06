//! Errores de la capa de persistencia.

use thiserror::Error;

/// Error de base de datos o de mapeo.
#[derive(Debug, Error)]
pub enum DbError {
    /// Error de SQLite.
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Entidad inexistente.
    #[error("No encontrado: {entidad} id={id}")]
    NotFound {
        /// Nombre de la entidad.
        entidad: &'static str,
        /// Identificador buscado.
        id: i64,
    },
    /// Violación de una restricción.
    #[error("Violación de integridad: {0}")]
    Integrity(String),
    /// Migración fallida.
    #[error("Migración fallida: {0}")]
    Migration(String),
    /// Dato inválido al leer o escribir.
    #[error("Dato inválido: {0}")]
    InvalidData(String),
    /// No se pudo preparar el directorio o el archivo de la BD.
    #[error("E/S: {0}")]
    Io(String),
}

impl From<std::io::Error> for DbError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl DbError {
    /// `true` si es un error de clave foránea (`FOREIGN KEY constraint failed`).
    #[must_use]
    pub fn is_foreign_key(&self) -> bool {
        match self {
            Self::Sqlite(rusqlite::Error::SqliteFailure(err, _)) => {
                matches!(err.code, rusqlite::ErrorCode::ConstraintViolation)
                    && err.extended_code == 787 // SQLITE_CONSTRAINT_FOREIGNKEY
            }
            _ => false,
        }
    }
}
