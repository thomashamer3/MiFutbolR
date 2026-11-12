//! Cuentas locales de MiFutbolR.

/// Cuenta de usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usuario {
    /// Id.
    pub id: i64,
    /// Nombre visible.
    pub nombre: String,
    /// ¿Tiene contraseña?
    pub con_password: bool,
    /// Fecha de creación.
    pub creado_en: String,
    /// Último acceso.
    pub ultimo_acceso: String,
}

/// Estado de la sesión local.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sesion {
    /// Usuario actual, si hay.
    pub usuario: Option<Usuario>,
    /// ¿Se vio la pantalla de bienvenida?
    pub bienvenida_vista: bool,
}
