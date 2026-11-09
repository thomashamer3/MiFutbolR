//! Servicios de negocio: validan, orquestan transacciones y llaman a los
//! repositorios. La UI solo habla con esta capa (§4.1).
//!
//! Las implementaciones se agregan módulo por módulo; por ahora el árbol se
//! reserva para mantener estable la estructura del proyecto.

pub mod amigos_service;
pub mod botin_service;
pub mod busqueda_service;
pub mod camiseta_service;
pub mod cancha_service;
pub mod carrera_service;
pub mod comparador_service;
pub mod equipo_service;
pub mod estadisticas_service;
pub mod export_service;
pub mod financiamiento_service;
pub mod inicio_service;
pub mod legacy_import_service;
pub mod lesion_service;
pub mod logros_service;
pub mod media_service;
pub mod notificacion_service;
pub mod partido_service;
pub mod progresion_service;
pub mod quimica_service;
pub mod recordatorios_service;
pub mod records_service;
pub mod resumen_service;
pub mod temporadas_service;
pub mod tiendas_service;
pub mod torneos_service;
pub mod ultimos_service;
pub mod undo_service;
