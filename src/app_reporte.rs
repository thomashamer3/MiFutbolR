//! Pantalla genérica de reporte y formularios de elección (`Screen::Eleccion`).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Eleccion, Field, Form, Modal, Screen};
use crate::db::reportes_repo::FiltroPartidos;
use crate::domain::dates;
use crate::domain::models::{EquipoFilter, EstadisticaKind};
use crate::domain::reporte::Reporte;
use crate::services::reportes_service::{self, TipoReporte};
use crate::services::{cancha_service, equipo_service, simulacion_service};

impl Eleccion {
    /// Título de la pantalla.
    #[must_use]
    pub const fn titulo(self) -> &'static str {
        match self {
            Self::SimularEquipos => "Partidos · simular con equipos guardados",
            Self::FiltrosPartidos => "Estadísticas · filtros avanzados",
            Self::GenerarReporte => "Estadísticas · reportes automáticos",
            Self::NuevaNotificacion => "Notificaciones · nueva",
        }
    }
}

impl App {
    /// Muestra un reporte desde arriba.
    pub(crate) fn abrir_reporte(&mut self, reporte: Reporte) {
        self.reporte = reporte;
        self.reporte_scroll = 0;
        self.push_screen(Screen::Reporte);
    }

    /// Desplazamiento y exportación del reporte.
    pub(crate) fn handle_reporte_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.reporte_scroll = self.reporte_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.reporte_scroll = self.reporte_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => self.reporte_scroll = self.reporte_scroll.saturating_add(10),
            KeyCode::PageUp => self.reporte_scroll = self.reporte_scroll.saturating_sub(10),
            KeyCode::Home => self.reporte_scroll = 0,
            KeyCode::Char('x') => self.exportar_reporte(),
            _ => return false,
        }
        true
    }

    /// Guarda el reporte como `reporte_<sello>.txt` en Exportaciones/.
    fn exportar_reporte(&mut self) {
        let destino = self.paths.export_dir.join(format!(
            "reporte_{}.txt",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        let resultado = std::fs::create_dir_all(&self.paths.export_dir)
            .and_then(|()| std::fs::write(&destino, self.reporte.texto()));
        match resultado {
            Ok(()) => self.status = format!("Reporte exportado: {}", destino.display()),
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Abre un formulario de elección.
    pub(crate) fn abrir_eleccion(&mut self, eleccion: Eleccion, campos: Vec<Field>) {
        self.form = Some(Form::new(eleccion.titulo(), campos));
        self.push_screen(Screen::Eleccion(eleccion));
    }

    /// Ctrl+S en un formulario de elección.
    pub(crate) fn save_eleccion(&mut self, eleccion: Eleccion) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let resultado = match eleccion {
            Eleccion::SimularEquipos => self.simular_equipos(&form),
            Eleccion::FiltrosPartidos => self.filtrar_partidos(&form),
            Eleccion::GenerarReporte => self.generar_reporte(&form),
            Eleccion::NuevaNotificacion => self.crear_notificacion(&form),
        };
        match resultado {
            Ok(Some(reporte)) => {
                self.form = None;
                self.stack.pop();
                self.abrir_reporte(reporte);
            }
            Ok(None) => {
                self.form = None;
                self.stack.pop();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error));
            }
        }
        Ok(())
    }

    /// Partidos → `s`: elegir local y visitante (`simular_partido_guardados`).
    pub(crate) fn abrir_simulacion(&mut self) {
        let equipos: Vec<(String, String)> =
            equipo_service::listar(self.db.conn(), &EquipoFilter::default())
                .unwrap_or_default()
                .into_iter()
                .map(|e| (e.id.to_string(), e.nombre))
                .collect();
        if equipos.len() < 2 {
            self.status =
                "Se necesitan al menos 2 equipos guardados para simular un partido".to_string();
            return;
        }
        let campos = vec![
            Field::select(
                "local",
                "Equipo local",
                equipos[0].0.clone(),
                equipos.clone(),
            ),
            Field::select(
                "visitante",
                "Equipo visitante",
                equipos[1].0.clone(),
                equipos,
            ),
        ];
        self.abrir_eleccion(Eleccion::SimularEquipos, campos);
    }

    /// Enter en el hub de Estadísticas: las categorías nuevas abren un reporte o
    /// un formulario; el resto, su detalle de siempre.
    pub(crate) fn abrir_estadistica(&mut self, kind: EstadisticaKind) {
        let reporte = match kind {
            EstadisticaKind::Contexto => reportes_service::contexto(self.db.conn()),
            EstadisticaKind::Posiciones => reportes_service::tabla_posiciones(self.db.conn()),
            EstadisticaKind::Filtros => {
                let mut canchas = vec![(String::new(), "(todas)".to_string())];
                canchas.extend(
                    cancha_service::listar(self.db.conn(), &Default::default())
                        .unwrap_or_default()
                        .into_iter()
                        .map(|c| (c.id.to_string(), c.nombre)),
                );
                let resultados = ["", "1", "2", "3"]
                    .iter()
                    .zip(["(todos)", "Victoria", "Empate", "Derrota"])
                    .map(|(v, e)| ((*v).to_string(), e.to_string()))
                    .collect();
                let campos = vec![
                    Field::date("desde", "Fecha desde", ""),
                    Field::date("hasta", "Fecha hasta", ""),
                    Field::select("resultado", "Resultado", "", resultados),
                    Field::select("cancha", "Cancha", "", canchas),
                ];
                self.abrir_eleccion(Eleccion::FiltrosPartidos, campos);
                return;
            }
            EstadisticaKind::Reportes => {
                let tipos = vec![
                    (
                        "html".to_string(),
                        TipoReporte::GeneralHtml.nombre().to_string(),
                    ),
                    (
                        "txt".to_string(),
                        TipoReporte::RendimientoTxt.nombre().to_string(),
                    ),
                    ("ver".to_string(), "Ver reportes generados".to_string()),
                ];
                let campos = vec![Field::select("tipo", "Reporte", "html", tipos)];
                self.abrir_eleccion(Eleccion::GenerarReporte, campos);
                return;
            }
            _ => {
                self.estadisticas_scroll = 0;
                self.push_screen(Screen::EstadisticasDetalle(kind));
                return;
            }
        };
        match reporte {
            Ok(reporte) => self.abrir_reporte(reporte),
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    fn filtrar_partidos(&self, form: &Form) -> Result<Option<Reporte>, String> {
        let valor = |clave: &str| form.value(clave).unwrap_or("").trim().to_string();
        let filtro = FiltroPartidos {
            desde: dates::parse_date(&valor("desde")),
            hasta: dates::parse_date(&valor("hasta")),
            resultado: valor("resultado").parse().ok(),
            cancha_id: valor("cancha").parse().ok(),
            cansancio_alto: false,
        };
        reportes_service::filtrar(self.db.conn(), &filtro)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    fn generar_reporte(&mut self, form: &Form) -> Result<Option<Reporte>, String> {
        let tipo = match form.value("tipo") {
            Some("html") => Some(TipoReporte::GeneralHtml),
            Some("txt") => Some(TipoReporte::RendimientoTxt),
            _ => None,
        };
        let dir = self.paths.export_dir.clone();
        if let Some(tipo) = tipo {
            let ruta = reportes_service::generar(self.db.conn(), &dir, tipo)?;
            self.status = format!("Reporte generado: {}", ruta.display());
        }
        reportes_service::generados(self.db.conn(), &dir)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// Notificaciones → `n` (`crear_notificacion` de C).
    pub(crate) fn abrir_nueva_notificacion(&mut self) {
        let tipos = crate::services::notificacion_service::TIPOS_MANUALES
            .iter()
            .map(|t| ((*t).to_string(), (*t).to_string()))
            .collect();
        let campos = vec![
            Field::select("tipo", "Tipo", "general", tipos),
            Field::text("titulo", "Título", "", 80).required(),
            Field::text("mensaje", "Mensaje", "", 240).required(),
        ];
        self.abrir_eleccion(Eleccion::NuevaNotificacion, campos);
    }

    fn crear_notificacion(&mut self, form: &Form) -> Result<Option<Reporte>, String> {
        let valor = |clave: &str| form.value(clave).unwrap_or("").trim().to_string();
        crate::services::notificacion_service::crear(
            self.db.conn(),
            &valor("tipo"),
            &valor("titulo"),
            &valor("mensaje"),
        )
        .map_err(|e| e.to_string())?;
        self.refrescar_notificaciones();
        self.status = "Notificación creada".to_string();
        Ok(None)
    }

    fn simular_equipos(&self, form: &Form) -> Result<Option<Reporte>, String> {
        let equipo = |clave: &str| {
            let id = form
                .value(clave)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);
            equipo_service::obtener(self.db.conn(), id).map_err(|e| e.to_string())
        };
        let (local, visitante) = (equipo("local")?, equipo("visitante")?);
        simulacion_service::simular(&mut rand::thread_rng(), &local, &visitante)
            .map(|sim| Some(sim.reporte))
    }
}
