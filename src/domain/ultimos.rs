//! Últimos partidos por resultado y aportes (IMP-16).

use crate::domain::inicio::PartidoBreve;

/// Un "último" calculado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UltimoItem {
    /// Etiqueta.
    pub etiqueta: &'static str,
    /// Partido o `None`.
    pub partido: Option<PartidoBreve>,
    /// Tiempo transcurrido.
    pub hace: String,
}
