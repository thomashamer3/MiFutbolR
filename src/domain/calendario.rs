//! Eventos del calendario mensual (IMP-02).

use jiff::civil::Date;

/// Tipo de evento del calendario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventoTipo {
    /// Partido.
    Partido,
    /// Movimiento financiero.
    Financiamiento,
    /// Recordatorio (cuando exista el módulo).
    Recordatorio,
}

impl EventoTipo {
    /// Etiqueta corta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Partido => "Partido",
            Self::Financiamiento => "Finanzas",
            Self::Recordatorio => "Recordatorio",
        }
    }

    /// Icono textual.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Partido => "⚽",
            Self::Financiamiento => "$",
            Self::Recordatorio => "!",
        }
    }
}

/// Evento con fecha absoluta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventoCalendario {
    /// Fecha.
    pub fecha: Date,
    /// Tipo.
    pub tipo: EventoTipo,
    /// Título.
    pub titulo: String,
    /// Detalle.
    pub detalle: String,
    /// Id de la entidad de origen.
    pub referencia: Option<i64>,
}

/// Estado de navegación del calendario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarioState {
    /// Año visible.
    pub year: i32,
    /// Mes visible (1-12).
    pub month: u32,
    /// Día seleccionado (1-31).
    pub dia: u32,
    /// Eventos del mes visible.
    pub eventos: Vec<EventoCalendario>,
}

impl Default for CalendarioState {
    fn default() -> Self {
        let hoy = crate::domain::dates::today();
        Self {
            year: i32::from(hoy.year()),
            month: hoy.month().unsigned_abs().into(),
            dia: hoy.day().unsigned_abs().into(),
            eventos: Vec::new(),
        }
    }
}

impl CalendarioState {
    /// Primer día del mes visible.
    #[must_use]
    pub fn primer_dia(&self) -> Date {
        crate::domain::dates::fecha(self.year, self.month, 1)
            .unwrap_or_else(|| crate::domain::dates::fecha(2000, 1, 1).unwrap_or_default())
    }

    /// Cantidad de días del mes visible.
    #[must_use]
    pub fn dias_del_mes(&self) -> u32 {
        crate::domain::dates::fecha(self.year, self.month, 1)
            .map_or(30, |fecha| fecha.days_in_month().unsigned_abs().into())
    }

    /// Avanza o retrocede un mes.
    pub fn mover_mes(&mut self, delta: i32) {
        let total = self.year * 12 + self.month as i32 - 1 + delta;
        self.year = total.div_euclid(12);
        self.month = (total.rem_euclid(12) + 1) as u32;
        self.dia = self.dia.min(self.dias_del_mes()).max(1);
    }

    /// Mueve el día seleccionado dentro del mes.
    pub fn mover_dia(&mut self, delta: i32) {
        let dia = self.dia as i32 + delta;
        self.dia = dia.clamp(1, self.dias_del_mes() as i32) as u32;
    }

    /// Fecha seleccionada.
    #[must_use]
    pub fn seleccionada(&self) -> Date {
        crate::domain::dates::fecha(self.year, self.month, self.dia)
            .unwrap_or_else(|| self.primer_dia())
    }

    /// Eventos del día seleccionado.
    #[must_use]
    pub fn eventos_del_dia(&self) -> Vec<&EventoCalendario> {
        let fecha = self.seleccionada();
        self.eventos
            .iter()
            .filter(|evento| evento.fecha == fecha)
            .collect()
    }
}
