//! Progresión de jugadores y planes (IMP-23).

/// Plan de entrenamiento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEntrenamiento {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Descripción.
    pub descripcion: String,
    /// Duración en semanas.
    pub duracion_semanas: i64,
    /// Sesiones por semana.
    pub sesiones_por_semana: i64,
}

/// Medición semanal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgresionFila {
    /// Id.
    pub id: i64,
    /// Jugador.
    pub jugador_id: i64,
    /// Nombre del jugador.
    pub jugador: String,
    /// Plan.
    pub plan_id: Option<i64>,
    /// Nombre del plan.
    pub plan: String,
    /// Semana.
    pub semana: i64,
    /// Ataque.
    pub ataque: i64,
    /// Defensa.
    pub defensa: i64,
    /// Resistencia.
    pub resistencia: i64,
    /// Velocidad.
    pub velocidad: i64,
    /// Técnica.
    pub tecnica: i64,
}

impl ProgresionFila {
    /// Promedio de las cinco habilidades.
    #[must_use]
    pub fn promedio(&self) -> f64 {
        (self.ataque + self.defensa + self.resistencia + self.velocidad + self.tecnica) as f64 / 5.0
    }
}
