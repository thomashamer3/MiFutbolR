//! Estado y navegación de la aplicación (ADR-05).
//!
//! `App` es la máquina de pantallas: pila de navegación, modales, formularios
//! y preferencias. El dibujo vive en `ui`; aquí no se renderiza nada.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::widgets::{ListState, TableState};

use crate::config::AppPaths;
use crate::db::Database;
use crate::domain::models::{
    AppSettings, Botin, BotinFilter, BotinRow, Camiseta, CamisetaFilter, CamisetaRow, Cancha,
    CanchaFilter, CanchaRow, CarreraHall, CarreraIdentidad, CarreraPerfil, CarreraResumen,
    CategoriaTotal, Equipo, EquipoFilter, EquipoRow, EstadisticaKind, Estadisticas, EstadoFisico,
    FichaCamiseta, FinanciamientoFilter, FinanzasResumen, FormMode, HallPartido, HitoRow, Lesion,
    LesionFilter, LesionRow, LesionStats, Partido, PartidoFilter, PartidoResumen, PartidoRow,
    PresupuestoVista, TimelineEvento, TransaccionFinanciera,
};
use crate::domain::resumen::{DatosResumen, ResumenFormato};
use crate::domain::validation::{self, DomainError};
use crate::domain::{Cents, dates};
use crate::services::botin_service;
use crate::services::camiseta_service::{self, ResultadoEliminacion};

/// Nombre visible de la aplicación.
pub const APP_NAME: &str = "MiFutbolR";

/// Módulos de la especificación (§1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    /// Módulo 1.
    Camisetas,
    /// Módulo 2.
    Canchas,
    /// Módulo 3.
    Botines,
    /// Módulo 4.
    Equipos,
    /// Módulo 5.
    Partidos,
    /// Módulo 6.
    Lesiones,
    /// Módulo 7.
    Estadisticas,
    /// Módulo 8.
    Financiamiento,
    /// Módulo 9.
    Carrera,
    /// Módulo 10.
    Resumen,
    /// Módulo 11.
    Ajustes,
}

impl Module {
    /// Orden del menú principal.
    pub const ALL: [Self; 11] = [
        Self::Camisetas,
        Self::Canchas,
        Self::Botines,
        Self::Equipos,
        Self::Partidos,
        Self::Lesiones,
        Self::Estadisticas,
        Self::Financiamiento,
        Self::Carrera,
        Self::Resumen,
        Self::Ajustes,
    ];

    /// Etiqueta del menú.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Camisetas => "Camisetas",
            Self::Canchas => "Canchas",
            Self::Botines => "Botines",
            Self::Equipos => "Equipos",
            Self::Partidos => "Partidos",
            Self::Lesiones => "Lesiones",
            Self::Estadisticas => "Estadísticas",
            Self::Financiamiento => "Financiamiento",
            Self::Carrera => "Carrera futbolística",
            Self::Resumen => "Resumen compartible",
            Self::Ajustes => "Ajustes",
        }
    }

    /// Clave legacy usada en `custom_menus`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Camisetas => "camisetas",
            Self::Canchas => "canchas",
            Self::Botines => "botines",
            Self::Equipos => "equipos",
            Self::Partidos => "partidos",
            Self::Lesiones => "lesiones",
            Self::Estadisticas => "estadisticas",
            Self::Financiamiento => "financiamiento",
            Self::Carrera => "carrera",
            Self::Resumen => "resumen",
            Self::Ajustes => "ajustes",
        }
    }

    /// Descripción breve para el menú.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Camisetas => "Catálogo de camisetas, costo, estado y sorteo.",
            Self::Canchas => "Canchas, superficies, horarios, precios y servicios.",
            Self::Botines => "Botines disponibles, predeterminado y sorteo.",
            Self::Equipos => "Equipos fijos y momentáneos con sus jugadores.",
            Self::Partidos => "Registro de partidos, wizard, filtros y derivados.",
            Self::Lesiones => "Lesiones por jugador, estado y partidos asociados.",
            Self::Estadisticas => "Métricas generales, rendimiento, canchas y goles.",
            Self::Financiamiento => "Transacciones, presupuestos y alertas de gasto.",
            Self::Carrera => "Identidad, hitos, timeline y badges de carrera.",
            Self::Resumen => "Resumen compartible en TXT/CSV/JSON/HTML/MD.",
            Self::Ajustes => "Tema, idioma, modo, backup e integridad.",
        }
    }
}

/// Pantalla activa de la pila de navegación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Menú principal.
    MainMenu,
    /// Módulo aún no implementado.
    Module(Module),
    /// Listado de camisetas.
    CamisetasList,
    /// Detalle de una camiseta.
    CamisetaDetail(i64),
    /// Formulario de camiseta.
    CamisetaForm(FormMode),
    /// Listado de canchas.
    CanchasList,
    /// Detalle de una cancha.
    CanchaDetail(i64),
    /// Formulario de cancha.
    CanchaForm(FormMode),
    /// Listado de botines.
    BotinesList,
    /// Detalle de un botín.
    BotinDetail(i64),
    /// Formulario de botín.
    BotinForm(FormMode),
    /// Listado de equipos.
    EquiposList,
    /// Detalle de un equipo.
    EquipoDetail(i64),
    /// Formulario de equipo.
    EquipoForm(FormMode),
    /// Formulario de jugador.
    JugadorForm(FormMode),
    /// Listado de partidos.
    PartidosList,
    /// Detalle de un partido.
    PartidoDetail(i64),
    /// Formulario de partido.
    PartidoForm(FormMode),
    /// Asistente por pasos para crear un partido.
    PartidoWizard,
    /// Listado de lesiones.
    LesionesList,
    /// Detalle de una lesión.
    LesionDetail(i64),
    /// Formulario de lesión.
    LesionForm(FormMode),
    /// Hub de estadísticas.
    EstadisticasHub,
    /// Detalle de una categoría de estadísticas.
    EstadisticasDetalle(EstadisticaKind),
    /// Listado de financiamiento.
    FinanciamientoList,
    /// Formulario de transacción.
    TransaccionForm(FormMode),
    /// Formulario de presupuesto mensual.
    PresupuestoForm,
    /// Menú del módulo Carrera.
    CarreraMenu,
    /// Resumen de carrera.
    CarreraResumen,
    /// Ficha de identidad.
    CarreraIdentidadForm,
    /// Hitos de carrera.
    CarreraHitos,
    /// Formulario de hito.
    CarreraHitoForm(FormMode),
    /// Línea de tiempo de carrera.
    CarreraTimeline,
    /// Hall of Fame personal.
    CarreraHallOfFame,
    /// Perfil dinámico.
    CarreraPerfil,
    /// Modo retro.
    CarreraRetro,
    /// Logros de carrera.
    CarreraLogros,
    /// Vista previa del resumen compartible.
    ResumenPreview,
    /// Formulario de ajustes.
    AjustesForm,
    /// Formulario de importación legacy.
    LegacyImportForm,
    /// Ayuda contextual.
    Ayuda,
}

/// Acción confirmable en un modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    /// Eliminar la camiseta indicada.
    DeleteCamiseta(i64),
    /// Descartar el formulario con cambios.
    DiscardForm,
    /// Eliminar la cancha indicada.
    DeleteCancha(i64),
    /// Eliminar el botín indicado.
    DeleteBotin(i64),
    /// Reiniciar el sorteo de camisetas.
    ResetSorteo,
    /// Eliminar el equipo indicado.
    DeleteEquipo(i64),
    /// Eliminar el jugador indicado.
    DeleteJugador(i64),
    /// Eliminar el partido indicado.
    DeletePartido(i64),
    /// Eliminar la lesión indicada.
    DeleteLesion(i64),
    /// Eliminar la transacción indicada.
    DeleteTransaccion(i64),
    /// Eliminar el hito indicado.
    DeleteHito(i64),
    /// Restaurar el backup más reciente.
    RestoreBackup,
    /// Reiniciar el sorteo de botines.
    ResetSorteoBotines,
    /// Salir de la aplicación.
    Quit,
}

/// Modal activo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    /// Confirmación Sí/No.
    Confirm {
        /// Título.
        title: String,
        /// Mensaje.
        message: String,
        /// Acción al confirmar.
        action: ConfirmAction,
    },
    /// Error no bloqueante.
    Error(String),
    /// Información.
    Info(String),
}

impl Modal {
    /// Construye una confirmación.
    #[must_use]
    pub fn confirm(
        title: impl Into<String>,
        message: impl Into<String>,
        action: ConfirmAction,
    ) -> Self {
        Self::Confirm {
            title: title.into(),
            message: message.into(),
            action,
        }
    }
}

/// Tipo de campo de un formulario (§19.5).
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    /// Texto de una línea.
    Text {
        /// Longitud máxima en caracteres.
        max: usize,
    },
    /// Texto multilínea.
    Multiline {
        /// Longitud máxima.
        max: usize,
    },
    /// Entero en rango.
    Int {
        /// Mínimo.
        min: i64,
        /// Máximo.
        max: i64,
    },
    /// Decimal en rango.
    Decimal {
        /// Mínimo.
        min: f64,
        /// Máximo.
        max: f64,
    },
    /// Importe monetario (se guarda en centavos).
    Money,
    /// Fecha `DD/MM/YYYY`.
    Date,
    /// Hora `HH:MM`.
    Time,
    /// Fecha y hora.
    DateTime,
    /// Booleano Sí/No.
    Bool,
    /// Selección simple: `(valor, etiqueta)`.
    Select {
        /// Opciones.
        options: Vec<(String, String)>,
    },
    /// Selección múltiple.
    MultiSelect {
        /// Opciones.
        options: Vec<(String, String)>,
        /// Valores marcados.
        selected: Vec<String>,
    },
}

/// Campo de un formulario.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Clave estable del campo.
    pub key: &'static str,
    /// Etiqueta visible.
    pub label: String,
    /// Valor en texto.
    pub value: String,
    /// Tipo.
    pub kind: FieldKind,
    /// ¿Obligatorio?
    pub required: bool,
    /// Ayuda contextual.
    pub help: Option<String>,
}

impl Field {
    /// Crea un campo de texto.
    #[must_use]
    pub fn text(key: &'static str, label: &str, value: impl Into<String>, max: usize) -> Self {
        Self {
            key,
            label: label.to_string(),
            value: value.into(),
            kind: FieldKind::Text { max },
            required: false,
            help: None,
        }
    }

    /// Crea un campo select.
    #[must_use]
    pub fn select(
        key: &'static str,
        label: &str,
        value: impl Into<String>,
        options: Vec<(String, String)>,
    ) -> Self {
        Self {
            key,
            label: label.to_string(),
            value: value.into(),
            kind: FieldKind::Select { options },
            required: false,
            help: None,
        }
    }

    /// Crea un campo booleano.
    #[must_use]
    pub fn boolean(key: &'static str, label: &str, value: bool) -> Self {
        Self {
            key,
            label: label.to_string(),
            value: if value { "Sí" } else { "No" }.to_string(),
            kind: FieldKind::Bool,
            required: false,
            help: None,
        }
    }

    /// Marca el campo como obligatorio.
    #[must_use]
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Texto a mostrar como valor (las opciones muestran su etiqueta).
    #[must_use]
    pub fn display_value(&self) -> String {
        match &self.kind {
            FieldKind::Select { options } | FieldKind::MultiSelect { options, .. } => options
                .iter()
                .find(|(value, _)| value == &self.value)
                .map_or_else(|| self.value.clone(), |(_, label)| label.clone()),
            _ => self.value.clone(),
        }
    }

    /// `true` si el campo acepta escritura directa.
    #[must_use]
    pub const fn is_text_entry(&self) -> bool {
        matches!(
            self.kind,
            FieldKind::Text { .. }
                | FieldKind::Multiline { .. }
                | FieldKind::Int { .. }
                | FieldKind::Decimal { .. }
                | FieldKind::Money
                | FieldKind::Date
                | FieldKind::Time
                | FieldKind::DateTime
        )
    }

    /// Alterna un booleano o avanza un select.
    pub fn toggle(&mut self) {
        let current_value = self.value.clone();
        match &mut self.kind {
            FieldKind::Bool => {
                self.value = if current_value == "Sí" { "No" } else { "Sí" }.to_string();
            }
            FieldKind::Select { options } => {
                if options.is_empty() {
                    return;
                }
                let current = options
                    .iter()
                    .position(|(value, _)| value == &current_value)
                    .unwrap_or(0);
                let next = (current + 1) % options.len();
                self.value = options[next].0.clone();
            }
            FieldKind::MultiSelect { selected, .. } => {
                if selected.contains(&current_value) {
                    selected.retain(|value| value != &current_value);
                } else if !current_value.is_empty() {
                    selected.push(current_value);
                }
            }
            _ => {}
        }
    }

    /// Avanza/retrocede un select.
    pub fn cycle(&mut self, options: &[(String, String)]) {
        if options.is_empty() {
            return;
        }
        let current = options.iter().position(|(value, _)| value == &self.value);
        let next = match (current, options.len()) {
            (Some(index), len) => (index + 1) % len,
            (None, _) => 0,
        };
        self.value = options[next].0.clone();
    }

    /// Valida el campo y devuelve el mensaje de error.
    pub fn validate(&self) -> Result<(), String> {
        let trimmed = self.value.trim();
        if self.required && trimmed.is_empty() {
            return Err("Este campo es obligatorio.".to_string());
        }
        match &self.kind {
            FieldKind::Text { max } | FieldKind::Multiline { max } => {
                if self.value.chars().count() > *max {
                    return Err(format!("Máximo {max} caracteres."));
                }
            }
            FieldKind::Int { min, max } => {
                if !trimmed.is_empty() {
                    let value: i64 = trimmed
                        .parse()
                        .map_err(|_| "Debe ser un número entero.".to_string())?;
                    if value < *min || value > *max {
                        return Err(format!("Debe estar entre {min} y {max}."));
                    }
                }
            }
            FieldKind::Decimal { min, max } => {
                if !trimmed.is_empty() {
                    let value: f64 = trimmed
                        .parse()
                        .map_err(|_| "Debe ser un número.".to_string())?;
                    if !value.is_finite() || value < *min || value > *max {
                        return Err(format!("Debe estar entre {min} y {max}."));
                    }
                }
            }
            FieldKind::Money => {
                Cents::parse_user_input(&self.value).map_err(|error| error.to_string())?;
            }
            FieldKind::Date => {
                if !trimmed.is_empty() && dates::parse_date(trimmed).is_none() {
                    return Err("Fecha inválida (use DD/MM/AAAA).".to_string());
                }
            }
            FieldKind::Time => {
                if !trimmed.is_empty() && dates::parse_time(trimmed).is_none() {
                    return Err("Hora inválida (use HH:MM).".to_string());
                }
            }
            FieldKind::DateTime => {
                if !trimmed.is_empty() && dates::parse_datetime(trimmed).is_none() {
                    return Err("Fecha y hora inválidas.".to_string());
                }
            }
            FieldKind::Bool => {
                if !matches!(self.value.as_str(), "Sí" | "No") {
                    return Err("Valor booleano inválido.".to_string());
                }
            }
            FieldKind::Select { options } => {
                if !options.iter().any(|(value, _)| value == &self.value) {
                    return Err("Seleccione una opción válida.".to_string());
                }
            }
            FieldKind::MultiSelect { .. } => {}
        }
        Ok(())
    }

    /// Agrega un carácter respetando el máximo.
    pub fn push_char(&mut self, ch: char) {
        let max = match self.kind {
            FieldKind::Text { max } | FieldKind::Multiline { max } => max,
            _ => usize::MAX,
        };
        if self.value.chars().count() < max {
            self.value.push(ch);
        }
    }

    /// Borra el último carácter.
    pub fn pop_char(&mut self) {
        self.value.pop();
    }
}

/// Formulario activo (§19.5).
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// Título.
    pub title: String,
    /// Campos.
    pub fields: Vec<Field>,
    /// Campo con foco.
    pub focus: usize,
    /// Error de validación.
    pub error: Option<String>,
    /// ¿Tiene cambios sin guardar?
    pub dirty: bool,
}

impl Form {
    /// Crea un formulario.
    #[must_use]
    pub fn new(title: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            title: title.into(),
            fields,
            focus: 0,
            error: None,
            dirty: false,
        }
    }

    /// Campo con foco.
    #[must_use]
    pub fn current(&self) -> Option<&Field> {
        self.fields.get(self.focus)
    }

    /// Valor de un campo por clave.
    #[must_use]
    pub fn value(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.value.as_str())
    }

    /// Siguiente campo.
    pub fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + 1) % self.fields.len();
        }
    }

    /// Campo anterior.
    pub fn previous_field(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
        }
    }

    /// Valida todos los campos y enfoca el primero inválido.
    pub fn validate(&mut self) -> Result<(), (usize, String)> {
        for (index, field) in self.fields.iter().enumerate() {
            if let Err(message) = field.validate() {
                self.error = Some(message.clone());
                self.focus = index;
                return Err((index, message));
            }
        }
        self.error = None;
        Ok(())
    }

    /// Escribe un carácter en el campo activo.
    pub fn push_char(&mut self, ch: char) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.is_text_entry() {
                field.push_char(ch);
                self.dirty = true;
            }
        }
    }

    /// Borra el último carácter del campo activo.
    pub fn pop_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            if field.is_text_entry() && !field.value.is_empty() {
                field.pop_char();
                self.dirty = true;
            }
        }
    }

    /// Alterna el campo activo (bool/select).
    pub fn toggle_current(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus) {
            field.toggle();
            self.dirty = true;
        }
    }

    /// `true` si el campo activo se alterna con Espacio/Enter.
    #[must_use]
    pub fn current_is_toggle(&self) -> bool {
        matches!(
            self.current().map(|field| &field.kind),
            Some(FieldKind::Bool | FieldKind::Select { .. } | FieldKind::MultiSelect { .. })
        )
    }

    /// `true` si el campo activo es una selección simple.
    #[must_use]
    pub fn current_is_select(&self) -> bool {
        matches!(
            self.current().map(|field| &field.kind),
            Some(FieldKind::Select { .. })
        )
    }

    /// Avanza o retrocede la opción del campo activo.
    pub fn cycle_current(&mut self, forward: bool) {
        let Some(field) = self.fields.get_mut(self.focus) else {
            return;
        };
        match &field.kind {
            FieldKind::Bool => {
                field.value = if field.value == "Sí" { "No" } else { "Sí" }.to_string();
            }
            FieldKind::Select { options } => {
                if options.is_empty() {
                    return;
                }
                let current = options
                    .iter()
                    .position(|(value, _)| value == &field.value)
                    .unwrap_or(0);
                let next = if forward {
                    (current + 1) % options.len()
                } else {
                    (current + options.len() - 1) % options.len()
                };
                field.value = options[next].0.clone();
            }
            _ => {}
        }
        self.dirty = true;
    }
}

/// Pantalla destino de la búsqueda incremental.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    /// Camisetas.
    Camisetas,
    /// Canchas.
    Canchas,
    /// Botines.
    Botines,
    /// Equipos.
    Equipos,
    /// Partidos.
    Partidos,
    /// Lesiones.
    Lesiones,
    /// Financiamiento.
    Financiamiento,
}

/// Estado de la búsqueda incremental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchState {
    /// Listado que se filtra.
    pub target: SearchTarget,
    /// Texto escrito.
    pub buffer: String,
}

/// Estado global de la aplicación.
pub struct App {
    /// Base de datos.
    pub db: Database,
    /// Rutas.
    pub paths: AppPaths,
    /// Pila de pantallas.
    pub stack: Vec<Screen>,
    /// Modal activo.
    pub modal: Option<Modal>,
    /// Mensaje de estado.
    pub status: String,
    /// Salir.
    pub should_quit: bool,
    /// Preferencias.
    pub settings: AppSettings,
    /// Menú principal.
    pub menu_state: ListState,
    /// Filas del listado de camisetas.
    pub camisetas: Vec<CamisetaRow>,
    /// Selección del listado.
    pub camisetas_table: TableState,
    /// Filtro activo.
    pub camiseta_filter: CamisetaFilter,
    /// Detalle cargado.
    pub camiseta_detail: Option<Camiseta>,
    /// Estadísticas derivadas del detalle.
    pub camiseta_ficha: Option<FichaCamiseta>,
    /// Formulario activo.
    pub form: Option<Form>,
    /// Camiseta original del formulario (edición).
    pub form_original: Option<Camiseta>,
    /// Filas del listado de canchas.
    pub canchas: Vec<CanchaRow>,
    /// Selección del listado de canchas.
    pub canchas_table: TableState,
    /// Filtro de canchas.
    pub cancha_filter: CanchaFilter,
    /// Detalle de cancha cargado.
    pub cancha_detail: Option<Cancha>,
    /// Cancha original del formulario (edición).
    pub form_original_cancha: Option<Cancha>,
    /// Filas del listado de botines.
    pub botines: Vec<BotinRow>,
    /// Selección del listado de botines.
    pub botines_table: TableState,
    /// Filtro de botines.
    pub botin_filter: BotinFilter,
    /// Detalle de botín cargado.
    pub botin_detail: Option<Botin>,
    /// Botín original del formulario (edición).
    pub form_original_botin: Option<Botin>,
    /// Filas del listado de equipos.
    pub equipos: Vec<EquipoRow>,
    /// Selección del listado de equipos.
    pub equipos_table: TableState,
    /// Filtro de equipos.
    pub equipo_filter: EquipoFilter,
    /// Equipo detallado.
    pub equipo_detail: Option<Equipo>,
    /// Equipo original del formulario.
    pub form_original_equipo: Option<Equipo>,
    /// Selección de jugadores en el detalle.
    pub jugadores_table: TableState,
    /// Id del jugador en edición (0 = alta).
    pub jugador_edit_id: Option<i64>,
    /// Filas del listado de partidos.
    pub partidos: Vec<PartidoRow>,
    /// Selección del listado de partidos.
    pub partidos_table: TableState,
    /// Filtro de partidos.
    pub partido_filter: PartidoFilter,
    /// Página actual (1-based).
    pub partido_page: i64,
    /// Tamaño de página.
    pub partido_page_size: i64,
    /// Total de partidos del filtro.
    pub partido_total: i64,
    /// Resumen agregado del filtro.
    pub partido_resumen: PartidoResumen,
    /// Detalle de partido cargado.
    pub partido_detail: Option<Partido>,
    /// Partido original del formulario.
    pub form_original_partido: Option<Partido>,
    /// Paso actual del asistente de partidos (0..=3).
    pub partido_wizard_step: usize,
    /// Filas del listado de lesiones.
    pub lesiones: Vec<LesionRow>,
    /// Selección del listado de lesiones.
    pub lesiones_table: TableState,
    /// Filtro de lesiones.
    pub lesion_filter: LesionFilter,
    /// Estadísticas de lesiones.
    pub lesion_stats: LesionStats,
    /// Detalle de lesión cargado.
    pub lesion_detail: Option<Lesion>,
    /// Lesión original del formulario.
    pub form_original_lesion: Option<Lesion>,
    /// Estadísticas cargadas.
    pub estadisticas: Option<Estadisticas>,
    /// Selección del hub.
    pub estadisticas_hub: ListState,
    /// Desplazamiento del detalle.
    pub estadisticas_scroll: u16,
    /// Transacciones del filtro.
    pub transacciones: Vec<TransaccionFinanciera>,
    /// Selección del listado.
    pub transacciones_table: TableState,
    /// Filtro financiero.
    pub finanzas_filter: FinanciamientoFilter,
    /// Resumen del filtro.
    pub finanzas_resumen: FinanzasResumen,
    /// Totales por categoría.
    pub finanzas_categorias: Vec<CategoriaTotal>,
    /// Presupuesto del mes actual.
    pub presupuesto: Option<PresupuestoVista>,
    /// Mes de trabajo `YYYY-MM`.
    pub finanzas_mes: String,
    /// Transacción original del formulario.
    pub form_original_transaccion: Option<TransaccionFinanciera>,
    /// Hub de carrera.
    pub carrera_hub: ListState,
    /// Resumen de carrera.
    pub carrera_resumen: Option<CarreraResumen>,
    /// Identidad cargada.
    pub carrera_identidad: Option<CarreraIdentidad>,
    /// Hitos.
    pub carrera_hitos: Vec<HitoRow>,
    /// Selección de hitos.
    pub carrera_hitos_table: TableState,
    /// Hito en edición.
    pub carrera_hito_edit: Option<HitoRow>,
    /// Timeline.
    pub carrera_timeline: Vec<TimelineEvento>,
    /// Hall of Fame.
    pub carrera_hall: Option<CarreraHall>,
    /// Perfil dinámico.
    pub carrera_perfil: Option<CarreraPerfil>,
    /// Recuerdos del modo retro.
    pub carrera_retro: Vec<HallPartido>,
    /// Desplazamiento de pantallas de texto.
    pub carrera_scroll: u16,
    /// Datos del resumen generado.
    pub resumen_datos: Option<DatosResumen>,
    /// Formato activo del resumen.
    pub resumen_formato: ResumenFormato,
    /// Texto renderizado.
    pub resumen_texto: String,
    /// Desplazamiento de la vista previa.
    pub resumen_scroll: u16,
    /// Búsqueda activa.
    pub search: Option<SearchState>,
}

impl App {
    /// Crea el estado inicial.
    pub fn new(db: Database, paths: AppPaths) -> Result<Self> {
        let settings = db.settings()?;
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));
        let status = match db.path() {
            Some(path) => format!("BD: {}", path.display()),
            None => "BD: en memoria".to_string(),
        };
        Ok(Self {
            db,
            paths,
            stack: vec![Screen::MainMenu],
            modal: None,
            status,
            should_quit: false,
            settings,
            menu_state,
            camisetas: Vec::new(),
            camisetas_table: TableState::default(),
            camiseta_filter: CamisetaFilter::default(),
            camiseta_detail: None,
            camiseta_ficha: None,
            canchas: Vec::new(),
            canchas_table: TableState::default(),
            cancha_filter: CanchaFilter::default(),
            cancha_detail: None,
            form_original_cancha: None,
            botines: Vec::new(),
            botines_table: TableState::default(),
            botin_filter: BotinFilter::default(),
            botin_detail: None,
            form_original_botin: None,
            equipos: Vec::new(),
            equipos_table: TableState::default(),
            equipo_filter: EquipoFilter::default(),
            equipo_detail: None,
            form_original_equipo: None,
            jugadores_table: TableState::default(),
            jugador_edit_id: None,
            partidos: Vec::new(),
            partidos_table: TableState::default(),
            partido_filter: PartidoFilter::default(),
            partido_page: 1,
            partido_page_size: 20,
            partido_total: 0,
            partido_resumen: PartidoResumen::default(),
            partido_detail: None,
            form_original_partido: None,
            partido_wizard_step: 0,
            lesiones: Vec::new(),
            lesiones_table: TableState::default(),
            lesion_filter: LesionFilter::default(),
            lesion_stats: LesionStats::default(),
            lesion_detail: None,
            form_original_lesion: None,
            estadisticas: None,
            estadisticas_hub: ListState::default(),
            estadisticas_scroll: 0,
            transacciones: Vec::new(),
            transacciones_table: TableState::default(),
            finanzas_filter: FinanciamientoFilter::default(),
            finanzas_resumen: FinanzasResumen::default(),
            finanzas_categorias: Vec::new(),
            presupuesto: None,
            finanzas_mes: String::new(),
            form_original_transaccion: None,
            carrera_hub: ListState::default(),
            carrera_resumen: None,
            carrera_identidad: None,
            carrera_hitos: Vec::new(),
            carrera_hitos_table: TableState::default(),
            carrera_hito_edit: None,
            carrera_timeline: Vec::new(),
            carrera_hall: None,
            carrera_perfil: None,
            carrera_retro: Vec::new(),
            carrera_scroll: 0,
            resumen_datos: None,
            resumen_formato: ResumenFormato::Markdown,
            resumen_texto: String::new(),
            resumen_scroll: 0,
            form: None,
            form_original: None,
            search: None,
        })
    }

    /// Pantalla visible.
    #[must_use]
    pub fn current_screen(&self) -> Screen {
        self.stack.last().copied().unwrap_or(Screen::MainMenu)
    }

    /// Título de la pantalla.
    #[must_use]
    pub fn screen_title(&self) -> String {
        match self.current_screen() {
            Screen::MainMenu => "Menú principal".to_string(),
            Screen::Module(module) => module.label().to_string(),
            Screen::CamisetasList => "Camisetas".to_string(),
            Screen::CamisetaDetail(_) => "Camisetas · detalle".to_string(),
            Screen::CamisetaForm(_) => "Camisetas · formulario".to_string(),
            Screen::CanchasList => "Canchas".to_string(),
            Screen::CanchaDetail(_) => "Canchas · detalle".to_string(),
            Screen::CanchaForm(_) => "Canchas · formulario".to_string(),
            Screen::BotinesList => "Botines".to_string(),
            Screen::BotinDetail(_) => "Botines · detalle".to_string(),
            Screen::BotinForm(_) => "Botines · formulario".to_string(),
            Screen::EquiposList => "Equipos".to_string(),
            Screen::EquipoDetail(_) => "Equipos · detalle".to_string(),
            Screen::EquipoForm(_) => "Equipos · formulario".to_string(),
            Screen::JugadorForm(_) => "Equipos · jugador".to_string(),
            Screen::PartidosList => "Partidos".to_string(),
            Screen::PartidoDetail(_) => "Partidos · detalle".to_string(),
            Screen::PartidoForm(_) => "Partidos · formulario".to_string(),
            Screen::PartidoWizard => {
                format!("Partidos · asistente ({}/4)", self.partido_wizard_step + 1)
            }
            Screen::LesionesList => "Lesiones".to_string(),
            Screen::LesionDetail(_) => "Lesiones · detalle".to_string(),
            Screen::LesionForm(_) => "Lesiones · formulario".to_string(),
            Screen::EstadisticasHub => "Estadísticas".to_string(),
            Screen::EstadisticasDetalle(kind) => format!("Estadísticas · {}", kind.label()),
            Screen::FinanciamientoList => "Financiamiento".to_string(),
            Screen::TransaccionForm(_) => "Financiamiento · transacción".to_string(),
            Screen::PresupuestoForm => "Financiamiento · presupuesto".to_string(),
            Screen::CarreraMenu => "Carrera".to_string(),
            Screen::CarreraResumen => "Carrera · resumen".to_string(),
            Screen::CarreraIdentidadForm => "Carrera · identidad".to_string(),
            Screen::CarreraHitos => "Carrera · hitos".to_string(),
            Screen::CarreraHitoForm(_) => "Carrera · hito".to_string(),
            Screen::CarreraTimeline => "Carrera · timeline".to_string(),
            Screen::CarreraHallOfFame => "Carrera · hall of fame".to_string(),
            Screen::CarreraPerfil => "Carrera · perfil".to_string(),
            Screen::CarreraRetro => "Carrera · retro".to_string(),
            Screen::CarreraLogros => "Carrera · logros".to_string(),
            Screen::ResumenPreview => "Resumen compartible".to_string(),
            Screen::AjustesForm => "Ajustes".to_string(),
            Screen::LegacyImportForm => "Ajustes · importar legacy".to_string(),
            Screen::Ayuda => "Ayuda".to_string(),
        }
    }

    /// Atajos a mostrar en la barra inferior según la pantalla.
    #[must_use]
    pub fn footer_hints(&self) -> &'static str {
        if let Some(search) = &self.search {
            return if search.buffer.is_empty() {
                "Buscar…  Enter aplicar  Esc cancelar"
            } else {
                "Enter aplicar  Esc cancelar"
            };
        }
        match self.current_screen() {
            Screen::MainMenu => "↑↓/jk: mover   Enter: abrir   q: salir   ?: ayuda",
            Screen::CamisetasList => {
                "n: nueva  e: editar  d: borrar  Enter: detalle  s: sortear  f: filtro  o: orden  /: buscar  r: refrescar"
            }
            Screen::CamisetaDetail(_) => "e: editar  d: borrar  Esc: volver",
            Screen::CamisetaForm(_)
            | Screen::CanchaForm(_)
            | Screen::BotinForm(_)
            | Screen::EquipoForm(_)
            | Screen::JugadorForm(_)
            | Screen::PartidoForm(_)
            | Screen::LesionForm(_) => "Tab: campo  Ctrl+S: guardar  Esc: cancelar",
            Screen::CanchasList => {
                "n: nueva  e: editar  d: borrar  a: activar/desactivar  Enter: detalle  o: orden  /: buscar  r: refrescar"
            }
            Screen::CanchaDetail(_) => "e: editar  d: borrar  a: activar/desactivar  Esc: volver",
            Screen::BotinesList => {
                "n: nuevo  e: editar  d: borrar  s: sortear  R: reiniciar sorteo  p: predeterminado  a: activar/desactivar  /: buscar"
            }
            Screen::BotinDetail(_) => "e: editar  d: borrar  p: predeterminado  Esc: volver",
            Screen::EquiposList => {
                "n: nuevo  e: editar  d: borrar  a: activar/desactivar  Enter: detalle  o: orden  /: buscar  r: refrescar"
            }
            Screen::EquipoDetail(_) => {
                "a: agregar jugador  e: editar jugador  d: borrar jugador  Esc: volver"
            }
            Screen::PartidosList => {
                "n: nuevo  e: editar  d: borrar  Enter: detalle  F: favorito  f: tipo  v: resultado  o: orden  PgUp/PgDn: página  /: buscar"
            }
            Screen::PartidoDetail(_) => "e: editar  d: borrar  F: favorito  Esc: volver",
            Screen::PartidoWizard => {
                "Tab: campo  Enter/PgDn: siguiente paso  PgUp: anterior  Ctrl+S: guardar  Esc: cancelar"
            }
            Screen::LesionesList => {
                "n: nueva  e: editar  d: borrar  Enter: detalle  f: estado  o: orden  /: buscar  r: refrescar"
            }
            Screen::LesionDetail(_) => "e: editar  d: borrar  Esc: volver",
            Screen::EstadisticasHub => "↑↓: categoría  Enter: abrir  r: recalcular  Esc: volver",
            Screen::EstadisticasDetalle(_) => "↑↓: desplazar  PgUp/PgDn: página  Esc: volver",
            Screen::FinanciamientoList => {
                "n: nueva  e: editar  d: borrar  p: presupuesto  f: tipo  c: categoría  /: buscar  r: refrescar"
            }
            Screen::TransaccionForm(_) | Screen::PresupuestoForm => {
                "Tab: campo  Ctrl+S: guardar  Esc: cancelar"
            }
            Screen::CarreraMenu => "↑↓: opción  Enter: abrir  Esc: volver",
            Screen::CarreraResumen
            | Screen::CarreraTimeline
            | Screen::CarreraHallOfFame
            | Screen::CarreraPerfil
            | Screen::CarreraRetro
            | Screen::CarreraLogros => "↑↓: desplazar  Esc: volver",
            Screen::CarreraIdentidadForm | Screen::CarreraHitoForm(_) => {
                "Tab: campo  Ctrl+S: guardar  Esc: cancelar"
            }
            Screen::CarreraHitos => "n: nuevo  e: editar  d: borrar  Esc: volver",
            Screen::ResumenPreview => {
                "m: Markdown  h: HTML  t: texto  g: guardar  ↑↓: desplazar  r: regenerar  Esc: volver"
            }
            Screen::AjustesForm => {
                "Tab: campo  Ctrl+S: guardar  Ctrl+B: backup  Ctrl+I: integridad  Ctrl+E: exportar BD  Ctrl+R: restaurar  Ctrl+L: importar legacy  Esc: volver"
            }
            Screen::LegacyImportForm => {
                "Ruta del archivo .db legacy  ·  Ctrl+S: importar  Esc: cancelar"
            }
            Screen::Module(_) | Screen::Ayuda => "Esc: volver",
        }
    }

    /// Módulo seleccionado en el menú.
    #[must_use]
    pub fn selected_module(&self) -> Module {
        Module::ALL
            .get(self.menu_state.selected().unwrap_or(0))
            .copied()
            .unwrap_or(Module::Camisetas)
    }

    /// Entra a una pantalla.
    pub fn push_screen(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    /// Vuelve atrás.
    pub fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.status = "Volvió a la pantalla anterior".to_string();
        }
    }

    /// Procesa una tecla.
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        if self.modal.is_some() {
            self.handle_modal_key(key);
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.save_current_form()?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(self.current_screen(), Screen::AjustesForm)
        {
            match key.code {
                KeyCode::Char('b') => {
                    self.backup_ajustes()?;
                    return Ok(());
                }
                KeyCode::Char('i') => {
                    self.integridad_ajustes()?;
                    return Ok(());
                }
                KeyCode::Char('e') => {
                    self.exportar_db_ajustes()?;
                    return Ok(());
                }
                KeyCode::Char('r') => {
                    self.confirmar_restauracion();
                    return Ok(());
                }
                KeyCode::Char('l') => {
                    self.open_legacy_import_form()?;
                    return Ok(());
                }
                _ => {}
            }
        }
        if self.search.is_some() {
            self.handle_search_key(key)?;
            return Ok(());
        }

        let consumed = match self.current_screen() {
            Screen::MainMenu => self.handle_main_menu_key(key)?,
            Screen::CamisetasList => self.handle_camisetas_list_key(key)?,
            Screen::CamisetaDetail(_) => self.handle_camiseta_detail_key(key)?,
            Screen::CamisetaForm(_)
            | Screen::CanchaForm(_)
            | Screen::BotinForm(_)
            | Screen::EquipoForm(_)
            | Screen::JugadorForm(_)
            | Screen::PartidoForm(_)
            | Screen::LesionForm(_)
            | Screen::TransaccionForm(_)
            | Screen::PresupuestoForm
            | Screen::CarreraIdentidadForm
            | Screen::CarreraHitoForm(_)
            | Screen::AjustesForm
            | Screen::LegacyImportForm => {
                self.handle_form_key(key)?;
                true
            }
            Screen::CanchasList => self.handle_canchas_list_key(key)?,
            Screen::CanchaDetail(_) => self.handle_cancha_detail_key(key)?,
            Screen::BotinesList => self.handle_botines_list_key(key)?,
            Screen::BotinDetail(_) => self.handle_botin_detail_key(key)?,
            Screen::EquiposList => self.handle_equipos_list_key(key)?,
            Screen::EquipoDetail(_) => self.handle_equipo_detail_key(key)?,
            Screen::PartidosList => self.handle_partidos_list_key(key)?,
            Screen::PartidoDetail(_) => self.handle_partido_detail_key(key)?,
            Screen::PartidoWizard => self.handle_partido_wizard_key(key)?,
            Screen::LesionesList => self.handle_lesiones_list_key(key)?,
            Screen::LesionDetail(_) => self.handle_lesion_detail_key(key)?,
            Screen::EstadisticasHub => self.handle_estadisticas_hub_key(key)?,
            Screen::EstadisticasDetalle(_) => self.handle_estadisticas_detalle_key(key)?,
            Screen::FinanciamientoList => self.handle_financiamiento_list_key(key)?,
            Screen::CarreraMenu => self.handle_carrera_menu_key(key)?,
            Screen::CarreraResumen
            | Screen::CarreraTimeline
            | Screen::CarreraHallOfFame
            | Screen::CarreraPerfil
            | Screen::CarreraRetro
            | Screen::CarreraLogros => self.handle_carrera_scroll_key(key)?,
            Screen::CarreraHitos => self.handle_carrera_hitos_key(key)?,
            Screen::ResumenPreview => self.handle_resumen_key(key)?,
            Screen::Module(_) | Screen::Ayuda => false,
        };
        if !consumed {
            self.handle_common_key(key);
        }
        Ok(())
    }

    fn handle_common_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Esc => self.go_back(),
            KeyCode::Char('?') => self.push_screen(Screen::Ayuda),
            _ => {}
        }
    }

    fn handle_main_menu_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_menu_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_menu_selection(-1),
            KeyCode::Enter => {
                let module = self.selected_module();
                self.open_module(module)?;
            }
            KeyCode::Char(digit) if digit.is_ascii_digit() => {
                let index = digit.to_digit(10).unwrap_or(0) as usize;
                if (1..=9).contains(&index) {
                    self.menu_state.select(Some(index - 1));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn handle_camisetas_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_table_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_table_selection(-1),
            KeyCode::Enter => self.open_camiseta_detail()?,
            KeyCode::Char('n') => self.open_camiseta_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_camiseta_id() {
                    self.open_camiseta_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_camiseta(),
            KeyCode::Char('s') => self.sortear_camiseta()?,
            KeyCode::Char('f') => {
                self.camiseta_filter.estado = self.camiseta_filter.estado.next();
                self.refresh_camisetas()?;
            }
            KeyCode::Char('o') => {
                self.camiseta_filter.orden = self.camiseta_filter.orden.next();
                self.refresh_camisetas()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Camisetas,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_camisetas()?;
                self.status = "Listado actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn handle_camiseta_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('e') => {
                if let Some(camiseta) = &self.camiseta_detail {
                    let id = camiseta.id;
                    self.open_camiseta_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_camiseta(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn handle_form_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            let dirty = self.form.as_ref().is_some_and(|form| form.dirty);
            if dirty {
                self.modal = Some(Modal::confirm(
                    "Descartar cambios",
                    "El formulario tiene cambios sin guardar. ¿Descartarlos? (S = sí, N = no)",
                    ConfirmAction::DiscardForm,
                ));
            } else {
                self.discard_form();
            }
            return Ok(());
        }

        let Some(form) = self.form.as_mut() else {
            return Ok(());
        };
        match key.code {
            KeyCode::Tab => form.next_field(),
            KeyCode::BackTab => form.previous_field(),
            KeyCode::Backspace => form.pop_char(),
            KeyCode::Enter => {
                if form.current_is_toggle() {
                    form.toggle_current();
                } else {
                    form.next_field();
                }
            }
            KeyCode::Left => {
                if form.current_is_select() {
                    form.cycle_current(false);
                }
            }
            KeyCode::Right => {
                if form.current_is_select() {
                    form.cycle_current(true);
                }
            }
            KeyCode::Char(' ') => {
                if form.current_is_toggle() {
                    form.toggle_current();
                } else {
                    form.push_char(' ');
                }
            }
            KeyCode::Char(ch) => form.push_char(ch),
            _ => {}
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.search = None;
                self.status = "Búsqueda cancelada".to_string();
            }
            KeyCode::Enter => {
                if let Some(search) = self.search.take() {
                    match search.target {
                        SearchTarget::Camisetas => {
                            self.camiseta_filter.texto = search.buffer;
                            self.refresh_camisetas()?;
                        }
                        SearchTarget::Canchas => {
                            self.cancha_filter.texto = search.buffer;
                            self.refresh_canchas()?;
                        }
                        SearchTarget::Botines => {
                            self.botin_filter.texto = search.buffer;
                            self.refresh_botines()?;
                        }
                        SearchTarget::Equipos => {
                            self.equipo_filter.texto = search.buffer;
                            self.refresh_equipos()?;
                        }
                        SearchTarget::Partidos => {
                            self.partido_filter.texto = search.buffer;
                            self.partido_page = 1;
                            self.refresh_partidos()?;
                        }
                        SearchTarget::Lesiones => {
                            self.lesion_filter.texto = search.buffer;
                            self.refresh_lesiones()?;
                        }
                        SearchTarget::Financiamiento => {
                            self.finanzas_filter.texto = search.buffer;
                            self.refresh_financiamiento()?;
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(search) = self.search.as_mut() {
                    search.buffer.pop();
                }
            }
            KeyCode::Char(ch) => {
                if let Some(search) = self.search.as_mut() {
                    search.buffer.push(ch);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        let cancel = matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N')
        );
        let accept = matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S')
        );
        if cancel {
            return;
        }
        if accept {
            if let Modal::Confirm { action, .. } = &modal {
                let action = *action;
                self.confirm(action);
            }
            return;
        }
        self.modal = Some(modal);
    }

    fn confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Quit => self.should_quit = true,
            ConfirmAction::DeleteCamiseta(id) => {
                if let Err(error) = self.delete_camiseta(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DiscardForm => self.discard_form(),
            ConfirmAction::DeleteCancha(id) => {
                if let Err(error) = self.delete_cancha(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteBotin(id) => {
                if let Err(error) = self.delete_botin(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteEquipo(id) => {
                if let Err(error) = self.delete_equipo(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::RestoreBackup => {
                if let Err(error) = self.restaurar_backup() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteHito(id) => {
                if let Err(error) = self.delete_carrera_hito(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteTransaccion(id) => {
                if let Err(error) = self.delete_transaccion(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteLesion(id) => {
                if let Err(error) = self.delete_lesion(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeletePartido(id) => {
                if let Err(error) = self.delete_partido(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::DeleteJugador(id) => {
                if let Err(error) = self.delete_jugador(id) {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            ConfirmAction::ResetSorteoBotines => {
                if let Err(error) = botin_service::reiniciar_sorteo(self.db.conn()) {
                    self.modal = Some(Modal::Error(error.to_string()));
                } else {
                    self.status = "Sorteo de botines reiniciado".to_string();
                }
            }
            ConfirmAction::ResetSorteo => {
                if let Err(error) = camiseta_service::reiniciar_sorteo(self.db.conn()) {
                    self.modal = Some(Modal::Error(error.to_string()));
                } else {
                    self.status = "Sorteo reiniciado".to_string();
                }
            }
        }
    }

    fn move_menu_selection(&mut self, delta: isize) {
        let len = Module::ALL.len() as isize;
        let current = self.menu_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.menu_state.select(Some(next));
    }

    fn move_table_selection(&mut self, delta: isize) {
        if self.camisetas.is_empty() {
            return;
        }
        let len = self.camisetas.len() as isize;
        let current = self
            .camisetas_table
            .selected()
            .unwrap_or(0)
            .min(self.camisetas.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.camisetas_table.select(Some(next));
    }

    fn request_quit(&mut self) {
        if self.current_screen() == Screen::MainMenu {
            self.modal = Some(Modal::confirm(
                "Salir",
                "¿Cerrar MiFutbolR? S = sí, N/Esc = no.",
                ConfirmAction::Quit,
            ));
        } else {
            self.go_back();
        }
    }

    fn open_module(&mut self, module: Module) -> Result<()> {
        match module {
            Module::Camisetas => {
                self.push_screen(Screen::CamisetasList);
                self.refresh_camisetas()?;
            }
            Module::Canchas => {
                self.push_screen(Screen::CanchasList);
                self.refresh_canchas()?;
            }
            Module::Botines => {
                self.push_screen(Screen::BotinesList);
                self.refresh_botines()?;
            }
            Module::Equipos => {
                self.push_screen(Screen::EquiposList);
                self.refresh_equipos()?;
            }
            Module::Partidos => {
                self.push_screen(Screen::PartidosList);
                self.refresh_partidos()?;
            }
            Module::Lesiones => {
                self.push_screen(Screen::LesionesList);
                self.refresh_lesiones()?;
            }
            Module::Estadisticas => {
                self.push_screen(Screen::EstadisticasHub);
                self.load_estadisticas()?;
            }
            Module::Financiamiento => {
                self.push_screen(Screen::FinanciamientoList);
                self.refresh_financiamiento()?;
            }
            Module::Carrera => {
                self.push_screen(Screen::CarreraMenu);
                self.refresh_carrera()?;
            }
            Module::Resumen => {
                self.push_screen(Screen::ResumenPreview);
                self.regenerar_resumen()?;
            }
            Module::Ajustes => {
                self.open_ajustes_form()?;
            }
        }
        Ok(())
    }

    /// Recarga el listado de camisetas.
    pub fn refresh_camisetas(&mut self) -> Result<()> {
        let rows = camiseta_service::listar(self.db.conn(), &self.camiseta_filter)?;
        self.camisetas = rows;
        if self.camisetas.is_empty() {
            self.camisetas_table.select(None);
        } else {
            let index = self
                .camisetas_table
                .selected()
                .unwrap_or(0)
                .min(self.camisetas.len() - 1);
            self.camisetas_table.select(Some(index));
        }
        Ok(())
    }

    /// Id de la fila seleccionada.
    #[must_use]
    pub fn selected_camiseta_id(&self) -> Option<i64> {
        let index = self.camisetas_table.selected()?;
        self.camisetas.get(index).map(|row| row.id)
    }

    fn open_camiseta_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_camiseta_id() else {
            self.status = "No hay camisetas para mostrar".to_string();
            return Ok(());
        };
        match camiseta_service::obtener(self.db.conn(), id) {
            Ok(camiseta) => {
                self.camiseta_detail = Some(camiseta);
                self.camiseta_ficha = camiseta_service::ficha(self.db.conn(), id).ok();
                self.push_screen(Screen::CamisetaDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_camiseta_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                self.form_original = None;
                self.form = Some(camiseta_form(None));
                self.push_screen(Screen::CamisetaForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_camiseta_id() {
                    self.open_camiseta_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_camiseta_form_for(&mut self, id: i64) -> Result<()> {
        match camiseta_service::obtener(self.db.conn(), id) {
            Ok(camiseta) => {
                self.form = Some(camiseta_form(Some(&camiseta)));
                self.form_original = Some(camiseta);
                self.push_screen(Screen::CamisetaForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_camiseta(&mut self) {
        let Some(id) = self.selected_camiseta_id() else {
            self.status = "No hay camiseta seleccionada".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar camiseta",
            "¿Eliminar la camiseta seleccionada? Si tiene historial se desactivará. (S = sí, N = no)",
            ConfirmAction::DeleteCamiseta(id),
        ));
    }

    fn delete_camiseta(&mut self, id: i64) -> Result<()> {
        match camiseta_service::eliminar(self.db.conn(), id) {
            Ok(ResultadoEliminacion::Eliminada) => {
                self.status = "Camiseta eliminada".to_string();
            }
            Ok(ResultadoEliminacion::Desactivada(dependencias)) => {
                self.status = format!(
                    "Camiseta desactivada ({} partidos, {} lesiones)",
                    dependencias.partidos, dependencias.lesiones
                );
            }
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        }
        if matches!(self.current_screen(), Screen::CamisetaDetail(_)) {
            self.stack.pop();
        }
        self.camiseta_detail = None;
        self.camiseta_ficha = None;
        self.refresh_camisetas()?;
        Ok(())
    }

    fn sortear_camiseta(&mut self) -> Result<()> {
        match camiseta_service::sortear(self.db.conn()) {
            Ok(Some(camiseta)) => {
                self.status = format!("Sorteada: {}", camiseta.nombre);
                self.modal = Some(Modal::Info(format!(
                    "Camiseta sorteada: {}\n\n{}",
                    camiseta.nombre,
                    camiseta.descripcion_corta()
                )));
                self.refresh_camisetas()?;
            }
            Ok(None) => {
                self.modal = Some(Modal::Info(
                    "No hay camisetas activas para sortear.".to_string(),
                ));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn discard_form(&mut self) {
        self.form = None;
        self.form_original = None;
        self.form_original_cancha = None;
        self.form_original_botin = None;
        self.form_original_equipo = None;
        self.form_original_partido = None;
        self.form_original_lesion = None;
        self.form_original_transaccion = None;
        self.carrera_hito_edit = None;
        self.jugador_edit_id = None;
        self.go_back();
    }

    fn save_current_form(&mut self) -> Result<()> {
        match self.current_screen() {
            Screen::CamisetaForm(mode) => self.save_camiseta_form(mode),
            Screen::CanchaForm(mode) => self.save_cancha_form(mode),
            Screen::BotinForm(mode) => self.save_botin_form(mode),
            Screen::EquipoForm(mode) => self.save_equipo_form(mode),
            Screen::JugadorForm(mode) => self.save_jugador_form(mode),
            Screen::PartidoForm(mode) => self.save_partido_form(mode),
            Screen::PartidoWizard => self.save_partido_wizard(),
            Screen::LesionForm(mode) => self.save_lesion_form(mode),
            Screen::TransaccionForm(mode) => self.save_transaccion_form(mode),
            Screen::PresupuestoForm => self.save_presupuesto_form(),
            Screen::CarreraIdentidadForm => self.save_carrera_identidad(),
            Screen::CarreraHitoForm(mode) => self.save_carrera_hito(mode),
            Screen::AjustesForm => self.save_ajustes_form(),
            Screen::LegacyImportForm => self.save_legacy_import(),
            _ => {
                self.status = "No hay formulario activo".to_string();
                Ok(())
            }
        }
    }

    /// Guarda el formulario de camiseta.
    pub(crate) fn save_camiseta_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let camiseta = match self.camiseta_desde_form() {
            Ok(camiseta) => camiseta,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nueva = camiseta;
                camiseta_service::crear(self.db.conn(), &mut nueva).map(|_| ())
            }
            FormMode::Edit => camiseta_service::actualizar(self.db.conn(), &camiseta),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original = None;
                self.stack.pop();
                self.refresh_camisetas()?;
                if let Screen::CamisetaDetail(id) = self.current_screen()
                    && let Ok(camiseta) = camiseta_service::obtener(self.db.conn(), id)
                {
                    self.camiseta_detail = Some(camiseta);
                    self.camiseta_ficha = camiseta_service::ficha(self.db.conn(), id).ok();
                }
                self.status = "Camiseta guardada".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn camiseta_desde_form(&self) -> Result<Camiseta, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();

        let estado = EstadoFisico::from_str(&get("estado_fisico"))
            .map_err(|_| DomainError::Message("Estado físico inválido".to_string()))?;
        let fecha_texto = get("fecha_compra");
        let fecha_compra = if fecha_texto.is_empty() {
            None
        } else {
            Some(validation::date(&fecha_texto, "fecha de compra")?)
        };
        let costo = validation::money(&get("costo"), "costo")?;
        let fue_regalo = get("fue_regalo") == "Sí";
        let original = self.form_original.as_ref();

        Ok(Camiseta {
            id: original.map_or(0, |camiseta| camiseta.id),
            nombre: get("nombre"),
            color_principal: get("color_principal"),
            color_secundario: get("color_secundario"),
            marca: get("marca"),
            modelo: get("modelo"),
            temporada: get("temporada"),
            estado_fisico: estado,
            fecha_compra,
            costo,
            observaciones: get("observaciones"),
            proveedor: get("proveedor"),
            fue_regalo,
            regalo_de: get("regalo_de"),
            imagen_ruta: get("imagen_ruta"),
            activa: original.is_none_or(|camiseta| camiseta.activa),
            sorteada: original.is_some_and(|camiseta| camiseta.sorteada),
        })
    }
}

/// Construye el formulario de camiseta.
fn camiseta_form(camiseta: Option<&Camiseta>) -> Form {
    let value = |key: &str| {
        camiseta.map(|c| match key {
            "nombre" => c.nombre.clone(),
            "color_principal" => c.color_principal.clone(),
            "color_secundario" => c.color_secundario.clone(),
            "marca" => c.marca.clone(),
            "modelo" => c.modelo.clone(),
            "temporada" => c.temporada.clone(),
            "estado_fisico" => c.estado_fisico.as_str().to_string(),
            "fecha_compra" => c.fecha_compra.map(dates::display_date).unwrap_or_default(),
            "costo" => c.costo.display_ars(),
            "observaciones" => c.observaciones.clone(),
            "proveedor" => c.proveedor.clone(),
            "regalo_de" => c.regalo_de.clone(),
            "imagen_ruta" => c.imagen_ruta.clone(),
            _ => String::new(),
        })
    };

    let estado_options = EstadoFisico::ALL
        .iter()
        .map(|estado| (estado.as_str().to_string(), estado.label().to_string()))
        .collect();

    let fields = vec![
        Field::text("nombre", "Nombre", value("nombre").unwrap_or_default(), 100).required(),
        Field::select(
            "estado_fisico",
            "Estado físico",
            value("estado_fisico").unwrap_or_else(|| EstadoFisico::Buena.as_str().to_string()),
            estado_options,
        ),
        Field::text("marca", "Marca", value("marca").unwrap_or_default(), 60),
        Field::text("modelo", "Modelo", value("modelo").unwrap_or_default(), 80),
        Field::text(
            "temporada",
            "Temporada",
            value("temporada").unwrap_or_default(),
            40,
        ),
        Field::text(
            "color_principal",
            "Color principal",
            value("color_principal").unwrap_or_default(),
            50,
        ),
        Field::text(
            "color_secundario",
            "Color secundario",
            value("color_secundario").unwrap_or_default(),
            50,
        ),
        Field {
            key: "fecha_compra",
            label: "Fecha de compra".to_string(),
            value: value("fecha_compra").unwrap_or_default(),
            kind: FieldKind::Date,
            required: false,
            help: Some("DD/MM/AAAA".to_string()),
        },
        Field {
            key: "costo",
            label: "Costo".to_string(),
            value: value("costo").unwrap_or_else(|| "$0,00".to_string()),
            kind: FieldKind::Money,
            required: false,
            help: None,
        },
        Field::text(
            "proveedor",
            "Proveedor",
            value("proveedor").unwrap_or_default(),
            120,
        ),
        Field::boolean(
            "fue_regalo",
            "¿Fue regalo?",
            camiseta.is_some_and(|c| c.fue_regalo),
        ),
        Field::text(
            "regalo_de",
            "Regalo de",
            value("regalo_de").unwrap_or_default(),
            100,
        ),
        Field::text(
            "imagen_ruta",
            "Imagen",
            value("imagen_ruta").unwrap_or_default(),
            255,
        ),
        Field {
            key: "observaciones",
            label: "Observaciones".to_string(),
            value: value("observaciones").unwrap_or_default(),
            kind: FieldKind::Multiline { max: 240 },
            required: false,
            help: None,
        },
    ];
    let title = if camiseta.is_some() {
        "Editar camiseta"
    } else {
        "Nueva camiseta"
    };
    Form::new(title, fields)
}

impl Camiseta {
    /// Resumen de una línea para modales.
    #[must_use]
    pub fn descripcion_corta(&self) -> String {
        let mut partes = Vec::new();
        if !self.marca.is_empty() {
            partes.push(self.marca.clone());
        }
        if !self.temporada.is_empty() {
            partes.push(self.temporada.clone());
        }
        partes.push(self.estado_fisico.label().to_string());
        partes.join(" · ")
    }
}

/// Bucle principal: dibuja, espera teclado y repite (§19.10).
pub fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| crate::ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key)?;
        }
        if app.should_quit {
            return Ok(());
        }
    }
}
