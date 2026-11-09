//! Catálogo de logros derivados de los partidos (IMP-04).

/// Niveles de cada serie.
pub const NIVELES: [&str; 5] = ["Novato", "Promedio", "Experto", "Maestro", "Leyenda"];

/// Definición estática de un logro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogroDef {
    /// Identificador estable.
    pub id: String,
    /// Nombre visible.
    pub nombre: String,
    /// Descripción.
    pub descripcion: String,
    /// Categoría.
    pub categoria: String,
    /// Tipo de métrica.
    pub tipo: String,
    /// Objetivo.
    pub objetivo: i64,
}

/// Progreso de un logro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogroProgreso {
    /// Definición.
    pub def: LogroDef,
    /// Valor actual.
    pub valor: i64,
    /// ¿Completado?
    pub completado: bool,
    /// ¿Ya se registró el desbloqueo?
    pub desbloqueado: bool,
}

impl LogroProgreso {
    /// Progreso entre 0 y 1.
    #[must_use]
    pub fn progreso(&self) -> f64 {
        if self.def.objetivo <= 0 {
            return 1.0;
        }
        (self.valor as f64 / self.def.objetivo as f64).clamp(0.0, 1.0)
    }

    /// Texto "valor/objetivo".
    #[must_use]
    pub fn texto_progreso(&self) -> String {
        format!(
            "{}/{}",
            self.valor.min(self.def.objetivo),
            self.def.objetivo
        )
    }
}

fn serie(
    out: &mut Vec<LogroDef>,
    prefijo: &str,
    verbo: &str,
    objeto: &str,
    tipo: &str,
    categoria: &str,
    objetivos: &[i64],
) {
    for (indice, objetivo) in objetivos.iter().enumerate() {
        let nivel = NIVELES.get(indice).copied().unwrap_or("Leyenda");
        out.push(LogroDef {
            id: format!("{tipo}_{objetivo}"),
            nombre: format!("{prefijo} {nivel}"),
            descripcion: format!("{verbo} {objetivo} {objeto}"),
            categoria: categoria.to_string(),
            tipo: tipo.to_string(),
            objetivo: *objetivo,
        });
    }
}

/// Construye el catálogo de logros.
#[must_use]
pub fn definiciones() -> Vec<LogroDef> {
    let mut out = Vec::new();

    serie(
        &mut out,
        "Goleador",
        "Anotar",
        "goles",
        "goles",
        "Ataque",
        &[1, 5, 10, 25, 50],
    );
    serie(
        &mut out,
        "Asistente",
        "Dar",
        "asistencias",
        "asistencias",
        "Ataque",
        &[1, 5, 10, 25, 50],
    );
    serie(
        &mut out,
        "Jugador",
        "Jugar",
        "partidos",
        "partidos",
        "Constancia",
        &[1, 5, 10, 25, 50],
    );
    serie(
        &mut out,
        "Ganador",
        "Ganar",
        "partidos",
        "victorias",
        "Resultados",
        &[1, 5, 10, 25],
    );
    serie(
        &mut out,
        "Empatador",
        "Empatar",
        "partidos",
        "empates",
        "Resultados",
        &[1, 5, 10],
    );
    serie(
        &mut out,
        "Perdedor",
        "Perder",
        "partidos",
        "derrotas",
        "Resultados",
        &[1, 5, 10],
    );
    serie(
        &mut out,
        "Contribuidor",
        "Acumular",
        "puntos (goles + asistencias)",
        "contribuciones",
        "Ataque",
        &[10, 25, 50, 100],
    );
    serie(
        &mut out,
        "Rendimiento",
        "Acumular",
        "puntos de rendimiento",
        "rendimiento_total",
        "Rendimiento",
        &[50, 100, 250, 500],
    );
    serie(
        &mut out,
        "Ánimo",
        "Acumular",
        "puntos de estado de ánimo",
        "animo_total",
        "Mental",
        &[50, 100, 250],
    );
    serie(
        &mut out,
        "Viajero",
        "Jugar en",
        "canchas distintas",
        "canchas",
        "Exploración",
        &[1, 3, 5, 10],
    );
    serie(
        &mut out,
        "Hat-Tricker",
        "Anotar 3 o más goles en",
        "partidos",
        "hat_tricks",
        "Ataque",
        &[1, 3, 5, 10],
    );
    serie(
        &mut out,
        "Poker",
        "Dar 4 o más asistencias en",
        "partidos",
        "poker",
        "Ataque",
        &[1, 3, 5],
    );
    serie(
        &mut out,
        "Perfecto",
        "Obtener rendimiento 10 en",
        "partidos",
        "rendimiento_perfecto",
        "Rendimiento",
        &[1, 3, 5],
    );
    serie(
        &mut out,
        "Zen",
        "Obtener ánimo 10 en",
        "partidos",
        "animo_perfecto",
        "Mental",
        &[1, 3, 5],
    );
    serie(
        &mut out,
        "Racha Ganadora",
        "Ganar",
        "partidos consecutivos",
        "racha_victorias",
        "Resultados",
        &[3, 5, 10],
    );
    serie(
        &mut out,
        "Racha Perdedora",
        "Perder",
        "partidos consecutivos",
        "racha_derrotas",
        "Resultados",
        &[3, 5],
    );
    serie(
        &mut out,
        "Goleador Victorioso",
        "Anotar",
        "goles en victorias",
        "goles_victorias",
        "Resultados",
        &[10, 25, 50],
    );
    serie(
        &mut out,
        "Asistente Victorioso",
        "Dar",
        "asistencias en victorias",
        "asistencias_victorias",
        "Resultados",
        &[10, 25],
    );
    serie(
        &mut out,
        "Último Goleador",
        "Anotar",
        "goles en el último partido",
        "ultimo_goles",
        "Actualidad",
        &[1, 3],
    );
    serie(
        &mut out,
        "Última Asistencia",
        "Dar",
        "asistencias en el último partido",
        "ultimo_asistencias",
        "Actualidad",
        &[1, 3],
    );

    out
}
