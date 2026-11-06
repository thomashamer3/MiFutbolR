//! Dominio puro: modelos, dinero, fechas, validaciones y cálculos.
//!
//! Este módulo no conoce la base de datos ni la TUI; se puede probar sin
//! dependencias externas (ADR-06).

pub mod cancha_codes;
pub mod dates;
pub mod models;
pub mod money;
pub mod resumen;
pub mod stats;
pub mod validation;

pub use dates::{display_date, display_datetime, month_key, parse_date, parse_datetime};
pub use models::*;
pub use money::Cents;
pub use validation::DomainError;
