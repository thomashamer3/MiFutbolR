//! Récords y rankings históricos (IMP-05).

/// Un récord o ranking listo para mostrar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordItem {
    /// Título del reporte.
    pub titulo: String,
    /// Detalle calculado.
    pub detalle: String,
    /// Partido relacionado, si aplica.
    pub partido_id: Option<i64>,
}
