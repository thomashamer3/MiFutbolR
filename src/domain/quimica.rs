//! Química entre jugadores (IMP-24).

/// Dupla con estadísticas conjuntas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dupla {
    /// Id.
    pub id: i64,
    /// Jugador 1.
    pub jugador1_id: i64,
    /// Jugador 2.
    pub jugador2_id: i64,
    /// Nombre del jugador 1.
    pub jugador1: String,
    /// Nombre del jugador 2.
    pub jugador2: String,
    /// Partidos juntos.
    pub partidos: i64,
    /// Victorias juntos.
    pub victorias: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Comentario.
    pub comentario: String,
}

impl Dupla {
    /// Porcentaje de victorias.
    #[must_use]
    pub fn winrate(&self) -> f64 {
        if self.partidos == 0 {
            0.0
        } else {
            self.victorias as f64 * 100.0 / self.partidos as f64
        }
    }

    /// Aportes por partido.
    #[must_use]
    pub fn aporte(&self) -> f64 {
        if self.partidos == 0 {
            0.0
        } else {
            (self.goles + self.asistencias) as f64 / self.partidos as f64
        }
    }
}

/// Mejor combinación calculada desde los equipos asignados a partidos
/// (`mostrar_mejor_quimica_jugadores` de MiFutbolC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MejorDupla {
    /// Jugador A.
    pub jugador_a: String,
    /// Jugador B.
    pub jugador_b: String,
    /// Victorias juntos.
    pub victorias: i64,
    /// Derrotas juntos.
    pub derrotas: i64,
    /// Empates juntos.
    pub empates: i64,
}

impl MejorDupla {
    /// Winrate como en C: victorias / (victorias + derrotas), sin empates.
    #[must_use]
    pub fn winrate(&self) -> f64 {
        let decididos = self.victorias + self.derrotas;
        if decididos == 0 {
            0.0
        } else {
            self.victorias as f64 * 100.0 / decididos as f64
        }
    }
}
