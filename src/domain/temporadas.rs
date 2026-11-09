//! Temporadas, fases y resumen (IMP-12).

use chrono::NaiveDate;

/// Temporada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Temporada {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Año.
    pub anio: i64,
    /// Inicio.
    pub fecha_inicio: NaiveDate,
    /// Fin.
    pub fecha_fin: NaiveDate,
    /// Estado.
    pub estado: String,
    /// Descripción.
    pub descripcion: String,
}

/// Fase de la temporada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporadaFase {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Tipo (`pretemporada`, `regular`, `postemporada`).
    pub tipo: String,
    /// Inicio.
    pub fecha_inicio: String,
    /// Fin.
    pub fecha_fin: String,
}

/// Totales de una temporada.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TemporadaResumen {
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
    /// Winrate.
    pub winrate: f64,
    /// Lesiones en el rango.
    pub lesiones: i64,
}

/// Ficha completa.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporadaFicha {
    /// Temporada.
    pub temporada: Temporada,
    /// Fases.
    pub fases: Vec<TemporadaFase>,
    /// Nombres de torneos asociados.
    pub torneos: Vec<String>,
    /// Resumen.
    pub resumen: TemporadaResumen,
    /// Fatiga (0-100).
    pub fatiga: f64,
}

/// Comparación de dos temporadas.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparacionTemporadas {
    /// Nombre de A.
    pub a: String,
    /// Nombre de B.
    pub b: String,
    /// Filas (métrica, A, B).
    pub filas: Vec<(String, String, String)>,
}
