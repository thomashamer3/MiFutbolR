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

/// Series del catálogo: (prefijo, verbo, objeto, tipo, categoría, objetivos).
#[allow(clippy::type_complexity)]
type SerieDef = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [i64],
);

const SERIES: &[SerieDef] = &[
    (
        "Goleador",
        "Anotar",
        "goles",
        "goles",
        "Ataque",
        &[1, 5, 10, 25, 50],
    ),
    (
        "Asistente",
        "Dar",
        "asistencias",
        "asistencias",
        "Ataque",
        &[1, 5, 10, 25, 50],
    ),
    (
        "Jugador",
        "Jugar",
        "partidos",
        "partidos",
        "Constancia",
        &[1, 5, 10, 25, 50],
    ),
    (
        "Ganador",
        "Ganar",
        "partidos",
        "victorias",
        "Resultados",
        &[1, 5, 10, 25],
    ),
    (
        "Empatador",
        "Empatar",
        "partidos",
        "empates",
        "Resultados",
        &[1, 5, 10],
    ),
    (
        "Perdedor",
        "Perder",
        "partidos",
        "derrotas",
        "Resultados",
        &[1, 5, 10],
    ),
    (
        "Contribuidor",
        "Acumular",
        "puntos (goles + asistencias)",
        "contribuciones",
        "Ataque",
        &[10, 25, 50, 100],
    ),
    (
        "Rendimiento",
        "Acumular",
        "puntos de rendimiento",
        "rendimiento_total",
        "Rendimiento",
        &[50, 100, 250, 500],
    ),
    (
        "Ánimo",
        "Acumular",
        "puntos de estado de ánimo",
        "animo_total",
        "Mental",
        &[50, 100, 250],
    ),
    (
        "Viajero",
        "Jugar en",
        "canchas distintas",
        "canchas",
        "Exploración",
        &[1, 3, 5, 10],
    ),
    (
        "Hat-Tricker",
        "Anotar 3 o más goles en",
        "partidos",
        "hat_tricks",
        "Ataque",
        &[1, 3, 5, 10],
    ),
    (
        "Poker",
        "Dar 4 o más asistencias en",
        "partidos",
        "poker",
        "Ataque",
        &[1, 3, 5],
    ),
    (
        "Perfecto",
        "Obtener rendimiento 10 en",
        "partidos",
        "rendimiento_perfecto",
        "Rendimiento",
        &[1, 3, 5],
    ),
    (
        "Zen",
        "Obtener ánimo 10 en",
        "partidos",
        "animo_perfecto",
        "Mental",
        &[1, 3, 5],
    ),
    (
        "Racha Ganadora",
        "Ganar",
        "partidos consecutivos",
        "racha_victorias",
        "Resultados",
        &[3, 5, 10],
    ),
    (
        "Racha Perdedora",
        "Perder",
        "partidos consecutivos",
        "racha_derrotas",
        "Resultados",
        &[3, 5],
    ),
    (
        "Goleador Victorioso",
        "Anotar",
        "goles en victorias",
        "goles_victorias",
        "Resultados",
        &[10, 25, 50],
    ),
    (
        "Asistente Victorioso",
        "Dar",
        "asistencias en victorias",
        "asistencias_victorias",
        "Resultados",
        &[10, 25],
    ),
    (
        "Último Goleador",
        "Anotar",
        "goles en el último partido",
        "ultimo_goles",
        "Actualidad",
        &[1, 3],
    ),
    (
        "Última Asistencia",
        "Dar",
        "asistencias en el último partido",
        "ultimo_asistencias",
        "Actualidad",
        &[1, 3],
    ),
];

/// Construye el catálogo de logros.
#[must_use]
pub fn definiciones() -> Vec<LogroDef> {
    let mut out = Vec::new();
    for &(prefijo, verbo, objeto, tipo, categoria, objetivos) in SERIES {
        serie(&mut out, prefijo, verbo, objeto, tipo, categoria, objetivos);
    }
    out
}
