//! Datos del panel "Hoy" (IMP-01).

use chrono::NaiveDateTime;

use crate::domain::logros::LogroProgreso;
use crate::domain::models::{RachaTipo, Resultado};

/// Partido resumido para el panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartidoBreve {
    /// Identificador.
    pub id: i64,
    /// Fecha y hora.
    pub fecha_hora: NaiveDateTime,
    /// Rival.
    pub rival: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Resultado.
    pub resultado: Resultado,
}

/// Totales del mes en curso.
#[derive(Debug, Clone, PartialEq)]
pub struct MesResumen {
    /// Clave `YYYY-MM`.
    pub mes: String,
    /// Partidos jugados.
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
    pub rendimiento_promedio: f64,
}

impl Default for MesResumen {
    fn default() -> Self {
        Self {
            mes: String::new(),
            partidos: 0,
            goles: 0,
            asistencias: 0,
            victorias: 0,
            empates: 0,
            derrotas: 0,
            rendimiento_promedio: 0.0,
        }
    }
}

/// Racha actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RachaInfo {
    /// Tipo de racha.
    pub tipo: RachaTipo,
    /// Cantidad de partidos.
    pub cantidad: i64,
}

impl Default for RachaInfo {
    fn default() -> Self {
        Self {
            tipo: RachaTipo::Neutral,
            cantidad: 0,
        }
    }
}

/// Resumen completo del panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InicioResumen {
    /// Próximo partido.
    pub proximo: Option<PartidoBreve>,
    /// Último partido.
    pub ultimo: Option<PartidoBreve>,
    /// Racha actual.
    pub racha: RachaInfo,
    /// Totales del mes.
    pub mes: MesResumen,
    /// Lesiones activas.
    pub lesiones_activas: i64,
    /// Alerta de presupuesto, si existe.
    pub presupuesto_alerta: Option<String>,
    /// Logro más cercano de completar.
    pub logro_cercano: Option<LogroProgreso>,
}
