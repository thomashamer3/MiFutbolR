//! Servicios de negocio: validan, orquestan transacciones y llaman a los
//! repositorios. La UI solo habla con esta capa (§4.1).
//!
//! Las implementaciones se agregan módulo por módulo; por ahora el árbol se
//! reserva para mantener estable la estructura del proyecto.

pub mod botin_service;
pub mod camiseta_service;
pub mod cancha_service;
pub mod carrera_service;
pub mod equipo_service;
pub mod estadisticas_service;
pub mod financiamiento_service;
pub mod legacy_import_service;
pub mod lesion_service;
pub mod partido_service;
pub mod resumen_service;
