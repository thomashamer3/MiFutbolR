//! Errores y validadores de dominio reutilizables.
//!
//! Las validaciones se ejecutan **antes** de tocar la base de datos y no
//! devuelven `DbError`: son fallas de negocio, no de persistencia.

use chrono::{NaiveDate, NaiveDateTime};
use thiserror::Error;

use super::dates;
use super::money::{Cents, MoneyError};

/// Error de validación de dominio.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// Un enum numérico recibió un código desconocido.
    #[error("Valor inválido para {field}: {value}")]
    InvalidEnum {
        /// Nombre del campo o tipo.
        field: &'static str,
        /// Código recibido.
        value: i64,
    },
    /// Falta un campo obligatorio.
    #[error("Campo obligatorio: {0}")]
    Required(&'static str),
    /// Texto fuera de los límites de longitud.
    #[error("{field} debe tener entre {min} y {max} caracteres")]
    Length {
        /// Nombre del campo.
        field: &'static str,
        /// Mínimo permitido.
        min: usize,
        /// Máximo permitido.
        max: usize,
    },
    /// Número fuera de rango.
    #[error("{field} fuera de rango: {value} (permitido {min}..={max})")]
    OutOfRange {
        /// Nombre del campo.
        field: &'static str,
        /// Mínimo permitido.
        min: i64,
        /// Máximo permitido.
        max: i64,
        /// Valor recibido.
        value: i64,
    },
    /// El valor debe ser mayor que cero.
    #[error("{field} debe ser mayor que cero")]
    MustBePositive {
        /// Nombre del campo.
        field: &'static str,
    },
    /// Fecha con formato inválido.
    #[error("Fecha inválida: {0}")]
    InvalidDate(String),
    /// Importe con formato inválido.
    #[error("Importe inválido: {0}")]
    InvalidMoney(String),
    /// Error libre con mensaje listo para mostrar.
    #[error("{0}")]
    Message(String),
}

impl From<MoneyError> for DomainError {
    fn from(value: MoneyError) -> Self {
        Self::InvalidMoney(value.to_string())
    }
}

/// Exige texto no vacío y devuelve el recorte.
pub fn required<'a>(text: &'a str, field: &'static str) -> Result<&'a str, DomainError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Err(DomainError::Required(field))
    } else {
        Ok(trimmed)
    }
}

/// Exige longitud máxima (en caracteres, no bytes).
pub fn max_len(text: &str, field: &'static str, max: usize) -> Result<(), DomainError> {
    if text.chars().count() > max {
        Err(DomainError::Length { field, min: 0, max })
    } else {
        Ok(())
    }
}

/// Exige longitud entre `min` y `max`.
pub fn length_between(
    text: &str,
    field: &'static str,
    min: usize,
    max: usize,
) -> Result<(), DomainError> {
    let len = text.chars().count();
    if len < min || len > max {
        Err(DomainError::Length { field, min, max })
    } else {
        Ok(())
    }
}

/// Exige un entero dentro de `min..=max`.
pub fn range_i64(value: i64, min: i64, max: i64, field: &'static str) -> Result<(), DomainError> {
    if !(min..=max).contains(&value) {
        Err(DomainError::OutOfRange {
            field,
            min,
            max,
            value,
        })
    } else {
        Ok(())
    }
}

/// Exige un entero mayor o igual que cero.
pub fn non_negative(value: i64, field: &'static str) -> Result<(), DomainError> {
    if value < 0 {
        Err(DomainError::OutOfRange {
            field,
            min: 0,
            max: i64::MAX,
            value,
        })
    } else {
        Ok(())
    }
}

/// Exige un entero mayor que cero.
pub fn positive(value: i64, field: &'static str) -> Result<(), DomainError> {
    if value <= 0 {
        Err(DomainError::MustBePositive { field })
    } else {
        Ok(())
    }
}

/// Exige un decimal finito dentro de rango.
pub fn range_f64(value: f64, min: f64, max: f64, field: &'static str) -> Result<(), DomainError> {
    if value.is_finite() && value >= min && value <= max {
        Ok(())
    } else {
        Err(DomainError::Message(format!(
            "{field} fuera de rango: {value} (permitido {min}..={max})"
        )))
    }
}

/// Parsea un importe escrito por el usuario.
pub fn money(text: &str, field: &'static str) -> Result<Cents, DomainError> {
    Cents::parse_user_input(text)
        .map_err(|err| DomainError::InvalidMoney(format!("{field}: {err}")))
}

/// Parsea una fecha tolerando ISO y `DD/MM/YYYY`.
pub fn date(text: &str, field: &'static str) -> Result<NaiveDate, DomainError> {
    dates::parse_date(text).ok_or_else(|| DomainError::InvalidDate(format!("{field}: {text}")))
}

/// Parsea una fecha-hora tolerando ISO y `DD/MM/YYYY HH:MM`.
pub fn datetime(text: &str, field: &'static str) -> Result<NaiveDateTime, DomainError> {
    dates::parse_datetime(text).ok_or_else(|| DomainError::InvalidDate(format!("{field}: {text}")))
}

/// Valida la extensión de una ruta de imagen (vacía es válida).
pub fn image_path(ruta: &str) -> Result<(), DomainError> {
    let trimmed = ruta.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
    let lower = trimmed.to_lowercase();
    if EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(&format!(".{extension}")))
    {
        Ok(())
    } else {
        Err(DomainError::Message(format!(
            "Extensión de imagen no permitida: {trimmed}"
        )))
    }
}

/// Parsea un booleano de formulario (`s/n`, `si/no`, `1/0`).
pub fn parse_bool(text: &str) -> Result<bool, DomainError> {
    match text.trim().to_lowercase().as_str() {
        "s" | "si" | "sí" | "1" | "true" | "verdadero" | "y" | "yes" => Ok(true),
        "n" | "no" | "0" | "false" | "falso" => Ok(false),
        other => Err(DomainError::Message(format!(
            "Valor booleano inválido: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_trims_and_rejects_blank() {
        assert_eq!(required("  hola ", "nombre").unwrap(), "hola");
        assert_eq!(
            required("   ", "nombre"),
            Err(DomainError::Required("nombre"))
        );
    }

    #[test]
    fn range_and_positive_checks() {
        assert!(range_i64(5, 0, 10, "x").is_ok());
        assert!(range_i64(11, 0, 10, "x").is_err());
        assert!(positive(0, "x").is_err());
        assert!(non_negative(-1, "x").is_err());
    }

    #[test]
    fn parses_bools_tolerantly() {
        assert!(parse_bool("S").unwrap());
        assert!(parse_bool("si").unwrap());
        assert!(!parse_bool("No").unwrap());
        assert!(parse_bool("quizas").is_err());
    }

    #[test]
    fn money_and_date_helpers_wrap_errors() {
        assert_eq!(money("1.500,50", "costo").unwrap(), Cents(150_050));
        assert!(money("nada", "costo").is_err());
        assert!(date("2026-11-06", "fecha").is_ok());
        assert!(date("31/02/2026", "fecha").is_err());
        assert!(datetime("2026-11-06 21:30", "fecha_hora").is_ok());
    }
}
