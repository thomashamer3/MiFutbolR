//! Utilidades compartidas por los repositorios: paginación, booleanos y
//! mapeos tolerantes de filas.

use rusqlite::Row;
use rusqlite::types::Value;

use super::error::DbError;

/// Ventana de paginación (§20.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// Tamaño de página; `0` significa "todos".
    pub limit: i64,
    /// Desplazamiento.
    pub offset: i64,
}

impl Page {
    /// Página completa (sin límite).
    pub const ALL: Self = Self {
        limit: 0,
        offset: 0,
    };

    /// Construye una página 1-indexada.
    #[must_use]
    pub fn numbered(page: i64, limit: i64) -> Self {
        let page = page.max(1);
        let limit = limit.max(1);
        Self {
            limit,
            offset: (page - 1) * limit,
        }
    }

    /// `LIMIT` a usar en SQL (`-1` = sin límite).
    #[must_use]
    pub const fn sql_limit(self) -> i64 {
        if self.limit <= 0 { -1 } else { self.limit }
    }

    /// `OFFSET` a usar en SQL.
    #[must_use]
    pub const fn sql_offset(self) -> i64 {
        if self.limit <= 0 { 0 } else { self.offset }
    }
}

/// Convierte un `i64` 0/1 de SQLite a `bool`.
#[must_use]
pub fn bool_from_i64(value: i64) -> bool {
    value != 0
}

/// Convierte un `bool` a `i64` 0/1 para SQLite.
#[must_use]
pub fn i64_from_bool(value: bool) -> i64 {
    i64::from(value)
}

/// Patrón `LIKE` para búsquedas de texto (§20.6).
#[must_use]
pub fn like_pattern(query: &str) -> String {
    let normalized = query.trim().replace('%', r"\%").replace('_', r"\_");
    format!("%{normalized}%")
}

/// Lee una columna `TEXT` tolerando `NULL` (`NULL` → `""`).
pub fn text_column(row: &Row<'_>, index: usize) -> Result<String, DbError> {
    let value: Value = row.get(index)?;
    Ok(match value {
        Value::Null => String::new(),
        Value::Text(text) => text,
        Value::Integer(number) => number.to_string(),
        Value::Real(number) => number.to_string(),
        Value::Blob(_) => String::new(),
    })
}

/// Lee una columna opcional `REAL`.
pub fn opt_f64(row: &Row<'_>, index: usize) -> Result<Option<f64>, DbError> {
    let value: Option<f64> = row.get(index)?;
    Ok(value)
}

/// Lee una columna opcional `INTEGER`.
pub fn opt_i64(row: &Row<'_>, index: usize) -> Result<Option<i64>, DbError> {
    let value: Option<i64> = row.get(index)?;
    Ok(value)
}

/// Convierte un código `-1` del legacy en `None`.
#[must_use]
pub fn opt_from_negative(value: i64) -> Option<i64> {
    if value < 0 { None } else { Some(value) }
}

/// Convierte `Option<i64>` en el `-1` que usa el legacy.
#[must_use]
pub fn negative_from_opt(value: Option<i64>) -> i64 {
    value.unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_numbering() {
        let page = Page::numbered(2, 10);
        assert_eq!(page.offset, 10);
        assert_eq!(page.sql_limit(), 10);
        assert_eq!(Page::ALL.sql_limit(), -1);
        assert_eq!(Page::ALL.sql_offset(), 0);
        assert_eq!(Page::numbered(0, 0).limit, 1);
    }

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern(" 50% "), r"%50\%%");
        assert_eq!(like_pattern("a_b"), r"%a\_b%");
    }

    #[test]
    fn negative_and_optional_helpers() {
        assert_eq!(opt_from_negative(-1), None);
        assert_eq!(opt_from_negative(0), Some(0));
        assert_eq!(negative_from_opt(None), -1);
        assert!(bool_from_i64(5));
        assert!(!bool_from_i64(0));
        assert_eq!(i64_from_bool(true), 1);
    }
}
