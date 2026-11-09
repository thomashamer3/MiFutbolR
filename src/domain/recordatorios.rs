//! Recordatorios y agenda (IMP-18).

use chrono::NaiveDate;

/// Periodicidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Periodicidad {
    /// Una vez.
    #[default]
    UnaVez,
    /// Diaria.
    Diaria,
    /// Semanal.
    Semanal,
    /// Mensual.
    Mensual,
}

impl Periodicidad {
    /// Todas.
    pub const ALL: [Self; 4] = [Self::UnaVez, Self::Diaria, Self::Semanal, Self::Mensual];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UnaVez => "Una vez",
            Self::Diaria => "Diaria",
            Self::Semanal => "Semanal",
            Self::Mensual => "Mensual",
        }
    }

    /// Código persistido.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        match self {
            Self::UnaVez => 0,
            Self::Diaria => 1,
            Self::Semanal => 2,
            Self::Mensual => 3,
        }
    }

    /// Desde el código.
    #[must_use]
    pub const fn from_i64(valor: i64) -> Self {
        match valor {
            1 => Self::Diaria,
            2 => Self::Semanal,
            3 => Self::Mensual,
            _ => Self::UnaVez,
        }
    }
}

/// Recordatorio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recordatorio {
    /// Id.
    pub id: i64,
    /// Título.
    pub titulo: String,
    /// Nota.
    pub nota: String,
    /// Fecha.
    pub fecha: NaiveDate,
    /// Hora en formato HH:MM.
    pub hora: String,
    /// Periodicidad.
    pub periodicidad: Periodicidad,
    /// Temática.
    pub tematica: String,
    /// ¿Completado?
    pub completado: bool,
    /// Última generación.
    pub ultima_gen: String,
}

/// Evento de la agenda (recordatorio o partido).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoAgenda {
    /// Fecha.
    pub fecha: NaiveDate,
    /// Hora.
    pub hora: String,
    /// Título.
    pub titulo: String,
    /// Tipo.
    pub tipo: &'static str,
}
