//! Entrenador IA (reglas de MiFutbolC, `entrenador_ia.c`).

/// Partido reciente que usa el entrenador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartidoIa {
    /// `YYYY-MM-DD HH:MM:SS`.
    pub fecha_hora: String,
    /// Rendimiento general 0-10.
    pub rendimiento: i64,
    /// Cansancio 0-10.
    pub cansancio: i64,
    /// Estado de ánimo 0-10.
    pub animo: i64,
    /// 1 victoria, 2 empate, 3 derrota, 0 sin cargar.
    pub resultado: i64,
}

/// Estado del jugador según los últimos 10 partidos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EstadoJugador {
    /// Partidos considerados.
    pub partidos: usize,
    /// Rendimiento promedio.
    pub rendimiento: f64,
    /// Cansancio promedio.
    pub cansancio: f64,
    /// Ánimo promedio.
    pub animo: f64,
    /// Días desde el último partido.
    pub dias_descanso: i64,
    /// Partidos de los últimos 7 jugados en la última semana.
    pub consecutivos: i64,
    /// Derrotas seguidas al inicio de los últimos 5.
    pub derrotas: i64,
    /// Factor de recuperación del cansancio (1 recién jugado, 0 tras 14 días).
    pub factor: f64,
    /// Riesgo de lesión (escala MiFutbolC, > 3 crítico).
    pub riesgo: f64,
}

/// Severidad de un consejo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severidad {
    /// Informativo.
    Info,
    /// Advertencia.
    Advertencia,
    /// Crítico.
    Critico,
}

impl Severidad {
    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Advertencia => "ADVERTENCIA",
            Self::Critico => "CRÍTICO",
        }
    }
}

/// Consejo generado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consejo {
    /// Texto.
    pub texto: &'static str,
    /// Severidad.
    pub severidad: Severidad,
    /// Área (Físico, Deportivo, Mental, Salud).
    pub area: &'static str,
}

/// Agregados de todos los partidos para los puntos débiles.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Agregados {
    /// Partidos totales.
    pub partidos: i64,
    /// Goles promedio.
    pub goles: f64,
    /// Asistencias promedio.
    pub asistencias: f64,
    /// Rendimiento máximo.
    pub rendimiento_max: i64,
    /// Rendimiento mínimo.
    pub rendimiento_min: i64,
    /// Lesiones registradas.
    pub lesiones: i64,
}

/// Análisis completo que muestra la pantalla.
#[derive(Debug, Clone, PartialEq)]
pub struct Analisis {
    /// Estado del jugador.
    pub estado: EstadoJugador,
    /// Consejos actuales.
    pub consejos: Vec<Consejo>,
    /// Probabilidades (victoria, empate, derrota) en %.
    pub prediccion: (f64, f64, f64),
    /// Formación sugerida.
    pub formacion: &'static str,
    /// Recomendaciones tácticas extra.
    pub tacticas: Vec<&'static str>,
    /// Días de descanso sugeridos y prioridad.
    pub descanso: (i64, &'static str),
    /// Nota crítica de descanso.
    pub nota_descanso: Option<&'static str>,
    /// Puntos débiles detectados.
    pub puntos_debiles: Vec<String>,
    /// Nivel de intervención configurado (0-3).
    pub nivel: i64,
    /// ¿Correspondería alertar antes del próximo partido?
    pub alerta_previa: bool,
    /// Consejos seguidos / ignorados en el historial.
    pub historial: (i64, i64),
}
