//! Estado y navegación de la aplicación (ADR-05).
//!
//! `App` es la máquina de pantallas: pila de navegación, modales, formularios
//! y preferencias. El dibujo vive en `ui`; aquí no se renderiza nada.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::DefaultTerminal;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::{ListState, TableState};

use crate::config::AppPaths;
use crate::db::Database;
use crate::domain::amigos::{Amigo, RankingAmigo};
use crate::domain::busqueda::ResultadoBusqueda;
use crate::domain::calendario::CalendarioState;
use crate::domain::comparador::{Comparacion, OpcionComparador, TipoComparacion};
use crate::domain::inicio::InicioResumen;
use crate::domain::logros::LogroProgreso;
use crate::domain::media::{MediaItem, TipoMedia};
use crate::domain::models::{
    AppSettings, Botin, BotinFilter, BotinRow, Camiseta, CamisetaFilter, CamisetaRow, Cancha,
    CanchaFilter, CanchaRow, CarreraHall, CarreraIdentidad, CarreraPerfil, CarreraResumen,
    CategoriaTotal, Equipo, EquipoFilter, EquipoRow, EstadisticaKind, Estadisticas, EstadoFisico,
    FichaCamiseta, FinanciamientoFilter, FinanzasResumen, FormMode, HallPartido, HitoRow, Lesion,
    LesionFilter, LesionRow, LesionStats, Mode, Partido, PartidoFilter, PartidoResumen, PartidoRow,
    PresupuestoVista, TimelineEvento, TransaccionFinanciera,
};
use crate::domain::notificacion::Notificacion;
use crate::domain::progresion::{PlanEntrenamiento, ProgresionFila};
use crate::domain::recordatorios::{EventoAgenda, Recordatorio};
use crate::domain::records::RecordItem;
use crate::domain::resumen::{DatosResumen, ResumenFormato};
use crate::domain::temporadas::{ComparacionTemporadas, Temporada, TemporadaFicha};
use crate::domain::tiendas::Tienda;
use crate::domain::torneos::{Torneo, TorneoFicha};
use crate::domain::ultimos::UltimoItem;
use crate::domain::usuario::Usuario;
use crate::domain::validation::{self, DomainError};
use crate::domain::{Cents, dates};
use crate::services::botin_service;
use crate::services::camiseta_service::{self, ResultadoEliminacion};
use crate::services::{inicio_service, notificacion_service};
use crate::ui::animation::UiAnimations;
use crate::ui::images::ImageCache;

/// Nombre visible de la aplicación.
pub const APP_NAME: &str = "MiFutbolR";

/// Pausa tras un dígito ambiguo del menú ("1" puede ser 1 o 15) antes de abrirlo.
pub const ESPERA_NUMERO_MENU: Duration = Duration::from_millis(900);

/// Opciones del menú principal, en el orden y numeración de MiFutbolC
/// (`menu.c`, `MENU_ITEMS`); al final, los extras de MiFutbolR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    /// 1.
    Calendario,
    /// 2.
    Camisetas,
    /// 3.
    Canchas,
    /// 4.
    Botines,
    /// 5.
    Equipos,
    /// 6.
    Partidos,
    /// 7.
    Lesiones,
    /// 8.
    Estadisticas,
    /// 9.
    Logros,
    /// 10.
    Financiamiento,
    /// 11.
    Torneos,
    /// 12.
    Temporadas,
    /// 13.
    Analisis,
    /// 14.
    Bienestar,
    /// 15.
    Carrera,
    /// 16.
    Recordatorios,
    /// 17.
    Colecciones,
    /// 18.
    Records,
    /// 19.
    Tiendas,
    /// 20.
    Media,
    /// 21.
    Tutorial,
    /// 22.
    Resumen,
    /// 23.
    Amigos,
    /// 24.
    Ajustes,
    /// 25.
    Sincronizacion,
    /// 26 (extra de MiFutbolR).
    Progresion,
    /// 27 (extra de MiFutbolR).
    EntrenadorIa,
    /// 28 (extra de MiFutbolR).
    Usuario,
}

impl Module {
    /// Orden del menú principal: la posición + 1 es el número de la opción.
    pub const ALL: [Self; 28] = [
        Self::Calendario,
        Self::Camisetas,
        Self::Canchas,
        Self::Botines,
        Self::Equipos,
        Self::Partidos,
        Self::Lesiones,
        Self::Estadisticas,
        Self::Logros,
        Self::Financiamiento,
        Self::Torneos,
        Self::Temporadas,
        Self::Analisis,
        Self::Bienestar,
        Self::Carrera,
        Self::Recordatorios,
        Self::Colecciones,
        Self::Records,
        Self::Tiendas,
        Self::Media,
        Self::Tutorial,
        Self::Resumen,
        Self::Amigos,
        Self::Ajustes,
        Self::Sincronizacion,
        Self::Progresion,
        Self::EntrenadorIa,
        Self::Usuario,
    ];

    /// Número fijo de la opción (el de MiFutbolC), aunque el modo la oculte.
    #[must_use]
    pub fn numero(self) -> usize {
        Self::ALL
            .iter()
            .position(|module| *module == self)
            .map_or(0, |posicion| posicion + 1)
    }

    /// ¿Visible en el modo Simple? (`DEFINE_GET_MENU` de `settings.c`).
    /// Usuario se suma porque en MiFutbolR es la única vía a las cuentas;
    /// Sincronización no tiene getter en C, así que se ve en todos los modos.
    #[must_use]
    pub const fn en_modo_simple(self) -> bool {
        matches!(
            self,
            Self::Calendario
                | Self::Camisetas
                | Self::Canchas
                | Self::Botines
                | Self::Partidos
                | Self::Lesiones
                | Self::Tutorial
                | Self::Ajustes
                | Self::Sincronizacion
                | Self::Usuario
        )
    }

    /// Etiqueta del menú.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Calendario => "Calendario",
            Self::Camisetas => "Camisetas",
            Self::Canchas => "Canchas",
            Self::Botines => "Botines",
            Self::Equipos => "Equipos",
            Self::Partidos => "Partidos",
            Self::Lesiones => "Lesiones",
            Self::Estadisticas => "Estadísticas",
            Self::Logros => "Logros",
            Self::Financiamiento => "Financiamiento",
            Self::Torneos => "Torneos",
            Self::Temporadas => "Temporada",
            Self::Analisis => "Análisis",
            Self::Bienestar => "Bienestar",
            Self::Carrera => "Carrera Futbolística",
            Self::Recordatorios => "Recordatorios",
            Self::Colecciones => "Colecciones",
            Self::Records => "Records & Rankings",
            Self::Tiendas => "Tiendas",
            Self::Media => "Referencias Multimedia",
            Self::Tutorial => "Tutorial",
            Self::Resumen => "Resumen Compartible",
            Self::Amigos => "Ranking Amigos",
            Self::Ajustes => "Ajustes",
            Self::Sincronizacion => "Sincronización (PocketBase)",
            Self::Progresion => "Progresión",
            Self::EntrenadorIa => "Entrenador IA",
            Self::Usuario => "Usuario",
        }
    }

    /// Etiqueta en el idioma de Ajustes (clave `menu_<key>` de `langs/*.json` de C).
    #[must_use]
    pub fn etiqueta(self, idioma: crate::domain::models::Language) -> &'static str {
        crate::i18n::t(idioma, &format!("menu_{}", self.key()), self.label())
    }

    /// Etiqueta corta para la barra de módulos.
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Calendario => "Calend",
            Self::Logros => "Logros",
            Self::Analisis => "Análisis",
            Self::Records => "Records",
            Self::Tutorial => "Tutorial",
            Self::Camisetas => "Camis",
            Self::Canchas => "Canchas",
            Self::Botines => "Botines",
            Self::Equipos => "Equipos",
            Self::Partidos => "Partidos",
            Self::Lesiones => "Lesiones",
            Self::Estadisticas => "Stats",
            Self::Financiamiento => "Finanzas",
            Self::Carrera => "Carrera",
            Self::Resumen => "Resumen",
            Self::Ajustes => "Ajustes",
            Self::Sincronizacion => "Sync",
            Self::Torneos => "Torneos",
            Self::Temporadas => "Temporadas",
            Self::Tiendas => "Tiendas",
            Self::Amigos => "Amigos",
            Self::Media => "Multimedia",
            Self::Recordatorios => "Recordatorios",
            Self::Progresion => "Progresión",
            Self::Bienestar => "Bienestar",
            Self::Colecciones => "Colecciones",
            Self::EntrenadorIa => "IA",
            Self::Usuario => "Usuario",
        }
    }

    /// Clave de `custom_menus` (los nombres de `menu_custom_menus` de MiFutbolC).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Calendario => "calendario",
            Self::Logros => "logros",
            Self::Analisis => "analisis",
            Self::Records => "records_rankings",
            Self::Tutorial => "tutorial",
            Self::Camisetas => "camisetas",
            Self::Canchas => "canchas",
            Self::Botines => "botines",
            Self::Equipos => "equipos",
            Self::Partidos => "partidos",
            Self::Lesiones => "lesiones",
            Self::Estadisticas => "estadisticas",
            Self::Financiamiento => "financiamiento",
            Self::Carrera => "carrera",
            Self::Resumen => "resumen_compartible",
            Self::Ajustes => "settings",
            Self::Sincronizacion => "sincronizacion",
            Self::Torneos => "torneos",
            Self::Temporadas => "temporada",
            Self::Tiendas => "tiendas",
            Self::Amigos => "ranking_amigos",
            Self::Media => "media",
            Self::Recordatorios => "recordatorios",
            Self::Progresion => "progresion",
            Self::Bienestar => "bienestar",
            Self::Colecciones => "colecciones",
            Self::EntrenadorIa => "entrenador_ia",
            Self::Usuario => "usuario",
        }
    }

    /// Descripción breve para el menú.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Calendario => "Partidos, lesiones y recordatorios del mes.",
            Self::Logros => "Logros y badges con su progreso.",
            Self::Analisis => "Básico, evolución, comparador, Entrenador IA y química.",
            Self::Records => "Récords personales y rankings.",
            Self::Tutorial => "Recorrido guiado por la aplicación.",
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
            Self::Sincronizacion => "Partidos, camisetas y botines con un servidor PocketBase.",
            Self::Torneos => "Torneos, equipos, fixture, resultados y tabla.",
            Self::Temporadas => "Temporadas, fases, resumen, fatiga y comparación.",
            Self::Tiendas => "Tiendas deportivas, contacto, productos y favoritos.",
            Self::Amigos => "Amigos, ranking y comparación con el usuario.",
            Self::Media => "Videos, fotos, artículos y audio asociados a partidos.",
            Self::Recordatorios => "Recordatorios, recurrencias y agenda.",
            Self::Progresion => "Planes de entrenamiento y evolución semanal por jugador.",
            Self::Bienestar => {
                "Hábitos, entrenamientos, nutrición, mente, salud, objetivos y asistente."
            }
            Self::Colecciones => "Inventario de equipamiento y colecciones.",
            Self::EntrenadorIa => "Consejos, predicción, táctica y descanso según tus partidos.",
            Self::Usuario => "Cuentas locales, inicio de sesión y bienvenida.",
        }
    }
}

/// Tabla cuyas filas aceptan clic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableHitTarget {
    /// Listado de camisetas.
    Camisetas,
    /// Listado de partidos.
    Partidos,
    /// Listado de canchas.
    Canchas,
    /// Listado de botines.
    Botines,
    /// Listado de equipos.
    Equipos,
    /// Listado de lesiones.
    Lesiones,
    /// Listado de transacciones.
    Financiamiento,
}

/// Pantalla activa de la pila de navegación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Menú principal.
    MainMenu,
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
    /// Administración de backups (Ajustes → Ctrl+G).
    BackupsList,
    /// Exportar / importar (Ajustes → Ctrl+D).
    ExportarImportar,
    /// Editor de menús personalizados (Ajustes → Ctrl+N; `menu_custom_menus` de C).
    MenusPersonalizados,
    /// Visor de imagen a pantalla completa.
    ImagenViewer,
    /// Formulario de importación legacy.
    LegacyImportForm,
    /// Calendario mensual.
    Calendario,
    /// Catálogo de logros.
    Logros,
    /// Bandeja de notificaciones.
    Notificaciones,
    /// Tutorial de inicio.
    Tutorial,
    /// Récords y rankings.
    Records,
    /// Listado de torneos.
    TorneosList,
    /// Detalle de un torneo.
    TorneoDetail(i64),
    /// Formulario de torneo.
    TorneoForm(FormMode),
    /// Formulario de resultado de un partido de torneo.
    TorneoResultadoForm(i64),
    /// Últimos partidos por resultado/aporte.
    Ultimos,
    /// Hub de análisis (Comparador A/B y Entrenador IA).
    AnalisisMenu,
    /// 25 Sincronización (PocketBase).
    Sincronizacion,
    /// Formulario de URLs del servidor.
    SyncConfigForm,
    /// Formulario de inicio de sesión en PocketBase.
    SyncLoginForm,
    /// Resolución de conflictos.
    SyncConflicto,
    /// Comparador A/B.
    Comparador,
    /// Análisis básico (últimos 5, rachas, mensaje).
    AnalisisBasico,
    /// Evolución temporal mensual.
    AnalisisEvolucion,
    /// Química entre jugadores.
    Quimica,
    /// Formulario de dupla.
    QuimicaForm(FormMode),
    /// Listado de temporadas.
    TemporadasList,
    /// Detalle de una temporada.
    TemporadaDetail(i64),
    /// Formulario de temporada.
    TemporadaForm(FormMode),
    /// Listado de tiendas.
    TiendasList,
    /// Formulario de tienda.
    TiendaForm(FormMode),
    /// Bienestar: listado por secciones.
    BienestarList,
    /// Formulario de Bienestar.
    BienestarForm(FormMode),
    /// Asistente de entrenamientos.
    BienestarAsistente,
    /// Inventario y colecciones.
    ColeccionesList,
    /// Formulario de ítem, colección o asignación.
    ColeccionForm(FormMode),
    /// Ítems de una colección.
    ColeccionDetail(i64),
    /// Entrenador IA.
    EntrenadorIa,
    /// Ranking de amigos.
    AmigosList,
    /// Formulario de amigo.
    AmigoForm(FormMode),
    /// Listado de multimedia.
    MediaList,
    /// Formulario de referencia multimedia.
    MediaForm(FormMode),
    /// Listado de recordatorios.
    RecordatoriosList,
    /// Formulario de recordatorio.
    RecordatorioForm(FormMode),
    /// Agenda fusionada.
    Agenda,
    /// Listado de progresión.
    ProgresionList,
    /// Formulario de medición.
    ProgresionForm(FormMode),
    /// Listado de planes.
    PlanesList,
    /// Formulario de plan.
    PlanForm(FormMode),
    /// Pantalla de bienvenida.
    Bienvenida,
    /// Listado de usuarios.
    UsuarioList,
    /// Formulario de usuario.
    UsuarioForm(FormMode),
    /// Inicio de sesión.
    UsuarioLogin,
    /// Contraseña actual antes de eliminar una cuenta.
    UsuarioEliminar,
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
    /// Restaurar el backup de la fila indicada de `App::backups`.
    RestoreBackupSel(usize),
    /// Eliminar el backup de la fila indicada de `App::backups`.
    DeleteBackupSel(usize),
    /// VACUUM + ANALYZE + integridad, con backup previo.
    Mantenimiento,
    /// Importar `recordatorios.json` reemplazando los existentes.
    ImportarRecordatoriosReemplazo,
    /// Reiniciar el sorteo de botines.
    ResetSorteoBotines,
    /// Deshacer la última operación.
    Undo,
    /// Eliminar el torneo indicado.
    DeleteTorneo(i64),
    /// Generar el fixture del torneo indicado.
    GenerarFixture(i64),
    /// Eliminar la temporada indicada.
    DeleteTemporada(i64),
    /// Eliminar la tienda indicada.
    DeleteTienda(i64),
    /// Eliminar un registro de la sección activa de Bienestar.
    DeleteBienestar(i64),
    /// Eliminar un ítem del inventario.
    DeleteInventarioItem(i64),
    /// Eliminar una colección.
    DeleteColeccion(i64),
    /// Eliminar el amigo indicado.
    DeleteAmigo(i64),
    /// Eliminar la dupla de química indicada.
    DeleteQuimica(i64),
    /// Eliminar la referencia multimedia indicada.
    DeleteMedia(i64),
    /// Eliminar el recordatorio indicado.
    DeleteRecordatorio(i64),
    /// Eliminar la medición indicada.
    DeleteProgresion(i64),
    /// Eliminar el plan indicado.
    DeletePlan(i64),
    /// Eliminar la cuenta indicada.
    DeleteUsuario(i64),
    /// Abrir Carrera → Modo retro (aviso de inicio de C).
    AbrirModoRetro,
    /// Restablecer los ajustes a sus valores por defecto.
    RestablecerAjustes,
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
    /// Contraseña (se muestra enmascarada).
    Password {
        /// Longitud máxima.
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

    /// Crea un campo de contraseña.
    #[must_use]
    pub fn password(key: &'static str, label: &str, value: impl Into<String>, max: usize) -> Self {
        Self {
            key,
            label: label.to_string(),
            value: value.into(),
            kind: FieldKind::Password { max },
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
            FieldKind::Password { .. } => "•".repeat(self.value.chars().count()),
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
                | FieldKind::Password { .. }
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
            FieldKind::MultiSelect { .. } | FieldKind::Password { .. } => {}
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
        if let Some(field) = self.fields.get_mut(self.focus)
            && field.is_text_entry()
        {
            field.push_char(ch);
            self.dirty = true;
        }
    }

    /// Borra el último carácter del campo activo.
    pub fn pop_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focus)
            && field.is_text_entry()
            && !field.value.is_empty()
        {
            field.pop_char();
            self.dirty = true;
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

/// Pestañas del catálogo de logros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogrosTab {
    /// Todos.
    Todos,
    /// Solo completados.
    Completados,
    /// En progreso (valor > 0).
    EnProgreso,
    /// Sin empezar.
    Bloqueados,
}

impl LogrosTab {
    /// Todas las pestañas.
    pub const ALL: [Self; 4] = [
        Self::Todos,
        Self::Completados,
        Self::EnProgreso,
        Self::Bloqueados,
    ];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Todos => "Todos",
            Self::Completados => "Completados",
            Self::EnProgreso => "En progreso",
            Self::Bloqueados => "Bloqueados",
        }
    }

    /// Siguiente pestaña.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Todos => Self::Completados,
            Self::Completados => Self::EnProgreso,
            Self::EnProgreso => Self::Bloqueados,
            Self::Bloqueados => Self::Todos,
        }
    }

    /// Anterior.
    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Todos => Self::Bloqueados,
            Self::Completados => Self::Todos,
            Self::EnProgreso => Self::Completados,
            Self::Bloqueados => Self::EnProgreso,
        }
    }

    /// ¿Entra en la pestaña?
    #[must_use]
    pub fn incluye(self, logro: &LogroProgreso) -> bool {
        match self {
            Self::Todos => true,
            Self::Completados => logro.completado,
            Self::EnProgreso => !logro.completado && logro.valor > 0,
            Self::Bloqueados => !logro.completado && logro.valor == 0,
        }
    }
}

/// Estado de la búsqueda global.
#[derive(Debug, Clone, Default)]
pub struct BusquedaState {
    /// Texto.
    pub query: String,
    /// Resultados.
    pub resultados: Vec<ResultadoBusqueda>,
    /// Selección.
    pub selected: usize,
}

/// Estado de la paleta de comandos.
#[derive(Debug, Clone, Default)]
pub struct PaletaState {
    /// Texto.
    pub query: String,
    /// Selección.
    pub selected: usize,
}

/// Overlay abierto sobre la pantalla actual.
#[derive(Debug, Clone)]
pub enum Overlay {
    /// Búsqueda global.
    Busqueda(BusquedaState),
    /// Paleta de comandos.
    Paleta(PaletaState),
}

/// Acciones de la paleta: (etiqueta, id).
pub const ACCIONES_PALETA: [(&str, &str); 25] = [
    ("Nuevo partido", "nuevo_partido"),
    ("Ir a Partidos", "partidos"),
    ("Ir a Camisetas", "camisetas"),
    ("Ir a Canchas", "canchas"),
    ("Calendario", "calendario"),
    ("Logros", "logros"),
    ("Récords y rankings", "records"),
    ("Torneos", "torneos"),
    ("Últimos partidos", "ultimos"),
    ("Comparador A/B", "comparador"),
    ("Temporadas", "temporadas"),
    ("Tiendas", "tiendas"),
    ("Ranking de amigos", "amigos"),
    ("Multimedia", "media"),
    ("Recordatorios", "recordatorios"),
    ("Progresión", "progresion"),
    ("Bienestar", "bienestar"),
    ("Colecciones e inventario", "colecciones"),
    ("Entrenador IA", "entrenador_ia"),
    ("Usuario", "usuario"),
    ("Notificaciones", "notificaciones"),
    ("Resumen compartible", "resumen"),
    ("Deshacer última acción", "deshacer"),
    ("Ajustes", "ajustes"),
    ("Ayuda", "ayuda"),
];

/// Estado global de la aplicación.
pub struct App {
    /// Base de datos de la sesión actual (cuenta o invitado).
    pub db: Database,
    /// Registro de cuentas locales (`cuentas.db`).
    pub cuentas: Database,
    /// Rutas.
    pub paths: AppPaths,
    /// Pila de pantallas.
    pub stack: Vec<Screen>,
    /// Módulo activo (para la barra de módulos).
    pub active_module: Option<Module>,
    /// Modal activo.
    pub modal: Option<Modal>,
    /// Mensaje de estado.
    pub status: String,
    /// Salir.
    pub should_quit: bool,
    /// Preferencias.
    pub settings: AppSettings,
    /// Animaciones de la TUI.
    pub animations: UiAnimations,
    /// Caché de imágenes para las fichas.
    pub images: ImageCache,
    /// Imagen abierta en el visor.
    pub imagen_abierta: Option<String>,
    /// Zonas clicables del sidebar (módulo por fila).
    pub sidebar_hits: Vec<(Rect, Module)>,
    /// Zonas clicables del menú principal (fila por módulo).
    pub menu_hits: Vec<(Rect, usize)>,
    /// Zonas clicables de la tabla dibujada.
    pub table_hits: Vec<(Rect, usize)>,
    /// Tabla a la que pertenecen `table_hits`.
    pub table_hit_target: Option<TableHitTarget>,
    /// Selección de tabla del frame anterior (para animar el cambio).
    pub last_selection: Option<(TableHitTarget, usize)>,
    /// Última pantalla dibujada (para animar transiciones).
    pub last_screen: Option<Screen>,
    /// Si el modal estaba abierto en el frame anterior.
    pub last_modal_open: bool,
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
    /// Panel "Hoy".
    pub inicio: InicioResumen,
    /// Calendario.
    pub calendario: CalendarioState,
    /// Logros.
    pub logros: Vec<LogroProgreso>,
    /// Pestaña de logros.
    pub logros_tab: LogrosTab,
    /// Selección de logros.
    pub logros_table: TableState,
    /// Notificaciones.
    pub notificaciones: Vec<Notificacion>,
    /// Selección de notificaciones.
    pub notificaciones_table: TableState,
    /// No leídas.
    pub no_leidas: i64,
    /// Overlay (búsqueda o paleta).
    pub overlay: Option<Overlay>,
    /// Paso del tutorial.
    pub tutorial_paso: usize,
    /// ¿Tutorial completado?
    pub tutorial_completado: bool,
    /// Récords calculados.
    pub records: Vec<RecordItem>,
    /// Selección de récords.
    pub records_table: TableState,
    /// Torneos.
    pub torneos: Vec<Torneo>,
    /// Selección de torneos.
    pub torneos_table: TableState,
    /// Ficha del torneo abierto.
    pub torneo_detail: Option<TorneoFicha>,
    /// Selección de partidos del fixture.
    pub torneo_partidos_table: TableState,
    /// Torneo en edición.
    pub torneo_edit_id: Option<i64>,
    /// Partido de torneo en carga de resultado.
    pub torneo_resultado_partido: Option<i64>,
    /// Últimos calculados.
    pub ultimos: Vec<UltimoItem>,
    /// Selección de últimos.
    pub ultimos_table: TableState,
    /// Tipo del comparador.
    pub comparador_tipo: TipoComparacion,
    /// Opciones del comparador.
    pub comparador_opciones: Vec<OpcionComparador>,
    /// Índice de A.
    pub comparador_a: usize,
    /// Índice de B.
    pub comparador_b: usize,
    /// Lado activo (0 = A, 1 = B).
    pub comparador_activo: usize,
    /// Resultado.
    pub comparador_resultado: Option<Comparacion>,
    /// Temporadas.
    pub temporadas: Vec<Temporada>,
    /// Selección de temporadas.
    pub temporadas_table: TableState,
    /// Ficha de temporada.
    pub temporada_detail: Option<TemporadaFicha>,
    /// Desplazamiento del detalle.
    pub temporada_scroll: u16,
    /// Temporada en edición.
    pub temporada_edit_id: Option<i64>,
    /// Comparación de temporadas.
    pub temporada_comparacion: Option<ComparacionTemporadas>,
    /// Tiendas.
    pub tiendas: Vec<Tienda>,
    /// Selección de tiendas.
    pub tiendas_table: TableState,
    /// Tienda en edición.
    pub tienda_edit_id: Option<i64>,
    /// Sección activa de Bienestar.
    pub bienestar_seccion: crate::domain::bienestar::Seccion,
    /// Registros de la sección activa.
    pub bienestar_registros: Vec<crate::domain::bienestar::Registro>,
    /// Selección de Bienestar.
    pub bienestar_table: TableState,
    /// Registro de Bienestar en edición.
    pub bienestar_edit_id: Option<i64>,
    /// Último cálculo del asistente.
    pub bienestar_asistente: Option<crate::domain::bienestar::Asistente>,
    /// Recomendaciones guardadas.
    pub bienestar_recomendaciones: Vec<crate::domain::bienestar::Recomendacion>,
    /// Inventario.
    pub inventario: Vec<crate::domain::colecciones::InventarioItem>,
    /// Selección del inventario.
    pub inventario_table: TableState,
    /// Colecciones.
    pub colecciones: Vec<crate::domain::colecciones::Coleccion>,
    /// Selección de colecciones.
    pub colecciones_table: TableState,
    /// `true` si la pestaña activa es Colecciones.
    pub colecciones_ver_colecciones: bool,
    /// Ítems de la colección abierta.
    pub coleccion_detalle: Vec<crate::domain::colecciones::InventarioItem>,
    /// Selección del detalle.
    pub coleccion_detalle_table: TableState,
    /// Id en edición (0 = nuevo).
    pub coleccion_edit_id: i64,
    /// Qué edita el formulario de colecciones.
    pub coleccion_formulario: crate::app_colecciones::FormularioColeccion,
    /// Último análisis del Entrenador IA.
    pub entrenador: Option<crate::domain::entrenador::Analisis>,
    /// Amigos.
    pub amigos: Vec<Amigo>,
    /// Ranking de amigos.
    pub amigos_ranking: Vec<RankingAmigo>,
    /// Selección de amigos.
    pub amigos_table: TableState,
    /// Amigo en edición.
    pub amigo_edit_id: Option<i64>,
    /// Backups listados en Ajustes → Ctrl+G.
    pub backups: Vec<crate::services::backup_service::BackupInfo>,
    /// Selección de backups.
    pub backups_table: TableState,
    /// Análisis básico calculado.
    pub analisis_basico: Option<crate::domain::analisis::AnalisisBasico>,
    /// Evolución temporal calculada.
    pub analisis_evolucion: Option<crate::domain::analisis::Evolucion>,
    /// Métrica del gráfico de evolución.
    pub analisis_metrica: crate::domain::analisis::Metrica,
    /// Desplazamiento de los reportes de análisis.
    pub analisis_scroll: u16,
    /// Duplas de química cargadas a mano.
    pub quimica_duplas: Vec<crate::domain::quimica::Dupla>,
    /// Mejor combinación según equipos de partidos.
    pub quimica_mejor: Option<crate::domain::quimica::MejorDupla>,
    /// Selección de duplas.
    pub quimica_table: TableState,
    /// Dupla en edición.
    pub quimica_edit_id: Option<i64>,
    /// Referencias multimedia.
    pub media: Vec<MediaItem>,
    /// Selección de multimedia.
    pub media_table: TableState,
    /// Referencia en edición.
    pub media_edit_id: Option<i64>,
    /// Filtro de tipo.
    pub media_filter: Option<TipoMedia>,
    /// Recordatorios.
    pub recordatorios: Vec<Recordatorio>,
    /// Selección de recordatorios.
    pub recordatorios_table: TableState,
    /// Filtro por temática de Recordatorios (`f`), como `filtrar_por_tematica` de C.
    pub recordatorios_tema: Option<String>,
    /// Recordatorio en edición.
    pub recordatorio_edit_id: Option<i64>,
    /// Agenda fusionada.
    pub agenda: Vec<EventoAgenda>,
    /// Selección de agenda.
    pub agenda_table: TableState,
    /// Mediciones de progresión.
    pub progresion: Vec<ProgresionFila>,
    /// Selección de progresión.
    pub progresion_table: TableState,
    /// Medición en edición.
    pub progresion_edit_id: Option<i64>,
    /// Jugadores para el formulario.
    pub progresion_jugadores: Vec<(i64, String)>,
    /// Planes para el formulario.
    pub progresion_planes: Vec<PlanEntrenamiento>,
    /// Planes.
    pub planes: Vec<PlanEntrenamiento>,
    /// Selección de planes.
    pub planes_table: TableState,
    /// Plan en edición.
    pub plan_edit_id: Option<i64>,
    /// Cuentas locales.
    pub usuarios: Vec<Usuario>,
    /// Selección de cuentas.
    pub usuarios_table: TableState,
    /// Cuenta en edición.
    pub usuario_edit_id: Option<i64>,
    /// Sesión actual.
    pub usuario_actual: Option<Usuario>,
    /// ¿Bienvenida vista?
    pub bienvenida_vista: bool,
    /// Nombre precargado en el login.
    pub usuario_login_nombre: String,
    /// Búsqueda activa.
    pub search: Option<SearchState>,
    /// El resumen de Inicio quedó desactualizado; se recalcula al verse el menú.
    pub inicio_sucio: bool,
    /// `total_changes()` de la conexión en el último recálculo del Inicio.
    pub inicio_cambios: u64,
    /// Consultas de clima en segundo plano (Open-Meteo).
    pub clima: crate::services::clima_service::ClimaWorker,
    /// Próxima revisión de la cola de clima.
    pub clima_proxima_consulta: std::time::Instant,
    /// Hub de análisis.
    pub analisis_hub: ListState,
    /// Número tecleado en el menú principal y cuándo llegó el último dígito.
    pub menu_numero: Option<(String, std::time::Instant)>,
    /// Claves de `custom_menus` desactivadas (modo Personalizado).
    pub menu_ocultos: Vec<String>,
    /// La oferta de restaurar backup ya se hizo en esta sesión.
    pub restauracion_ofrecida: bool,
    /// Sincronización PocketBase en segundo plano.
    pub sync: crate::services::sync_service::SyncWorker,
    /// Menú de sincronización.
    pub sync_hub: ListState,
    /// Menú Exportar / Importar de Ajustes.
    pub export_hub: ListState,
    /// Estado mostrado en la pantalla de sincronización (sin el token).
    pub sync_estado: crate::services::sync_service::Estado,
    /// Resultado de la última sincronización.
    pub sync_mensaje: String,
    /// Ya se hizo la sincronización de inicio (móvil + auto-sync).
    pub sincronizacion_inicio_hecha: bool,
    /// Tareas de arranque de C (backup de inicio, aviso de Modo Retro y de
    /// actualización). Solo el binario las activa: los tests no escriben
    /// backups ni consultan la red salvo que lo pidan.
    pub tareas_inicio: bool,
    /// Las tareas de arranque ya corrieron sobre esta base.
    pub tareas_inicio_hechas: bool,
    /// URL de la última release; `None` = no consultar nunca (tests).
    pub url_actualizaciones: Option<String>,
    /// Comprobación de actualizaciones en segundo plano.
    pub actualizacion: crate::services::actualizacion_service::ActualizacionWorker,
    /// Editor de menús personalizados.
    pub menus_hub: ListState,
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
        let inicio = inicio_service::resumen(db.conn()).unwrap_or_default();
        let no_leidas = notificacion_service::no_leidas(db.conn()).unwrap_or(0);
        let tutorial_completado =
            crate::db::settings_repo::tutorial_completado(db.conn()).unwrap_or(false);
        let sesion = crate::services::usuario_service::sesion(db.conn()).unwrap_or_default();
        let menu_ocultos = crate::db::settings_repo::menus_ocultos(db.conn());
        let cuentas = Database::open_cuentas(&paths.cuentas_file)?;
        let stack = if sesion.bienvenida_vista {
            vec![Screen::MainMenu]
        } else {
            vec![Screen::Bienvenida]
        };
        let mut app = Self {
            db,
            cuentas,
            paths,
            stack,
            active_module: None,
            modal: None,
            status,
            should_quit: false,
            settings,
            animations: UiAnimations::default(),
            images: ImageCache::default(),
            imagen_abierta: None,
            sidebar_hits: Vec::new(),
            menu_hits: Vec::new(),
            table_hits: Vec::new(),
            table_hit_target: None,
            last_selection: None,
            last_screen: None,
            last_modal_open: false,
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
            inicio,
            calendario: CalendarioState::default(),
            logros: Vec::new(),
            logros_tab: LogrosTab::Todos,
            logros_table: TableState::default(),
            notificaciones: Vec::new(),
            notificaciones_table: TableState::default(),
            no_leidas,
            overlay: None,
            tutorial_paso: 0,
            tutorial_completado,
            records: Vec::new(),
            records_table: TableState::default(),
            torneos: Vec::new(),
            torneos_table: TableState::default(),
            torneo_detail: None,
            torneo_partidos_table: TableState::default(),
            torneo_edit_id: None,
            torneo_resultado_partido: None,
            ultimos: Vec::new(),
            ultimos_table: TableState::default(),
            comparador_tipo: TipoComparacion::Camiseta,
            comparador_opciones: Vec::new(),
            comparador_a: 0,
            comparador_b: 0,
            comparador_activo: 0,
            comparador_resultado: None,
            temporadas: Vec::new(),
            temporadas_table: TableState::default(),
            temporada_detail: None,
            temporada_scroll: 0,
            temporada_edit_id: None,
            temporada_comparacion: None,
            tiendas: Vec::new(),
            tiendas_table: TableState::default(),
            tienda_edit_id: None,
            bienestar_seccion: crate::domain::bienestar::Seccion::Habitos,
            bienestar_registros: Vec::new(),
            bienestar_table: TableState::default(),
            bienestar_edit_id: None,
            bienestar_asistente: None,
            bienestar_recomendaciones: Vec::new(),
            inventario: Vec::new(),
            inventario_table: TableState::default(),
            colecciones: Vec::new(),
            colecciones_table: TableState::default(),
            colecciones_ver_colecciones: false,
            coleccion_detalle: Vec::new(),
            coleccion_detalle_table: TableState::default(),
            coleccion_edit_id: 0,
            coleccion_formulario: crate::app_colecciones::FormularioColeccion::default(),
            entrenador: None,
            amigos: Vec::new(),
            amigos_ranking: Vec::new(),
            amigos_table: TableState::default(),
            amigo_edit_id: None,
            backups: Vec::new(),
            backups_table: TableState::default(),
            analisis_basico: None,
            analisis_evolucion: None,
            analisis_metrica: crate::domain::analisis::Metrica::default(),
            analisis_scroll: 0,
            quimica_duplas: Vec::new(),
            quimica_mejor: None,
            quimica_table: TableState::default(),
            quimica_edit_id: None,
            media: Vec::new(),
            media_table: TableState::default(),
            media_edit_id: None,
            media_filter: None,
            recordatorios: Vec::new(),
            recordatorios_table: TableState::default(),
            recordatorios_tema: None,
            recordatorio_edit_id: None,
            agenda: Vec::new(),
            agenda_table: TableState::default(),
            progresion: Vec::new(),
            progresion_table: TableState::default(),
            progresion_edit_id: None,
            progresion_jugadores: Vec::new(),
            progresion_planes: Vec::new(),
            planes: Vec::new(),
            planes_table: TableState::default(),
            plan_edit_id: None,
            usuarios: Vec::new(),
            usuarios_table: TableState::default(),
            usuario_edit_id: None,
            usuario_login_nombre: String::new(),
            usuario_actual: None,
            bienvenida_vista: sesion.bienvenida_vista,
            search: None,
            inicio_sucio: false,
            inicio_cambios: 0,
            clima: crate::services::clima_service::ClimaWorker::default(),
            clima_proxima_consulta: std::time::Instant::now(),
            analisis_hub: ListState::default().with_selected(Some(0)),
            menu_numero: None,
            menu_ocultos,
            restauracion_ofrecida: false,
            sync: crate::services::sync_service::SyncWorker::default(),
            sync_hub: ListState::default().with_selected(Some(0)),
            export_hub: ListState::default().with_selected(Some(0)),
            sync_estado: crate::services::sync_service::Estado::default(),
            sync_mensaje: String::new(),
            sincronizacion_inicio_hecha: false,
            tareas_inicio: false,
            tareas_inicio_hechas: false,
            url_actualizaciones: None,
            actualizacion: Default::default(),
            menus_hub: ListState::default().with_selected(Some(0)),
        };
        let _ = app.refresh_usuarios();
        Ok(app)
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
            Screen::BackupsList => "Ajustes · backups".to_string(),
            Screen::ExportarImportar => "Ajustes · exportar / importar".to_string(),
            Screen::MenusPersonalizados => "Ajustes · menús personalizados".to_string(),
            Screen::ImagenViewer => "Imagen".to_string(),
            Screen::LegacyImportForm => "Ajustes · importar legacy".to_string(),
            Screen::Calendario => format!(
                "Calendario · {:02}/{:04}",
                self.calendario.month, self.calendario.year
            ),
            Screen::Logros => "Logros".to_string(),
            Screen::Notificaciones => "Notificaciones".to_string(),
            Screen::Tutorial => "Tutorial".to_string(),
            Screen::Records => "Récords y rankings".to_string(),
            Screen::TorneosList => "Torneos".to_string(),
            Screen::TorneoDetail(id) => format!("Torneos · #{id}"),
            Screen::TorneoForm(_) => "Torneos · formulario".to_string(),
            Screen::TorneoResultadoForm(_) => "Torneos · resultado".to_string(),
            Screen::Ultimos => "Últimos".to_string(),
            Screen::AnalisisMenu => "Análisis".to_string(),
            Screen::Sincronizacion => "Sincronización (PocketBase)".to_string(),
            Screen::SyncConfigForm => "Sincronización · servidor".to_string(),
            Screen::SyncLoginForm => "Sincronización · iniciar sesión".to_string(),
            Screen::SyncConflicto => "Sincronización · conflictos".to_string(),
            Screen::Comparador => "Comparador A/B".to_string(),
            Screen::AnalisisBasico => "Análisis básico".to_string(),
            Screen::AnalisisEvolucion => "Evolución temporal".to_string(),
            Screen::Quimica => "Química entre jugadores".to_string(),
            Screen::QuimicaForm(_) => "Química · formulario".to_string(),
            Screen::TemporadasList => "Temporadas".to_string(),
            Screen::TemporadaDetail(id) => format!("Temporadas · #{id}"),
            Screen::TemporadaForm(_) => "Temporadas · formulario".to_string(),
            Screen::TiendasList => "Tiendas".to_string(),
            Screen::TiendaForm(_) => "Tiendas · formulario".to_string(),
            Screen::BienestarList => format!("Bienestar · {}", self.bienestar_seccion.label()),
            Screen::BienestarForm(_) => "Bienestar · formulario".to_string(),
            Screen::BienestarAsistente => "Bienestar · asistente".to_string(),
            Screen::ColeccionesList => "Colecciones e inventario".to_string(),
            Screen::ColeccionForm(_) => "Colecciones · formulario".to_string(),
            Screen::ColeccionDetail(_) => "Colecciones · detalle".to_string(),
            Screen::EntrenadorIa => "Entrenador IA".to_string(),
            Screen::AmigosList => "Ranking de amigos".to_string(),
            Screen::AmigoForm(_) => "Amigos · formulario".to_string(),
            Screen::MediaList => "Multimedia".to_string(),
            Screen::MediaForm(_) => "Multimedia · formulario".to_string(),
            Screen::RecordatoriosList => "Recordatorios".to_string(),
            Screen::RecordatorioForm(_) => "Recordatorios · formulario".to_string(),
            Screen::Agenda => "Agenda".to_string(),
            Screen::ProgresionList => "Progresión".to_string(),
            Screen::ProgresionForm(_) => "Progresión · formulario".to_string(),
            Screen::PlanesList => "Planes de entrenamiento".to_string(),
            Screen::PlanForm(_) => "Planes · formulario".to_string(),
            Screen::Bienvenida => "Bienvenida".to_string(),
            Screen::UsuarioList => "Usuario".to_string(),
            Screen::UsuarioForm(_) => "Usuario · formulario".to_string(),
            Screen::UsuarioLogin => "Iniciar sesión".to_string(),
            Screen::UsuarioEliminar => "Eliminar cuenta".to_string(),
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
            Screen::MainMenu => {
                "↑↓/jk: mover  Enter/número: abrir  0/Esc/q: salir  Ctrl+F: buscar  !: avisos  ?: ayuda"
            }
            Screen::CamisetasList => {
                "n: nueva  e: editar  d: borrar  a: activar/reactivar  Enter: detalle  s: sortear  f: filtro  o: orden  /: buscar  x: exportar  r: refrescar"
            }
            Screen::CamisetaDetail(_) => {
                "e: editar  d: borrar  a: activar/reactivar  v: imagen  Esc: volver"
            }
            Screen::CamisetaForm(_)
            | Screen::CanchaForm(_)
            | Screen::BotinForm(_)
            | Screen::EquipoForm(_)
            | Screen::JugadorForm(_)
            | Screen::PartidoForm(_)
            | Screen::LesionForm(_)
            | Screen::TorneoForm(_)
            | Screen::TorneoResultadoForm(_)
            | Screen::TemporadaForm(_)
            | Screen::TiendaForm(_)
            | Screen::BienestarForm(_)
            | Screen::ColeccionForm(_)
            | Screen::AmigoForm(_)
            | Screen::SyncConfigForm
            | Screen::SyncLoginForm
            | Screen::QuimicaForm(_)
            | Screen::MediaForm(_)
            | Screen::RecordatorioForm(_)
            | Screen::ProgresionForm(_)
            | Screen::PlanForm(_)
            | Screen::UsuarioForm(_)
            | Screen::UsuarioEliminar => "Tab: campo  Ctrl+S: guardar  Esc: cancelar",
            Screen::CanchasList => {
                "n: nueva  e: editar  d: borrar  a: activar/desactivar  Enter: detalle  o: orden  /: buscar  x: exportar  r: refrescar"
            }
            Screen::CanchaDetail(_) => {
                "e: editar  d: borrar  a: activar/desactivar  v: imagen  Esc: volver"
            }
            Screen::BotinesList => {
                "n: nuevo  e: editar  d: borrar  s: sortear  R: reiniciar sorteo  p: predeterminado  a: activar/desactivar  /: buscar"
            }
            Screen::BotinDetail(_) => "e: editar  d: borrar  p: predeterminado  Esc: volver",
            Screen::EquiposList => {
                "n: nuevo  e: editar  d: borrar  a: activar/desactivar  Enter: detalle  o: orden  /: buscar  x: exportar  r: refrescar"
            }
            Screen::EquipoDetail(_) => {
                "a: agregar jugador  e: editar jugador  d: borrar jugador  Esc: volver"
            }
            Screen::PartidosList => {
                "n: nuevo  e: editar  d: borrar  Enter: detalle  F: favorito  f: tipo  v: resultado  o: orden  PgUp/PgDn: página  /: buscar  c: clima  x: exportar"
            }
            Screen::PartidoDetail(_) => "e: editar  d: borrar  F: favorito  Esc: volver",
            Screen::PartidoWizard => {
                "Tab: campo  Enter/PgDn: siguiente paso  PgUp: anterior  Ctrl+S: guardar  Esc: cancelar"
            }
            Screen::LesionesList => {
                "n: nueva  e: editar  d: borrar  Enter: detalle  f: estado  o: orden  /: buscar  x: exportar  r: refrescar"
            }
            Screen::LesionDetail(_) => "e: editar  d: borrar  Esc: volver",
            Screen::EstadisticasHub => "↑↓: categoría  Enter: abrir  r: recalcular  Esc: volver",
            Screen::EstadisticasDetalle(_) => "↑↓: desplazar  PgUp/PgDn: página  Esc: volver",
            Screen::FinanciamientoList => {
                "n: nueva  e: editar  d: borrar  p: presupuesto  f: tipo  c: categoría  /: buscar  x: exportar  r: refrescar"
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
                "Tab: campo  Ctrl+S: guardar  Ctrl+B: backup  Ctrl+I: integridad  Ctrl+R: restaurar  Ctrl+G: backups  Ctrl+T: mantenimiento  Ctrl+D: exportar / importar  Ctrl+N: menús  Ctrl+U: actualizaciones  Ctrl+F: ver configuración  Ctrl+Q: restablecer  Esc: volver"
            }
            Screen::LegacyImportForm => {
                "Ruta del archivo .db legacy  ·  Ctrl+S: importar  Esc: cancelar"
            }
            Screen::BackupsList => {
                "Enter: restaurar  d: borrar  b: crear backup  r: refrescar  Esc: volver"
            }
            Screen::ExportarImportar => "↑↓: opción  Enter o número: ejecutar  Esc: volver",
            Screen::MenusPersonalizados => {
                "↑↓: menú  Enter/Espacio: mostrar u ocultar  Esc: volver"
            }
            Screen::Calendario => {
                "h/l: día  j/k: semana  n/p: mes  t: hoy  Enter: abrir  Esc: volver"
            }
            Screen::Logros => "h/l: pestaña  j/k: mover  r: refrescar  Esc: volver",
            Screen::Notificaciones => "j/k: mover  m: leída  M: todas  Enter: abrir  Esc: volver",
            Screen::Tutorial => "←/→: paso  Enter: siguiente  Esc: salir",
            Screen::Records => "j/k: mover  Enter: abrir partido  r: recalcular  Esc: volver",
            Screen::TorneosList => {
                "n: nuevo  e: editar  d: borrar  Enter: detalle  x: exportar  r: refrescar"
            }
            Screen::TorneoDetail(_) => {
                "g: fixture  r: resultado  a: equipos  e: editar  d: borrar  Esc: volver"
            }
            Screen::Ultimos => "j/k: mover  Enter: abrir partido  r: refrescar  Esc: volver",
            Screen::TemporadasList => {
                "n: nueva  e: editar  d: borrar  Enter: detalle  r: refrescar"
            }
            Screen::TemporadaDetail(_) => {
                "e: editar  d: borrar  c: comparar  ↑↓: desplazar  Esc: volver"
            }
            Screen::AnalisisMenu => "↑↓: opción  Enter: abrir  Esc: volver",
            Screen::Sincronizacion => "↑↓: opción  Enter o 1-8: ejecutar  Esc: volver",
            Screen::SyncConflicto => "1: conservar local  2: conservar servidor  Esc: volver",
            Screen::AnalisisBasico => "↑↓: desplazar  Esc: volver",
            Screen::AnalisisEvolucion => {
                "m: métrica (goles/asistencias/rendimiento)  ↑↓: desplazar  Esc: volver"
            }
            Screen::Quimica => "n: nueva dupla  e: editar  d: borrar  r: refrescar  Esc: volver",
            Screen::Comparador => {
                "1/2: tipo  Tab: A/B  j/k: mover  Enter: comparar  r: recargar  Esc: volver"
            }
            Screen::TiendasList => "n: nueva  e: editar  d: borrar  r: refrescar  Esc: volver",
            Screen::BienestarList => {
                "Tab/←→: sección  n: nuevo  e: editar  d: borrar  o: omitido  v: ver archivo  a: asistente  r: refrescar"
            }
            Screen::BienestarAsistente => "s: guardar recomendación  r: recalcular  Esc: volver",
            Screen::ColeccionesList => {
                "Tab: pestaña  n: nuevo  e: editar  d: borrar  s: sincronizar camisetas  a: agregar a colección  Enter: abrir  x/i: backup JSON exportar/importar"
            }
            Screen::ColeccionDetail(_) => "u: quitar de la colección  Esc: volver",
            Screen::EntrenadorIa => {
                "s: consejos seguidos  i: ignorados  v: nivel de intervención  r: refrescar  Esc: volver"
            }
            Screen::AmigosList => "n: nuevo  e: editar  d: borrar  r: refrescar  Esc: volver",
            Screen::MediaList => {
                "n: nueva  e: editar  d: borrar  f: tipo  r: refrescar  Esc: volver"
            }
            Screen::RecordatoriosList => {
                "n: nuevo  e: editar  d: borrar  c: completar  a: agenda  f: temática  x: exportar  i/I: importar (unir/reemplazar)  r: refrescar"
            }
            Screen::Agenda => "j/k: mover  Esc: volver",
            Screen::ProgresionList => "n: nueva  e: editar  d: borrar  p: planes  r: refrescar",
            Screen::PlanesList => "n: nuevo  e: editar  d: borrar  r: refrescar  Esc: volver",
            Screen::Bienvenida => {
                "↑↓: elegir cuenta  Enter: entrar  n: crear cuenta  l: iniciar sesión  i: invitado  q/Esc: salir"
            }
            Screen::UsuarioList => {
                "n: nueva  l: iniciar  p: contraseña  d: borrar  c: cerrar sesión"
            }
            Screen::UsuarioLogin => "Enter: iniciar sesión  Esc: volver",
            Screen::Ayuda | Screen::ImagenViewer => "Esc: volver",
        }
    }

    /// Opciones visibles del menú según el modo (`create_filtered_menu` de C).
    #[must_use]
    pub fn menu_modulos(&self) -> Vec<Module> {
        Module::ALL
            .into_iter()
            .filter(|module| match self.settings.mode {
                Mode::Simple => module.en_modo_simple(),
                Mode::Advanced => true,
                Mode::Custom => {
                    matches!(module, Module::Ajustes | Module::Sincronizacion)
                        || !self.menu_ocultos.iter().any(|clave| clave == module.key())
                }
            })
            .collect()
    }

    /// Módulo seleccionado en el menú.
    #[must_use]
    pub fn selected_module(&self) -> Module {
        let modulos = self.menu_modulos();
        let indice = self.menu_state.selected().unwrap_or(0);
        modulos
            .get(indice)
            .or(modulos.last())
            .copied()
            .unwrap_or(Module::Ajustes)
    }

    /// Entra a una pantalla.
    pub fn push_screen(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    /// Vuelve atrás.
    pub fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            if self.current_screen() == Screen::MainMenu {
                self.active_module = None;
                // Recalcular (33 ms con 20k partidos) solo si hubo escrituras desde el último.
                if self.inicio_sucio || self.db.conn().total_changes() != self.inicio_cambios {
                    let _ = self.refrescar_inicio();
                }
            }
            self.status = "Volvió a la pantalla anterior".to_string();
        }
    }

    /// Abre el visor de imagen si la ruta se resuelve.
    pub fn abrir_imagen(&mut self, ruta: &str) {
        if let Some(ruta) = ImageCache::resolver(ruta, &self.paths.images_dir, &self.paths.data_dir)
        {
            self.imagen_abierta = Some(ruta.to_string_lossy().into_owned());
            self.push_screen(Screen::ImagenViewer);
        } else {
            self.status = "No hay imagen disponible".to_string();
        }
    }

    /// Procesa eventos de ratón: rueda para navegar y clic en el sidebar.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        if self.modal.is_some() {
            return Ok(());
        }
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            let posicion = Position::new(mouse.column, mouse.row);
            if let Some(module) = self
                .sidebar_hits
                .iter()
                .find(|(area, _)| area.contains(posicion))
                .map(|(_, module)| *module)
            {
                self.switch_module(module);
                return Ok(());
            }
            if self.current_screen() == Screen::MainMenu
                && let Some(indice) = self
                    .menu_hits
                    .iter()
                    .find(|(area, _)| area.contains(posicion))
                    .map(|(_, indice)| *indice)
            {
                self.menu_state.select(Some(indice));
                self.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
                return Ok(());
            }
            if let Some(indice) = self
                .table_hits
                .iter()
                .find(|(area, _)| area.contains(posicion))
                .map(|(_, indice)| *indice)
            {
                match self.table_hit_target {
                    Some(TableHitTarget::Camisetas) if indice < self.camisetas.len() => {
                        self.camisetas_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Partidos) if indice < self.partidos.len() => {
                        self.partidos_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Canchas) if indice < self.canchas.len() => {
                        self.canchas_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Botines) if indice < self.botines.len() => {
                        self.botines_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Equipos) if indice < self.equipos.len() => {
                        self.equipos_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Lesiones) if indice < self.lesiones.len() => {
                        self.lesiones_table.select(Some(indice));
                    }
                    Some(TableHitTarget::Financiamiento) if indice < self.transacciones.len() => {
                        self.transacciones_table.select(Some(indice));
                    }
                    _ => {}
                }
            }
            return Ok(());
        }
        let code = match mouse.kind {
            MouseEventKind::ScrollDown => KeyCode::Down,
            MouseEventKind::ScrollUp => KeyCode::Up,
            _ => return Ok(()),
        };
        self.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    /// Cambia de módulo desde el sidebar reiniciando la pila de pantallas.
    pub fn switch_module(&mut self, module: Module) {
        if self.active_module == Some(module) {
            return;
        }
        self.stack = vec![Screen::MainMenu];
        self.active_module = Some(module);
        if let Err(error) = self.open_module(module) {
            self.modal = Some(Modal::Error(error.to_string()));
        }
    }

    /// Procesa una tecla.
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        let resultado = self.procesar_tecla(key);
        // Recalcular el Inicio (33 ms con 20k partidos) solo si se está viendo.
        if self.inicio_sucio && self.current_screen() == Screen::MainMenu {
            let _ = self.refrescar_inicio();
        }
        resultado
    }

    fn procesar_tecla(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        if self.modal.is_some() {
            self.handle_modal_key(key);
            return Ok(());
        }
        if self.overlay.is_some() {
            self.handle_overlay_key(key)?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.save_current_form()?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('z')
            && self.form.is_none()
        {
            self.solicitar_undo()?;
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
                KeyCode::Char('g') => {
                    self.open_backups();
                    return Ok(());
                }
                KeyCode::Char('d') => {
                    self.open_exportar_importar();
                    return Ok(());
                }
                KeyCode::Char('t') => {
                    self.modal = Some(Modal::confirm(
                        "Mantenimiento de la base",
                        "Se creará un backup y luego VACUUM, ANALYZE e integridad. (S = sí, N = no)",
                        ConfirmAction::Mantenimiento,
                    ));
                    return Ok(());
                }
                KeyCode::Char('x') => {
                    self.exportar_todo_ajustes();
                    return Ok(());
                }
                KeyCode::Char('p') => {
                    self.exportar_pdf_ajustes();
                    return Ok(());
                }
                KeyCode::Char('o') => {
                    self.exportar_ods_ajustes();
                    return Ok(());
                }
                KeyCode::Char('w') => {
                    self.exportar_mobile_ajustes();
                    return Ok(());
                }
                KeyCode::Char('y') => {
                    self.importar_mobile_ajustes();
                    return Ok(());
                }
                KeyCode::Char('a') => {
                    self.alternar_mobile_auto_ajustes();
                    return Ok(());
                }
                KeyCode::Char('n') => {
                    self.push_screen(Screen::MenusPersonalizados);
                    return Ok(());
                }
                KeyCode::Char('u') => {
                    self.buscar_actualizaciones(true);
                    return Ok(());
                }
                KeyCode::Char('f') => {
                    self.modal = Some(Modal::Info(self.configuracion_actual()));
                    return Ok(());
                }
                KeyCode::Char('q') => {
                    self.modal = Some(Modal::confirm(
                        "Restablecer ajustes",
                        "¿Restablecer toda la configuración a los valores por defecto? (S = sí, N = no)",
                        ConfirmAction::RestablecerAjustes,
                    ));
                    return Ok(());
                }
                _ => {}
            }
        }
        if self.search.is_some() {
            self.handle_search_key(key)?;
            return Ok(());
        }
        if self.handle_overlay_shortcut(key)? {
            return Ok(());
        }
        if key.code == KeyCode::Char('x') && self.es_listado_exportable() {
            self.exportar_listado_actual()?;
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
            | Screen::LegacyImportForm
            | Screen::TorneoForm(_)
            | Screen::TorneoResultadoForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::BackupsList => self.handle_backups_key(key)?,
            Screen::ExportarImportar => self.handle_exportar_importar_key(key)?,
            Screen::MenusPersonalizados => self.handle_menus_personalizados_key(key)?,
            Screen::CanchasList => self.handle_canchas_list_key(key)?,
            Screen::CanchaDetail(_) => self.handle_cancha_detail_key(key)?,
            Screen::BotinesList => self.handle_botines_list_key(key)?,
            Screen::BotinDetail(_) => self.handle_botin_detail_key(key)?,
            Screen::EquiposList => self.handle_equipos_list_key(key)?,
            Screen::EquipoDetail(_) => self.handle_equipo_detail_key(key)?,
            Screen::PartidosList => self.handle_partidos_list_key(key)?,
            Screen::PartidoDetail(_) => self.handle_partido_detail_key(key)?,
            Screen::PartidoWizard => self.handle_partido_wizard_key(key)?,
            Screen::ImagenViewer => {
                if key.code == KeyCode::Esc {
                    self.imagen_abierta = None;
                    self.go_back();
                    true
                } else {
                    false
                }
            }
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
            Screen::Calendario => self.handle_calendario_key(key)?,
            Screen::Logros => self.handle_logros_key(key)?,
            Screen::Notificaciones => self.handle_notificaciones_key(key)?,
            Screen::Tutorial => self.handle_tutorial_key(key)?,
            Screen::Records => self.handle_records_key(key)?,
            Screen::TorneosList => self.handle_torneos_list_key(key)?,
            Screen::TorneoDetail(_) => self.handle_torneo_detail_key(key)?,
            Screen::TemporadaForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::TemporadasList => self.handle_temporadas_list_key(key)?,
            Screen::TemporadaDetail(_) => self.handle_temporada_detail_key(key)?,
            Screen::TiendasList => self.handle_tiendas_list_key(key)?,
            Screen::BienestarList => self.handle_bienestar_list_key(key)?,
            Screen::BienestarAsistente => self.handle_bienestar_asistente_key(key)?,
            Screen::ColeccionesList => self.handle_colecciones_list_key(key)?,
            Screen::ColeccionDetail(id) => self.handle_coleccion_detail_key(id, key)?,
            Screen::EntrenadorIa => self.handle_entrenador_key(key)?,
            Screen::AmigosList => self.handle_amigos_list_key(key)?,
            Screen::MediaList => self.handle_media_list_key(key)?,
            Screen::RecordatoriosList => self.handle_recordatorios_list_key(key)?,
            Screen::Agenda => self.handle_agenda_key(key)?,
            Screen::ProgresionList => self.handle_progresion_list_key(key)?,
            Screen::PlanesList => self.handle_planes_list_key(key)?,
            Screen::Bienvenida => self.handle_bienvenida_key(key)?,
            Screen::UsuarioList => self.handle_usuario_list_key(key)?,
            Screen::Ultimos => self.handle_ultimos_key(key)?,
            Screen::AnalisisMenu => self.handle_analisis_menu_key(key)?,
            Screen::Sincronizacion => self.handle_sincronizacion_key(key)?,
            Screen::SyncConflicto => self.handle_sync_conflicto_key(key)?,
            Screen::Comparador => self.handle_comparador_key(key)?,
            Screen::AnalisisBasico | Screen::AnalisisEvolucion => {
                self.handle_analisis_reporte_key(key)?
            }
            Screen::Quimica => self.handle_quimica_key(key)?,
            Screen::TiendaForm(_)
            | Screen::AmigoForm(_)
            | Screen::SyncConfigForm
            | Screen::SyncLoginForm
            | Screen::QuimicaForm(_)
            | Screen::BienestarForm(_)
            | Screen::ColeccionForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::MediaForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::RecordatorioForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::ProgresionForm(_) | Screen::PlanForm(_) => {
                self.handle_form_key(key)?;
                true
            }
            Screen::UsuarioForm(_) | Screen::UsuarioEliminar => {
                self.handle_form_key(key)?;
                true
            }
            Screen::UsuarioLogin => self.handle_usuario_login_key(key)?,
            Screen::Ayuda => false,
        };
        if !consumed {
            self.handle_common_key(key)?;
        }
        Ok(())
    }

    fn handle_common_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Fuera del menú, `q`/Esc vuelven; en el menú piden confirmar la salida.
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            KeyCode::Char('?') => self.push_screen(Screen::Ayuda),
            KeyCode::Char('g') => self.ir_a_inicio()?,
            KeyCode::Char('!') => self.open_notificaciones()?,
            // Los atajos de letra solo valen en el menú: dentro de un módulo la
            // letra puede significar otra cosa y no debe saltar a otra pantalla.
            _ if self.current_screen() != Screen::MainMenu => {}
            KeyCode::Char('c') => self.open_calendario()?,
            KeyCode::Char('l') => self.open_logros()?,
            KeyCode::Char('n') => self.abrir_nuevo_partido()?,
            KeyCode::Char('t') => self.open_tutorial()?,
            KeyCode::Char('R') => self.open_records()?,
            KeyCode::Char('T') => self.open_torneos()?,
            KeyCode::Char('U') => self.open_ultimos()?,
            KeyCode::Char('C') => self.open_comparador()?,
            KeyCode::Char('E') => self.open_temporadas()?,
            KeyCode::Char('D') => self.open_tiendas()?,
            KeyCode::Char('A') => self.open_amigos()?,
            KeyCode::Char('M') => self.open_media()?,
            KeyCode::Char('Y') => self.open_recordatorios()?,
            KeyCode::Char('P') => self.open_progresion()?,
            KeyCode::Char('L') => self.open_logros()?,
            _ => {}
        }
        Ok(())
    }

    fn handle_main_menu_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let KeyCode::Char(digito) = key.code
            && digito.is_ascii_digit()
        {
            self.teclear_numero_menu(digito)?;
            return Ok(true);
        }
        // Otra tecla corta el número en curso: Enter lo abre y Esc lo descarta.
        if let Some((numero, _)) = self.menu_numero.take() {
            match key.code {
                KeyCode::Enter => {
                    self.abrir_numero_menu(&numero)?;
                    return Ok(true);
                }
                KeyCode::Esc => {
                    self.status = "Número descartado".to_string();
                    return Ok(true);
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_menu_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_menu_selection(-1),
            KeyCode::Enter => {
                let module = self.selected_module();
                self.open_module(module)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Suma un dígito al número tecleado: abre la opción en cuanto el número
    /// deja de ser prefijo de otra visible (con 1..28, "3" abre ya y "1" espera).
    fn teclear_numero_menu(&mut self, digito: char) -> Result<()> {
        let mut numero = self
            .menu_numero
            .take()
            .map(|(numero, _)| numero)
            .unwrap_or_default();
        numero.push(digito);
        if numero == "0" {
            self.request_quit();
            return Ok(());
        }
        let modulos = self.menu_modulos();
        let ambiguo = modulos.iter().any(|module| {
            let otro = module.numero().to_string();
            otro.len() > numero.len() && otro.starts_with(&numero)
        });
        if !ambiguo {
            return self.abrir_numero_menu(&numero);
        }
        if let Some(posicion) = modulos
            .iter()
            .position(|module| module.numero().to_string() == numero)
        {
            self.menu_state.select(Some(posicion));
        }
        self.status = format!("Opción {numero}… (Enter abre)");
        self.menu_numero = Some((numero, std::time::Instant::now()));
        Ok(())
    }

    fn abrir_numero_menu(&mut self, numero: &str) -> Result<()> {
        let modulos = self.menu_modulos();
        match modulos
            .iter()
            .position(|module| module.numero().to_string() == numero)
        {
            Some(posicion) => {
                self.menu_state.select(Some(posicion));
                self.open_module(modulos[posicion])
            }
            None => {
                self.status = format!(
                    "La opción {numero} no está disponible en el modo {}",
                    self.settings.mode.label()
                );
                Ok(())
            }
        }
    }

    /// Abre el número en espera cuando pasó [`ESPERA_NUMERO_MENU`] desde el
    /// último dígito. Devuelve `true` si había uno vencido.
    pub fn vencer_numero_menu(&mut self, ahora: std::time::Instant) -> bool {
        let vencido = self
            .menu_numero
            .as_ref()
            .is_some_and(|(_, desde)| ahora.duration_since(*desde) >= ESPERA_NUMERO_MENU);
        if !vencido {
            return false;
        }
        if let Some((numero, _)) = self.menu_numero.take()
            && self.current_screen() == Screen::MainMenu
            && self.modal.is_none()
            && let Err(error) = self.abrir_numero_menu(&numero)
        {
            self.modal = Some(Modal::Error(error.to_string()));
        }
        true
    }

    fn handle_camisetas_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.camisetas_table, self.camisetas.len(), 1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.camisetas_table, self.camisetas.len(), -1)
            }
            KeyCode::Enter => self.open_camiseta_detail()?,
            KeyCode::Char('n') => self.open_camiseta_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_camiseta_id() {
                    self.open_camiseta_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_camiseta(),
            KeyCode::Char('a') => self.toggle_camiseta_activa()?,
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
            KeyCode::Char('a') => self.toggle_camiseta_activa()?,
            KeyCode::Char('v') => {
                if let Some(camiseta) = &self.camiseta_detail {
                    let ruta = camiseta.imagen_ruta.clone();
                    self.abrir_imagen(&ruta);
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn handle_form_key(&mut self, key: KeyEvent) -> Result<()> {
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
            ConfirmAction::DeleteTorneo(id) => {
                let resultado = crate::services::torneos_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, |app| {
                    app.go_back();
                    app.refresh_torneos()
                });
            }
            ConfirmAction::DeleteTienda(id) => {
                let resultado = crate::services::tiendas_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_tiendas);
            }
            ConfirmAction::DeleteBienestar(id) => self.eliminar_bienestar(id),
            ConfirmAction::DeleteInventarioItem(id) => self.eliminar_inventario_item(id),
            ConfirmAction::DeleteColeccion(id) => self.eliminar_coleccion(id),
            ConfirmAction::DeleteProgresion(id) => {
                let resultado = crate::services::progresion_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_progresion);
            }
            ConfirmAction::DeleteUsuario(id) => self.confirmar_eliminar_usuario(id),
            ConfirmAction::DeletePlan(id) => {
                let resultado =
                    crate::services::progresion_service::eliminar_plan(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_planes);
            }
            ConfirmAction::DeleteRecordatorio(id) => {
                let resultado =
                    crate::services::recordatorios_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_recordatorios);
            }
            ConfirmAction::DeleteMedia(id) => {
                let resultado = crate::services::media_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_media);
            }
            ConfirmAction::DeleteAmigo(id) => {
                let resultado = crate::services::amigos_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, App::refresh_amigos);
            }
            ConfirmAction::DeleteQuimica(id) => {
                let resultado = crate::services::quimica_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, |app| {
                    app.refresh_quimica();
                    Ok(())
                });
            }
            ConfirmAction::DeleteTemporada(id) => {
                let resultado = crate::services::temporadas_service::eliminar(self.db.conn(), id);
                self.confirmar_eliminacion(resultado, |app| {
                    app.go_back();
                    app.refresh_temporadas()
                });
            }
            ConfirmAction::DeleteCamiseta(id) => {
                let resultado = self.delete_camiseta(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteCancha(id) => {
                let resultado = self.delete_cancha(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteBotin(id) => {
                let resultado = self.delete_botin(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteEquipo(id) => {
                let resultado = self.delete_equipo(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteHito(id) => {
                let resultado = self.delete_carrera_hito(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteTransaccion(id) => {
                let resultado = self.delete_transaccion(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteLesion(id) => {
                let resultado = self.delete_lesion(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeletePartido(id) => {
                let resultado = self.delete_partido(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::DeleteJugador(id) => {
                let resultado = self.delete_jugador(id);
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::GenerarFixture(id) => {
                match crate::services::torneos_service::generar_fixture(self.db.conn(), id) {
                    Ok(cantidad) => {
                        self.status = format!("Fixture generado: {cantidad} partidos");
                        if let Err(error) = self.refresh_torneo_detail() {
                            self.modal = Some(Modal::Error(error.to_string()));
                        }
                    }
                    Err(error) => {
                        self.modal = Some(Modal::Error(error.to_string()));
                    }
                }
            }
            ConfirmAction::Undo => {
                match crate::services::undo_service::deshacer_ultima(self.db.conn()) {
                    Ok(Some(descripcion)) => {
                        self.status = format!("Deshecho: {descripcion}");
                        self.refrescar_pantalla_actual();
                    }
                    Ok(None) => {
                        self.status = "No hay operaciones para deshacer".to_string();
                    }
                    Err(error) => {
                        self.modal = Some(Modal::Error(format!("No se pudo deshacer: {error}")));
                    }
                }
            }
            ConfirmAction::RestoreBackup => {
                let resultado = self.restaurar_backup();
                self.confirmar_eliminacion(resultado, |_| Ok(()));
            }
            ConfirmAction::RestoreBackupSel(indice) => {
                if let Some(ruta) = self.backups.get(indice).map(|b| b.ruta.clone()) {
                    self.restaurar_backup_desde(&ruta);
                    self.refresh_backups();
                }
            }
            ConfirmAction::Mantenimiento => self.mantenimiento_ajustes(),
            ConfirmAction::ImportarRecordatoriosReemplazo => self.importar_recordatorios(true),
            ConfirmAction::DeleteBackupSel(indice) => {
                if let Some(ruta) = self.backups.get(indice).map(|b| b.ruta.clone()) {
                    let resultado = crate::services::backup_service::eliminar(&ruta);
                    self.confirmar_eliminacion(resultado, |app| {
                        app.refresh_backups();
                        app.status = "Backup eliminado".to_string();
                        Ok(())
                    });
                }
            }
            ConfirmAction::ResetSorteoBotines => {
                let resultado = botin_service::reiniciar_sorteo(self.db.conn());
                self.confirmar_eliminacion(resultado, |app| {
                    app.status = "Sorteo de botines reiniciado".to_string();
                    Ok(())
                });
            }
            ConfirmAction::ResetSorteo => {
                let resultado = camiseta_service::reiniciar_sorteo(self.db.conn());
                self.confirmar_eliminacion(resultado, |app| {
                    app.status = "Sorteo reiniciado".to_string();
                    Ok(())
                });
            }
            ConfirmAction::AbrirModoRetro => {
                if let Err(error) = self.open_module(Module::Carrera) {
                    self.modal = Some(Modal::Error(error.to_string()));
                } else {
                    self.push_screen(Screen::CarreraRetro);
                }
            }
            ConfirmAction::RestablecerAjustes => self.restablecer_ajustes(),
            ConfirmAction::Quit => self.should_quit = true,
            ConfirmAction::DiscardForm => self.discard_form(),
        }
    }

    /// Mueve la selección de una tabla; al pasar un extremo vuelve al otro.
    pub(crate) fn mover_tabla_seleccion(state: &mut TableState, total: usize, delta: isize) {
        state.select(mover_seleccion(state.selected(), total, delta));
    }

    pub(crate) fn confirmar_eliminacion<T, E, F>(&mut self, resultado: Result<T, E>, despues: F)
    where
        E: std::fmt::Display,
        F: FnOnce(&mut Self) -> Result<()>,
    {
        let error = match resultado {
            Ok(_) => despues(self).err().map(|error| error.to_string()),
            Err(error) => Some(error.to_string()),
        };
        if let Some(error) = error {
            self.modal = Some(Modal::Error(error));
        }
    }

    /// Cierre común de un borrado: ante error muestra el modal; si salió bien deja
    /// `mensaje` en la barra, sale de la ficha (si `es_ficha`) y llama a `recargar`.
    pub(crate) fn tras_eliminar<E, F>(
        &mut self,
        mensaje: Result<String, E>,
        es_ficha: fn(Screen) -> bool,
        recargar: F,
    ) -> Result<()>
    where
        E: std::fmt::Display,
        F: FnOnce(&mut Self) -> Result<()>,
    {
        match mensaje {
            Ok(mensaje) => self.status = mensaje,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        }
        if es_ficha(self.current_screen()) {
            self.stack.pop();
        }
        recargar(self)
    }

    fn move_menu_selection(&mut self, delta: isize) {
        let total = self.menu_modulos().len();
        self.menu_state
            .select(mover_seleccion(self.menu_state.selected(), total, delta));
    }

    /// En el menú o la portada pide confirmar la salida; en otra pantalla vuelve.
    pub(crate) fn request_quit(&mut self) {
        if matches!(self.current_screen(), Screen::MainMenu | Screen::Bienvenida) {
            self.modal = Some(Modal::confirm(
                "Salir",
                "¿Cerrar MiFutbolR? S = sí, N/Esc = no.",
                ConfirmAction::Quit,
            ));
        } else {
            self.go_back();
        }
    }

    /// Abre la opción del menú principal.
    pub fn open_module(&mut self, module: Module) -> Result<()> {
        self.active_module = Some(module);
        match module {
            Module::Calendario => self.open_calendario()?,
            Module::Logros => self.open_logros()?,
            Module::Records => self.open_records()?,
            Module::Tutorial => self.open_tutorial()?,
            Module::Analisis => self.push_screen(Screen::AnalisisMenu),
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
            Module::Sincronizacion => self.open_sincronizacion(),
            Module::Torneos => {
                self.push_screen(Screen::TorneosList);
                self.refresh_torneos()?;
            }
            Module::Temporadas => {
                self.push_screen(Screen::TemporadasList);
                self.refresh_temporadas()?;
            }
            Module::Tiendas => {
                self.push_screen(Screen::TiendasList);
                self.refresh_tiendas()?;
            }
            Module::Amigos => {
                self.push_screen(Screen::AmigosList);
                self.refresh_amigos()?;
            }
            Module::Media => {
                self.push_screen(Screen::MediaList);
                self.refresh_media()?;
            }
            Module::Recordatorios => {
                self.push_screen(Screen::RecordatoriosList);
                self.refresh_recordatorios()?;
            }
            Module::Progresion => {
                self.push_screen(Screen::ProgresionList);
                self.refresh_progresion()?;
            }
            Module::Bienestar => self.open_bienestar()?,
            Module::Colecciones => self.open_colecciones()?,
            Module::EntrenadorIa => self.open_entrenador()?,
            Module::Usuario => {
                self.push_screen(Screen::UsuarioList);
                self.refresh_usuarios()?;
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

    /// Reactiva o desactiva la camiseta (listado o detalle), como
    /// `reactivar_camiseta` de MiFutbolC: borrar una con historial la desactiva.
    fn toggle_camiseta_activa(&mut self) -> Result<()> {
        let actual = if matches!(self.current_screen(), Screen::CamisetaDetail(_)) {
            self.camiseta_detail
                .as_ref()
                .map(|camiseta| (camiseta.id, camiseta.activa))
        } else {
            self.camisetas_table
                .selected()
                .and_then(|indice| self.camisetas.get(indice))
                .map(|row| (row.id, row.activa))
        };
        let Some((id, activa)) = actual else {
            return Ok(());
        };
        match camiseta_service::set_activa(self.db.conn(), id, !activa) {
            Ok(()) => {
                if let Some(camiseta) = self.camiseta_detail.as_mut().filter(|c| c.id == id) {
                    camiseta.activa = !activa;
                }
                self.refresh_camisetas()?;
                self.status = if activa {
                    "Camiseta desactivada"
                } else {
                    "Camiseta reactivada"
                }
                .to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
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
        let mensaje =
            camiseta_service::eliminar(self.db.conn(), id).map(|resultado| match resultado {
                ResultadoEliminacion::Eliminada => "Camiseta eliminada".to_string(),
                ResultadoEliminacion::Desactivada(dependencias) => format!(
                    "Camiseta desactivada ({} partidos, {} lesiones)",
                    dependencias.partidos, dependencias.lesiones
                ),
            });
        self.tras_eliminar(
            mensaje,
            |pantalla| matches!(pantalla, Screen::CamisetaDetail(_)),
            |app| {
                app.camiseta_detail = None;
                app.camiseta_ficha = None;
                app.refresh_camisetas()
            },
        )
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
            Screen::PartidoForm(mode) => {
                let resultado = self.save_partido_form(mode);
                self.evaluar_logros_silencioso();
                resultado
            }
            Screen::PartidoWizard => {
                let resultado = self.save_partido_wizard();
                self.evaluar_logros_silencioso();
                resultado
            }
            Screen::LesionForm(mode) => self.save_lesion_form(mode),
            Screen::TransaccionForm(mode) => self.save_transaccion_form(mode),
            Screen::PresupuestoForm => self.save_presupuesto_form(),
            Screen::CarreraIdentidadForm => self.save_carrera_identidad(),
            Screen::CarreraHitoForm(mode) => self.save_carrera_hito(mode),
            Screen::TorneoForm(mode) => self.save_torneo_form(mode),
            Screen::TorneoResultadoForm(_) => self.save_torneo_resultado_form(),
            Screen::TemporadaForm(mode) => self.save_temporada_form(mode),
            Screen::TiendaForm(mode) => self.save_tienda_form(mode),
            Screen::BienestarForm(_) => self.save_bienestar_form(),
            Screen::ColeccionForm(_) => self.save_coleccion_form(),
            Screen::AmigoForm(mode) => self.save_amigo_form(mode),
            Screen::SyncConfigForm => self.save_sync_config_form(),
            Screen::SyncLoginForm => self.save_sync_login_form(),
            Screen::QuimicaForm(mode) => self.save_quimica_form(mode),
            Screen::MediaForm(mode) => self.save_media_form(mode),
            Screen::RecordatorioForm(mode) => self.save_recordatorio_form(mode),
            Screen::ProgresionForm(mode) => self.save_progresion_form(mode),
            Screen::PlanForm(mode) => self.save_plan_form(mode),
            Screen::UsuarioForm(mode) => self.save_usuario_form(mode),
            Screen::UsuarioLogin => self.save_usuario_login(),
            Screen::UsuarioEliminar => self.save_usuario_eliminar(),
            Screen::AjustesForm => self.save_ajustes_form(),
            Screen::LegacyImportForm => self.save_legacy_import(),
            Screen::ImagenViewer => Ok(()),
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
    // Solo se redibuja si llegó un evento que cambió algo o hay una animación
    // en curso: en reposo la app no consume CPU (antes redibujaba cada 120 ms).
    let mut redibujar = true;
    loop {
        redibujar |= app.tick_clima();
        redibujar |= app.tick_sync();
        redibujar |= app.tick_actualizacion();
        redibujar |= app.vencer_numero_menu(std::time::Instant::now());
        if redibujar || app.animations.running() {
            terminal.draw(|frame| crate::ui::draw(frame, app))?;
        }
        redibujar = false;
        let espera = if app.animations.running() {
            Duration::from_millis(16)
        } else if app.menu_numero.is_some() {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(1)
        };
        if event::poll(espera)? {
            redibujar = procesar_eventos(app, || {
                Ok(if event::poll(Duration::ZERO)? {
                    Some(event::read()?)
                } else {
                    None
                })
            })?;
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

/// Máximo de eventos procesados entre dos frames: un flujo continuo de
/// eventos no puede dejar la pantalla sin actualizar.
pub const EVENTOS_POR_FRAME: usize = 64;

/// Procesa los eventos ya disponibles (`siguiente` devuelve `None` cuando no
/// queda ninguno) para dibujar una sola vez por ráfaga. Devuelve `true` si
/// alguno puede haber cambiado lo que se ve.
pub fn procesar_eventos(
    app: &mut App,
    mut siguiente: impl FnMut() -> Result<Option<Event>>,
) -> Result<bool> {
    let mut cambio = false;
    for _ in 0..EVENTOS_POR_FRAME {
        let Some(evento) = siguiente()? else {
            break;
        };
        match evento {
            Event::Key(key) => app.handle_key(key)?,
            // Mover el mouse no cambia el estado: no merece un frame.
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved => continue,
            Event::Mouse(mouse) => app.handle_mouse(mouse)?,
            // Resize, foco, pegado: ratatui no redibuja solo al cambiar el tamaño.
            _ => {}
        }
        cambio = true;
        if app.should_quit {
            break;
        }
    }
    Ok(cambio)
}

/// Índice siguiente en una lista de `total` filas: al pasar un extremo vuelve al otro.
pub(crate) fn mover_seleccion(actual: Option<usize>, total: usize, delta: isize) -> Option<usize> {
    if total == 0 {
        return None;
    }
    let actual = actual.unwrap_or(0).min(total - 1) as isize;
    Some((actual + delta).rem_euclid(total as isize) as usize)
}

#[cfg(test)]
mod tests {
    use super::mover_seleccion;

    #[test]
    fn mover_seleccion_da_la_vuelta_y_lista_vacia_queda_sin_seleccion() {
        assert_eq!(mover_seleccion(Some(2), 3, 1), Some(0));
        assert_eq!(mover_seleccion(Some(0), 3, -1), Some(2));
        assert_eq!(mover_seleccion(Some(0), 3, 1), Some(1));
        assert_eq!(mover_seleccion(None, 0, 1), None);
        assert_eq!(mover_seleccion(Some(4), 0, -1), None);
    }
}
