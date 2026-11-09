//! Resultados de la búsqueda global (IMP-03).

/// Tipo de entidad encontrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TipoResultado {
    /// Partido.
    Partido,
    /// Camiseta.
    Camiseta,
    /// Cancha.
    Cancha,
    /// Equipo.
    Equipo,
    /// Jugador.
    Jugador,
    /// Lesión.
    Lesion,
    /// Movimiento financiero.
    Financiamiento,
}

impl TipoResultado {
    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Partido => "Partido",
            Self::Camiseta => "Camiseta",
            Self::Cancha => "Cancha",
            Self::Equipo => "Equipo",
            Self::Jugador => "Jugador",
            Self::Lesion => "Lesión",
            Self::Financiamiento => "Finanzas",
        }
    }
}

/// Un resultado de búsqueda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoBusqueda {
    /// Tipo.
    pub tipo: TipoResultado,
    /// Identificador.
    pub id: i64,
    /// Título principal.
    pub titulo: String,
    /// Subtítulo.
    pub subtitulo: String,
}
