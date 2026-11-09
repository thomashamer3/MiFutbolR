//! Notificaciones y alertas (IMP-06).

/// Notificación persistida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notificacion {
    /// Identificador.
    pub id: i64,
    /// Tipo lógico (`lesion`, `presupuesto`, `partido`, `racha`, `logro`).
    pub tipo: String,
    /// Severidad: 0 info, 1 aviso, 2 crítico.
    pub severidad: i64,
    /// Título.
    pub titulo: String,
    /// Mensaje.
    pub mensaje: String,
    /// ¿Leída?
    pub leida: bool,
    /// Entidad relacionada.
    pub entidad: String,
    /// Id de la entidad.
    pub entidad_id: Option<i64>,
    /// Fecha.
    pub fecha: String,
}

impl Notificacion {
    /// Etiqueta de severidad.
    #[must_use]
    pub const fn severidad_label(&self) -> &'static str {
        match self.severidad {
            2 => "CRÍTICO",
            1 => "AVISO",
            _ => "INFO",
        }
    }
}
