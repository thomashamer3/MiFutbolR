//! Torneos y fixture (IMP-11).

/// Tipo de torneo soportado por el fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipoTorneo {
    /// Todos contra todos a una rueda.
    #[default]
    SoloIda,
    /// Todos contra todos, ida y vuelta.
    IdaYVuelta,
}

impl TipoTorneo {
    /// Todos los tipos.
    pub const ALL: [Self; 2] = [Self::SoloIda, Self::IdaYVuelta];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SoloIda => "Solo ida",
            Self::IdaYVuelta => "Ida y vuelta",
        }
    }

    /// Código persistido.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        match self {
            Self::SoloIda => 0,
            Self::IdaYVuelta => 1,
        }
    }

    /// Desde el código persistido.
    #[must_use]
    pub const fn from_i64(valor: i64) -> Self {
        if valor == 1 {
            Self::IdaYVuelta
        } else {
            Self::SoloIda
        }
    }
}

/// Torneo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Torneo {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Tipo.
    pub tipo: TipoTorneo,
    /// Equipos asociados.
    pub cantidad_equipos: i64,
    /// Fase actual.
    pub fase_actual: String,
}

/// Equipo asociado a un torneo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorneoEquipo {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
}

/// Partido del fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartidoTorneoRow {
    /// Id.
    pub id: i64,
    /// Id del local.
    pub local_id: i64,
    /// Local.
    pub local: String,
    /// Id del visitante.
    pub visitante_id: i64,
    /// Visitante.
    pub visitante: String,
    /// Goles local.
    pub goles_local: i64,
    /// Goles visitante.
    pub goles_visitante: i64,
    /// Estado (`Pendiente`/`Jugado`).
    pub estado: String,
    /// Fase.
    pub fase: String,
}

/// Fila de la tabla de posiciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingRow {
    /// Equipo.
    pub equipo: String,
    /// Partidos jugados.
    pub pj: i64,
    /// Ganados.
    pub pg: i64,
    /// Empatados.
    pub pe: i64,
    /// Perdidos.
    pub pp: i64,
    /// Goles a favor.
    pub gf: i64,
    /// Goles en contra.
    pub gc: i64,
    /// Diferencia.
    pub dif: i64,
    /// Puntos.
    pub puntos: i64,
}

/// Ficha completa del torneo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorneoFicha {
    /// Torneo.
    pub torneo: Torneo,
    /// Equipos.
    pub equipos: Vec<TorneoEquipo>,
    /// Fixture.
    pub partidos: Vec<PartidoTorneoRow>,
    /// Tabla.
    pub tabla: Vec<StandingRow>,
}

impl TorneoFicha {
    /// Partidos jugados.
    #[must_use]
    pub fn jugados(&self) -> usize {
        self.partidos
            .iter()
            .filter(|p| p.estado == "Jugado")
            .count()
    }

    /// ¿Fixture generado?
    #[must_use]
    pub fn tiene_fixture(&self) -> bool {
        !self.partidos.is_empty()
    }
}
