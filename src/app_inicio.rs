//! Panel Hoy, calendario, logros, notificaciones y overlays (Ola 0).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    ACCIONES_PALETA, App, BusquedaState, Modal, Module, Overlay, PaletaState, Screen,
};
use crate::db::{calendario_repo, notificacion_repo};
use crate::domain::busqueda::TipoResultado;
use crate::domain::calendario::EventoTipo;
use crate::services::{
    busqueda_service, camiseta_service, cancha_service, equipo_service, lesion_service,
    logros_service, notificacion_service, partido_service,
};

impl App {
    /// Recarga el panel "Hoy".
    pub(crate) fn refrescar_inicio(&mut self) -> Result<()> {
        match crate::services::inicio_service::resumen(self.db.conn()) {
            Ok(resumen) => self.inicio = resumen,
            Err(error) => self.status = format!("No se pudo cargar el inicio: {error}"),
        }
        self.no_leidas = notificacion_service::no_leidas(self.db.conn()).unwrap_or(0);
        Ok(())
    }

    /// Vuelve al panel principal.
    pub(crate) fn ir_a_inicio(&mut self) -> Result<()> {
        self.stack = vec![Screen::MainMenu];
        self.active_module = None;
        self.overlay = None;
        self.refrescar_inicio()?;
        Ok(())
    }

    /// Abre el asistente de alta de partido desde cualquier pantalla.
    pub(crate) fn abrir_nuevo_partido(&mut self) -> Result<()> {
        self.open_module(Module::Partidos)?;
        self.open_partido_wizard()
    }

    // ------------------------------------------------------------------
    // Calendario
    // ------------------------------------------------------------------

    /// Abre o refresca el calendario.
    pub(crate) fn open_calendario(&mut self) -> Result<()> {
        self.refrescar_calendario()?;
        if !matches!(self.current_screen(), Screen::Calendario) {
            self.push_screen(Screen::Calendario);
        }
        Ok(())
    }

    /// Recarga los eventos del mes visible.
    pub(crate) fn refrescar_calendario(&mut self) -> Result<()> {
        match calendario_repo::eventos_mes(
            self.db.conn(),
            self.calendario.year,
            self.calendario.month,
        ) {
            Ok(eventos) => self.calendario.eventos = eventos,
            Err(error) => self.status = format!("No se pudo cargar el calendario: {error}"),
        }
        Ok(())
    }

    /// Teclas del calendario.
    pub(crate) fn handle_calendario_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.calendario.mover_dia(-1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.calendario.mover_dia(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.calendario.mover_dia(7);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.calendario.mover_dia(-7);
            }
            KeyCode::Char('n') => {
                self.calendario.mover_mes(1);
                self.refrescar_calendario()?;
            }
            KeyCode::Char('p') => {
                self.calendario.mover_mes(-1);
                self.refrescar_calendario()?;
            }
            KeyCode::Char('t') => {
                let hoy = chrono::Local::now().date_naive();
                use chrono::Datelike;
                self.calendario.year = hoy.year();
                self.calendario.month = hoy.month();
                self.calendario.dia = hoy.day();
                self.refrescar_calendario()?;
            }
            KeyCode::Enter => self.abrir_evento_del_dia()?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn abrir_evento_del_dia(&mut self) -> Result<()> {
        let evento = self
            .calendario
            .eventos_del_dia()
            .into_iter()
            .next()
            .cloned();
        let Some(evento) = evento else {
            self.status = "No hay eventos ese día".to_string();
            return Ok(());
        };
        let Some(id) = evento.referencia else {
            return Ok(());
        };
        match evento.tipo {
            EventoTipo::Partido => self.navegar_a(TipoResultado::Partido, id),
            EventoTipo::Financiamiento => self.open_module(Module::Financiamiento),
            EventoTipo::Recordatorio => {
                self.status = "El módulo de recordatorios todavía no está disponible".to_string();
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Logros
    // ------------------------------------------------------------------

    /// Abre el catálogo de logros.
    pub(crate) fn open_logros(&mut self) -> Result<()> {
        self.refrescar_logros()?;
        if !matches!(self.current_screen(), Screen::Logros) {
            self.push_screen(Screen::Logros);
        }
        Ok(())
    }

    /// Recarga logros y registra los nuevos.
    pub(crate) fn refrescar_logros(&mut self) -> Result<()> {
        match logros_service::listar(self.db.conn(), None) {
            Ok(progresos) => match logros_service::registrar_nuevos(self.db.conn(), &progresos) {
                Ok(nuevos) => {
                    if !nuevos.is_empty() {
                        let _ = notificacion_service::notificar_logros(self.db.conn(), &nuevos);
                        self.status = format!("{} logro(s) desbloqueado(s)", nuevos.len());
                    }
                    self.logros = logros_service::listar(self.db.conn(), None).unwrap_or(progresos);
                }
                Err(error) => {
                    self.logros = progresos;
                    self.status = format!("No se pudieron guardar los logros: {error}");
                }
            },
            Err(error) => self.status = format!("No se pudieron cargar los logros: {error}"),
        }
        self.no_leidas = notificacion_service::no_leidas(self.db.conn()).unwrap_or(0);
        self.ajustar_seleccion_logros();
        Ok(())
    }

    fn ajustar_seleccion_logros(&mut self) {
        let total = self.logros_filtrados().len();
        if total == 0 {
            self.logros_table.select(None);
        } else {
            let indice = self.logros_table.selected().unwrap_or(0).min(total - 1);
            self.logros_table.select(Some(indice));
        }
    }

    /// Logros visibles según la pestaña activa.
    #[must_use]
    pub fn logros_filtrados(&self) -> Vec<&crate::domain::logros::LogroProgreso> {
        self.logros
            .iter()
            .filter(|logro| self.logros_tab.incluye(logro))
            .collect()
    }

    /// Teclas del catálogo de logros.
    pub(crate) fn handle_logros_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.logros_tab = self.logros_tab.previous();
                self.logros_table.select(Some(0));
                self.ajustar_seleccion_logros();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.logros_tab = self.logros_tab.next();
                self.logros_table.select(Some(0));
                self.ajustar_seleccion_logros();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.logros_filtrados().len();
                if total > 0 {
                    let indice = (self.logros_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.logros_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.logros_table.selected().unwrap_or(0).saturating_sub(1);
                self.logros_table.select(Some(indice));
            }
            KeyCode::Char('r') => {
                self.refrescar_logros()?;
                self.status = "Logros actualizados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    // ------------------------------------------------------------------
    // Notificaciones
    // ------------------------------------------------------------------

    /// Abre la bandeja de notificaciones.
    pub(crate) fn open_notificaciones(&mut self) -> Result<()> {
        let _ = notificacion_service::generar(self.db.conn());
        self.refrescar_notificaciones();
        if !matches!(self.current_screen(), Screen::Notificaciones) {
            self.push_screen(Screen::Notificaciones);
        }
        Ok(())
    }

    /// Recarga la bandeja.
    pub(crate) fn refrescar_notificaciones(&mut self) {
        self.notificaciones =
            notificacion_service::listar(self.db.conn(), false).unwrap_or_default();
        self.no_leidas = notificacion_service::no_leidas(self.db.conn()).unwrap_or(0);
        if self.notificaciones.is_empty() {
            self.notificaciones_table.select(None);
        } else {
            self.notificaciones_table.select(Some(0));
        }
    }

    /// Teclas de la bandeja.
    pub(crate) fn handle_notificaciones_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.notificaciones.len();
                if total > 0 {
                    let indice =
                        (self.notificaciones_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.notificaciones_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self
                    .notificaciones_table
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                self.notificaciones_table.select(Some(indice));
            }
            KeyCode::Char('m') => {
                if let Some(notificacion) = self
                    .notificaciones_table
                    .selected()
                    .and_then(|indice| self.notificaciones.get(indice))
                {
                    let id = notificacion.id;
                    let _ = notificacion_repo::marcar_leida(self.db.conn(), id);
                    self.refrescar_notificaciones();
                }
            }
            KeyCode::Char('M') => {
                let _ = notificacion_service::marcar(self.db.conn(), None);
                self.refrescar_notificaciones();
            }
            KeyCode::Enter => {
                let notificacion = self
                    .notificaciones_table
                    .selected()
                    .and_then(|indice| self.notificaciones.get(indice))
                    .cloned();
                if let Some(notificacion) = notificacion {
                    self.abrir_notificacion(&notificacion)?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn abrir_notificacion(
        &mut self,
        notificacion: &crate::domain::notificacion::Notificacion,
    ) -> Result<()> {
        match (notificacion.entidad.as_str(), notificacion.entidad_id) {
            ("partido", Some(id)) => self.navegar_a(TipoResultado::Partido, id),
            ("lesion", Some(id)) => self.navegar_a(TipoResultado::Lesion, id),
            ("presupuesto", _) => self.open_module(Module::Financiamiento),
            ("logro", _) => self.open_logros(),
            ("racha", _) => self.open_notificaciones(),
            _ => {
                self.status = notificacion.mensaje.clone();
                Ok(())
            }
        }
    }

    /// Evalúa logros y refresca el panel sin abrir pantallas.
    pub(crate) fn evaluar_logros_silencioso(&mut self) {
        if let Ok(progresos) = logros_service::listar(self.db.conn(), None)
            && let Ok(nuevos) = logros_service::registrar_nuevos(self.db.conn(), &progresos)
            && !nuevos.is_empty()
        {
            let _ = notificacion_service::notificar_logros(self.db.conn(), &nuevos);
            self.status = format!("{} logro(s) desbloqueado(s)", nuevos.len());
        }
        self.no_leidas = notificacion_service::no_leidas(self.db.conn()).unwrap_or(0);
        let _ = self.refrescar_inicio();
    }

    // ------------------------------------------------------------------
    // Exportación de listados
    // ------------------------------------------------------------------

    /// ¿La pantalla actual es un listado exportable?
    #[must_use]
    pub(crate) fn es_listado_exportable(&self) -> bool {
        matches!(
            self.current_screen(),
            Screen::CamisetasList
                | Screen::CanchasList
                | Screen::BotinesList
                | Screen::EquiposList
                | Screen::PartidosList
                | Screen::LesionesList
                | Screen::FinanciamientoList
                | Screen::TorneosList
        )
    }

    /// Exporta el listado visible a CSV, JSON, HTML y Markdown.
    pub(crate) fn exportar_listado_actual(&mut self) -> Result<()> {
        let si = |valor: bool| if valor { "Sí" } else { "No" }.to_string();
        let (entidad, cabeceras, filas): (&str, Vec<&str>, Vec<Vec<String>>) =
            match self.current_screen() {
                Screen::CamisetasList => (
                    "camisetas",
                    vec![
                        "ID",
                        "Nombre",
                        "Activa",
                        "Partidos",
                        "Goles",
                        "Asistencias",
                        "Victorias",
                        "Empates",
                        "Derrotas",
                        "Imagen",
                    ],
                    self.camisetas
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.nombre.clone(),
                                si(fila.activa),
                                fila.partidos.to_string(),
                                fila.goles.to_string(),
                                fila.asistencias.to_string(),
                                fila.victorias.to_string(),
                                fila.empates.to_string(),
                                fila.derrotas.to_string(),
                                fila.imagen_ruta.clone(),
                            ]
                        })
                        .collect(),
                ),
                Screen::CanchasList => (
                    "canchas",
                    vec![
                        "ID",
                        "Nombre",
                        "Localidad",
                        "Activa",
                        "Partidos",
                        "Goles",
                        "Asistencias",
                        "Victorias",
                        "Empates",
                        "Derrotas",
                        "Rendimiento",
                    ],
                    self.canchas
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.nombre.clone(),
                                fila.localidad.clone(),
                                si(fila.activa),
                                fila.partidos.to_string(),
                                fila.goles.to_string(),
                                fila.asistencias.to_string(),
                                fila.victorias.to_string(),
                                fila.empates.to_string(),
                                fila.derrotas.to_string(),
                                format!("{:.2}", fila.rendimiento_promedio),
                            ]
                        })
                        .collect(),
                ),
                Screen::BotinesList => (
                    "botines",
                    vec![
                        "ID",
                        "Nombre",
                        "Activo",
                        "Sorteada",
                        "Partidos",
                        "Predeterminado",
                    ],
                    self.botines
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.nombre.clone(),
                                si(fila.activa),
                                si(fila.sorteada),
                                fila.partidos.to_string(),
                                si(fila.predeterminado),
                            ]
                        })
                        .collect(),
                ),
                Screen::EquiposList => (
                    "equipos",
                    vec!["ID", "Nombre", "Tipo", "Modalidad", "Jugadores", "Activa"],
                    self.equipos
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.nombre.clone(),
                                fila.tipo.label().to_string(),
                                fila.tipo_futbol.label().to_string(),
                                format!("{} ({})", fila.jugadores_count, fila.num_jugadores),
                                si(fila.activa),
                            ]
                        })
                        .collect(),
                ),
                Screen::PartidosList => (
                    "partidos",
                    vec![
                        "ID",
                        "Fecha",
                        "Cancha",
                        "Camiseta",
                        "Rival",
                        "Goles",
                        "Asistencias",
                        "Resultado",
                        "Tipo",
                        "Precio",
                        "Favorito",
                    ],
                    self.partidos
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.fecha_hora.format("%Y-%m-%d %H:%M").to_string(),
                                fila.cancha.clone(),
                                fila.camiseta.clone(),
                                fila.rival.clone(),
                                fila.goles.to_string(),
                                fila.asistencias.to_string(),
                                fila.resultado.label().to_string(),
                                fila.tipo_partido.label().to_string(),
                                fila.precio.display_ars(),
                                si(fila.favorito),
                            ]
                        })
                        .collect(),
                ),
                Screen::LesionesList => (
                    "lesiones",
                    vec![
                        "ID", "Jugador", "Tipo", "Fecha", "Estado", "Camiseta", "Partido",
                    ],
                    self.lesiones
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.jugador.clone(),
                                fila.tipo.clone(),
                                fila.fecha.format("%Y-%m-%d").to_string(),
                                fila.estado.label().to_string(),
                                fila.camiseta.clone(),
                                fila.partido.clone(),
                            ]
                        })
                        .collect(),
                ),
                Screen::FinanciamientoList => (
                    "financiamiento",
                    vec![
                        "ID",
                        "Fecha",
                        "Tipo",
                        "Categoría",
                        "Descripción",
                        "Monto",
                        "Ítem",
                    ],
                    self.transacciones
                        .iter()
                        .map(|fila| {
                            vec![
                                fila.id.to_string(),
                                fila.fecha.format("%Y-%m-%d").to_string(),
                                fila.tipo.label().to_string(),
                                fila.categoria.label().to_string(),
                                fila.descripcion.clone(),
                                fila.monto.display_ars(),
                                fila.item_especifico.clone(),
                            ]
                        })
                        .collect(),
                ),
                Screen::TorneosList => (
                    "torneos",
                    vec!["ID", "Nombre", "Tipo", "Equipos", "Fase"],
                    self.torneos
                        .iter()
                        .map(|torneo| {
                            vec![
                                torneo.id.to_string(),
                                torneo.nombre.clone(),
                                torneo.tipo.label().to_string(),
                                torneo.cantidad_equipos.to_string(),
                                torneo.fase_actual.clone(),
                            ]
                        })
                        .collect(),
                ),
                _ => {
                    self.status = "Este listado no se puede exportar".to_string();
                    return Ok(());
                }
            };

        match crate::services::export_service::exportar(
            &self.paths.export_dir,
            entidad,
            &cabeceras,
            &filas,
        ) {
            Ok(rutas) => {
                self.status = format!(
                    "Exportados {} archivos en {}",
                    rutas.len(),
                    self.paths.export_dir.display()
                );
            }
            Err(error) => {
                self.status = format!("No se pudo exportar: {error}");
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Últimos
    // ------------------------------------------------------------------

    /// Abre la pantalla de últimos.
    pub(crate) fn open_ultimos(&mut self) -> Result<()> {
        self.refrescar_ultimos()?;
        if !matches!(self.current_screen(), Screen::Ultimos) {
            self.push_screen(Screen::Ultimos);
        }
        Ok(())
    }

    /// Recarga los últimos.
    pub(crate) fn refrescar_ultimos(&mut self) -> Result<()> {
        match crate::services::ultimos_service::listar(self.db.conn()) {
            Ok(ultimos) => {
                self.ultimos = ultimos;
                self.ultimos_table.select(Some(0));
            }
            Err(error) => self.status = format!("No se pudieron calcular los últimos: {error}"),
        }
        Ok(())
    }

    /// Teclas de últimos.
    pub(crate) fn handle_ultimos_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let indice = (self.ultimos_table.selected().unwrap_or(0) + 1)
                    .min(self.ultimos.len().saturating_sub(1));
                self.ultimos_table.select(Some(indice));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.ultimos_table.selected().unwrap_or(0).saturating_sub(1);
                self.ultimos_table.select(Some(indice));
            }
            KeyCode::Enter => {
                if let Some(partido_id) = self
                    .ultimos_table
                    .selected()
                    .and_then(|indice| self.ultimos.get(indice))
                    .and_then(|item| item.partido.as_ref())
                    .map(|partido| partido.id)
                {
                    self.navegar_a(TipoResultado::Partido, partido_id)?;
                }
            }
            KeyCode::Char('r') => {
                self.refrescar_ultimos()?;
                self.status = "Últimos recalculados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    // ------------------------------------------------------------------
    // Undo
    // ------------------------------------------------------------------

    /// Pide confirmación para deshacer la última operación.
    pub(crate) fn solicitar_undo(&mut self) -> Result<()> {
        match crate::services::undo_service::ultima(self.db.conn()) {
            Ok(Some(entrada)) => {
                self.modal = Some(Modal::confirm(
                    "Deshacer",
                    format!(
                        "{} · {}\n¿Deshacer esta operación?",
                        entrada.descripcion, entrada.ts
                    ),
                    crate::app::ConfirmAction::Undo,
                ));
            }
            Ok(None) => {
                self.status = "No hay operaciones para deshacer".to_string();
            }
            Err(error) => {
                self.modal = Some(Modal::Error(format!(
                    "No se pudo leer el historial: {error}"
                )));
            }
        }
        Ok(())
    }

    /// Refresca la pantalla actual después de un cambio externo (undo).
    pub(crate) fn refrescar_pantalla_actual(&mut self) {
        if matches!(
            self.current_screen(),
            Screen::CamisetaDetail(_)
                | Screen::CanchaDetail(_)
                | Screen::BotinDetail(_)
                | Screen::EquipoDetail(_)
                | Screen::PartidoDetail(_)
                | Screen::LesionDetail(_)
        ) {
            self.go_back();
        }
        match self.current_screen() {
            Screen::CamisetasList => {
                let _ = self.refresh_camisetas();
            }
            Screen::CanchasList => {
                let _ = self.refresh_canchas();
            }
            Screen::BotinesList => {
                let _ = self.refresh_botines();
            }
            Screen::EquiposList => {
                let _ = self.refresh_equipos();
            }
            Screen::PartidosList => {
                let _ = self.refresh_partidos();
            }
            Screen::LesionesList => {
                let _ = self.refresh_lesiones();
            }
            Screen::FinanciamientoList => {
                let _ = self.refresh_financiamiento();
            }
            Screen::Calendario => {
                let _ = self.refrescar_calendario();
            }
            Screen::Logros => {
                let _ = self.refrescar_logros();
            }
            Screen::Records => {
                let _ = self.refrescar_records();
            }
            Screen::Notificaciones => self.refrescar_notificaciones(),
            _ => {}
        }
        let _ = self.refrescar_inicio();
    }

    // ------------------------------------------------------------------
    // Récords y rankings
    // ------------------------------------------------------------------

    /// Abre los récords.
    pub(crate) fn open_records(&mut self) -> Result<()> {
        self.refrescar_records()?;
        if !matches!(self.current_screen(), Screen::Records) {
            self.push_screen(Screen::Records);
        }
        Ok(())
    }

    /// Recalcula los récords.
    pub(crate) fn refrescar_records(&mut self) -> Result<()> {
        match crate::services::records_service::listar(self.db.conn()) {
            Ok(records) => {
                self.records = records;
                self.records_table.select(if self.records.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron calcular los récords: {error}"),
        }
        Ok(())
    }

    /// Teclas de récords.
    pub(crate) fn handle_records_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.records.len();
                if total > 0 {
                    let indice = (self.records_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.records_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.records_table.selected().unwrap_or(0).saturating_sub(1);
                self.records_table.select(Some(indice));
            }
            KeyCode::Enter => {
                if let Some(partido_id) = self
                    .records_table
                    .selected()
                    .and_then(|indice| self.records.get(indice))
                    .and_then(|record| record.partido_id)
                {
                    self.navegar_a(TipoResultado::Partido, partido_id)?;
                }
            }
            KeyCode::Char('r') => {
                self.refrescar_records()?;
                self.status = "Récords recalculados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    // ------------------------------------------------------------------
    // Tutorial
    // ------------------------------------------------------------------

    /// Abre el tutorial de inicio.
    pub(crate) fn open_tutorial(&mut self) -> Result<()> {
        self.tutorial_paso = 0;
        if !matches!(self.current_screen(), Screen::Tutorial) {
            self.push_screen(Screen::Tutorial);
        }
        Ok(())
    }

    /// Teclas del tutorial.
    pub(crate) fn handle_tutorial_key(&mut self, key: KeyEvent) -> Result<bool> {
        const PASOS: usize = 4;
        match key.code {
            KeyCode::Right | KeyCode::Enter | KeyCode::Char('n') => {
                if self.tutorial_paso + 1 >= PASOS {
                    let _ = crate::db::settings_repo::marcar_tutorial(self.db.conn());
                    self.tutorial_completado = true;
                    self.go_back();
                } else {
                    self.tutorial_paso += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('p') => {
                self.tutorial_paso = self.tutorial_paso.saturating_sub(1);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    // ------------------------------------------------------------------
    // Overlays: búsqueda global y paleta
    // ------------------------------------------------------------------

    /// Atajos globales de overlay (Ctrl+K / Ctrl+F).
    pub(crate) fn handle_overlay_shortcut(&mut self, key: KeyEvent) -> Result<bool> {
        if self.form.is_some() || !key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('k') => {
                self.abrir_paleta();
                Ok(true)
            }
            KeyCode::Char('f') => {
                self.abrir_busqueda();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Abre la búsqueda global.
    pub(crate) fn abrir_busqueda(&mut self) {
        self.overlay = Some(Overlay::Busqueda(BusquedaState::default()));
    }

    /// Abre la paleta de comandos.
    pub(crate) fn abrir_paleta(&mut self) {
        self.overlay = Some(Overlay::Paleta(PaletaState::default()));
    }

    /// Teclas del overlay abierto.
    pub(crate) fn handle_overlay_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(mut overlay) = self.overlay.take() else {
            return Ok(());
        };
        let mut cerrar = false;
        let mut navegar: Option<(TipoResultado, i64)> = None;
        let mut accion: Option<&'static str> = None;

        if key.code == KeyCode::Esc {
            return Ok(());
        }

        match &mut overlay {
            Overlay::Busqueda(estado) => match key.code {
                KeyCode::Backspace => {
                    estado.query.pop();
                    estado.selected = 0;
                }
                KeyCode::Up => estado.selected = estado.selected.saturating_sub(1),
                KeyCode::Down => {
                    if !estado.resultados.is_empty() {
                        estado.selected = (estado.selected + 1).min(estado.resultados.len() - 1);
                    }
                }
                KeyCode::Enter => {
                    if let Some(resultado) = estado.resultados.get(estado.selected) {
                        navegar = Some((resultado.tipo, resultado.id));
                        cerrar = true;
                    }
                }
                KeyCode::Char(caracter) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    estado.query.push(caracter);
                    estado.selected = 0;
                }
                _ => {}
            },
            Overlay::Paleta(estado) => match key.code {
                KeyCode::Backspace => {
                    estado.query.pop();
                    estado.selected = 0;
                }
                KeyCode::Up => estado.selected = estado.selected.saturating_sub(1),
                KeyCode::Down => {
                    let total = acciones_paleta(estado).len();
                    if total > 0 {
                        estado.selected = (estado.selected + 1).min(total - 1);
                    }
                }
                KeyCode::Enter => {
                    if let Some((_, id)) = acciones_paleta(estado).get(estado.selected) {
                        accion = Some(id);
                        cerrar = true;
                    }
                }
                KeyCode::Char(caracter) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    estado.query.push(caracter);
                    estado.selected = 0;
                }
                _ => {}
            },
        }

        if let Overlay::Busqueda(estado) = &mut overlay
            && !cerrar
        {
            match busqueda_service::buscar(self.db.conn(), &estado.query, 40) {
                Ok(resultados) => estado.resultados = resultados,
                Err(error) => self.status = format!("Error de búsqueda: {error}"),
            }
        }

        if !cerrar {
            self.overlay = Some(overlay);
        }
        if let Some((tipo, id)) = navegar {
            self.navegar_a(tipo, id)?;
        }
        if let Some(id) = accion {
            self.ejecutar_accion_paleta(id)?;
        }
        Ok(())
    }

    fn ejecutar_accion_paleta(&mut self, id: &str) -> Result<()> {
        match id {
            "nuevo_partido" => self.abrir_nuevo_partido(),
            "partidos" => self.open_module(Module::Partidos),
            "camisetas" => self.open_module(Module::Camisetas),
            "canchas" => self.open_module(Module::Canchas),
            "calendario" => self.open_calendario(),
            "logros" => self.open_logros(),
            "records" => self.open_records(),
            "torneos" => self.open_torneos(),
            "ultimos" => self.open_ultimos(),
            "comparador" => self.open_comparador(),
            "deshacer" => self.solicitar_undo(),
            "notificaciones" => self.open_notificaciones(),
            "resumen" => self.open_module(Module::Resumen),
            "ajustes" => self.open_module(Module::Ajustes),
            "ayuda" => {
                self.push_screen(Screen::Ayuda);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Navega al detalle de un resultado de búsqueda.
    pub(crate) fn navegar_a(&mut self, tipo: TipoResultado, id: i64) -> Result<()> {
        match tipo {
            TipoResultado::Partido => {
                self.open_module(Module::Partidos)?;
                if let Ok(partido) = partido_service::obtener(self.db.conn(), id) {
                    self.partido_detail = Some(partido);
                    self.push_screen(Screen::PartidoDetail(id));
                }
            }
            TipoResultado::Camiseta => {
                self.open_module(Module::Camisetas)?;
                if let Ok(camiseta) = camiseta_service::obtener(self.db.conn(), id) {
                    self.camiseta_detail = Some(camiseta);
                    self.camiseta_ficha = camiseta_service::ficha(self.db.conn(), id).ok();
                    self.push_screen(Screen::CamisetaDetail(id));
                }
            }
            TipoResultado::Cancha => {
                self.open_module(Module::Canchas)?;
                if let Ok(cancha) = cancha_service::obtener(self.db.conn(), id) {
                    self.cancha_detail = Some(cancha);
                    self.push_screen(Screen::CanchaDetail(id));
                }
            }
            TipoResultado::Equipo => {
                self.open_module(Module::Equipos)?;
                if let Ok(equipo) = equipo_service::obtener(self.db.conn(), id) {
                    self.equipo_detail = Some(equipo);
                    self.jugadores_table.select(Some(0));
                    self.push_screen(Screen::EquipoDetail(id));
                }
            }
            TipoResultado::Jugador => {
                let equipo_id = self
                    .db
                    .conn()
                    .query_row(
                        "SELECT equipo_id FROM jugador WHERE id = ?1",
                        rusqlite::params![id],
                        |row| row.get::<_, i64>(0),
                    )
                    .ok();
                if let Some(equipo_id) = equipo_id {
                    self.open_module(Module::Equipos)?;
                    if let Ok(equipo) = equipo_service::obtener(self.db.conn(), equipo_id) {
                        self.equipo_detail = Some(equipo);
                        self.jugadores_table.select(Some(0));
                        self.push_screen(Screen::EquipoDetail(equipo_id));
                    }
                }
            }
            TipoResultado::Lesion => {
                self.open_module(Module::Lesiones)?;
                if let Ok(lesion) = lesion_service::obtener(self.db.conn(), id) {
                    self.lesion_detail = Some(lesion);
                    self.push_screen(Screen::LesionDetail(id));
                }
            }
            TipoResultado::Financiamiento => {
                self.open_module(Module::Financiamiento)?;
            }
        }
        Ok(())
    }
}

/// Acciones de la paleta filtradas por el texto.
#[must_use]
pub fn acciones_paleta(estado: &PaletaState) -> Vec<(&'static str, &'static str)> {
    let consulta = estado.query.trim().to_lowercase();
    ACCIONES_PALETA
        .iter()
        .copied()
        .filter(|(etiqueta, _)| consulta.is_empty() || etiqueta.to_lowercase().contains(&consulta))
        .collect()
}
