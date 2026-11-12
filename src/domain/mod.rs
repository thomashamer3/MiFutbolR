//! Dominio puro: modelos, dinero, fechas, validaciones y cálculos.
//!
//! Este módulo no conoce la base de datos ni la TUI; se puede probar sin
//! dependencias externas (ADR-06).

pub mod amigos;
pub mod busqueda;
pub mod calendario;
pub mod cancha_codes;
pub mod comparador;
pub mod dates;
pub mod inicio;
pub mod logros;
pub mod media;
pub mod models;
pub mod money;
pub mod notificacion;
pub mod progresion;
pub mod quimica;
pub mod recordatorios;
pub mod records;
pub mod resumen;
pub mod stats;
pub mod temporadas;
pub mod tiendas;
pub mod torneos;
pub mod ultimos;
pub mod usuario;
pub mod validation;

pub use dates::{display_date, display_datetime, month_key, parse_date, parse_datetime};
pub use models::*;
pub use money::Cents;
pub use validation::DomainError;
