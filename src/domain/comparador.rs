//! Comparador A/B de entidades (IMP-14).

/// Entidad comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipoComparacion {
    /// Camisetas.
    #[default]
    Camiseta,
    /// Canchas.
    Cancha,
}

impl TipoComparacion {
    /// Todas.
    pub const ALL: [Self; 2] = [Self::Camiseta, Self::Cancha];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Camiseta => "Camisetas",
            Self::Cancha => "Canchas",
        }
    }
}

/// Opción seleccionable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcionComparador {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
}

/// Agregado de partidos de una entidad.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgregadoComparacion {
    /// Nombre.
    pub nombre: String,
    /// Partidos.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
    /// Rendimiento promedio.
    pub rendimiento: f64,
}

/// Lado ganador de una métrica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lado {
    /// A.
    A,
    /// B.
    B,
    /// Empate.
    Empate,
}

/// Una fila comparada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaComparacion {
    /// Métrica.
    pub metrica: &'static str,
    /// Valor de A.
    pub a: String,
    /// Valor de B.
    pub b: String,
    /// Ganador.
    pub mejor: Lado,
}

/// Resultado completo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparacion {
    /// Nombre de A.
    pub titulo_a: String,
    /// Nombre de B.
    pub titulo_b: String,
    /// Filas.
    pub filas: Vec<FilaComparacion>,
}
