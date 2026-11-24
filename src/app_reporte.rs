//! Pantalla genérica de reporte y formularios de elección (`Screen::Eleccion`).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Eleccion, Field, Form, Modal, Screen};
use crate::domain::models::EquipoFilter;
use crate::domain::reporte::Reporte;
use crate::services::{equipo_service, simulacion_service};

impl Eleccion {
    /// Título de la pantalla.
    #[must_use]
    pub const fn titulo(self) -> &'static str {
        match self {
            Self::SimularEquipos => "Partidos · simular con equipos guardados",
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
