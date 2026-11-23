//! Análisis básico, evolución temporal y química (port de `analisis.c`).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::models::FormMode;
use crate::domain::quimica::Dupla;
use crate::services::{analisis_service, quimica_service};

impl App {
    /// Abre el análisis básico.
    pub(crate) fn open_analisis_basico(&mut self) -> Result<()> {
        self.analisis_scroll = 0;
        match analisis_service::basico(self.db.conn()) {
            Ok(basico) => self.analisis_basico = basico,
            Err(error) => self.status = format!("No se pudo calcular el análisis: {error}"),
        }
        self.push_screen(Screen::AnalisisBasico);
        Ok(())
    }

    /// Abre la evolución temporal.
    pub(crate) fn open_analisis_evolucion(&mut self) -> Result<()> {
        self.analisis_scroll = 0;
        match analisis_service::evolucion(self.db.conn()) {
            Ok(evolucion) => self.analisis_evolucion = evolucion,
            Err(error) => self.status = format!("No se pudo calcular la evolución: {error}"),
        }
        self.push_screen(Screen::AnalisisEvolucion);
        Ok(())
    }

    /// Desplazamiento del análisis; `m` cambia la métrica de la evolución.
    pub(crate) fn handle_analisis_reporte_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.analisis_scroll = self.analisis_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.analisis_scroll = self.analisis_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => self.analisis_scroll = self.analisis_scroll.saturating_add(10),
            KeyCode::PageUp => self.analisis_scroll = self.analisis_scroll.saturating_sub(10),
            KeyCode::Home => self.analisis_scroll = 0,
            KeyCode::Char('m') if self.current_screen() == Screen::AnalisisEvolucion => {
                self.analisis_metrica = self.analisis_metrica.next();
                self.status = format!("Métrica: {}", self.analisis_metrica.label());
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Abre la química entre jugadores.
    pub(crate) fn open_quimica(&mut self) -> Result<()> {
        self.refresh_quimica();
        self.push_screen(Screen::Quimica);
        Ok(())
    }

    /// Recarga duplas y mejor combinación.
    pub(crate) fn refresh_quimica(&mut self) {
        let conn = self.db.conn();
        match (
            quimica_service::listar(conn),
            analisis_service::mejor_combinacion(conn),
        ) {
            (Ok(duplas), Ok(mejor)) => {
                self.quimica_duplas = duplas;
                self.quimica_mejor = mejor;
            }
            (Err(error), _) => self.status = format!("No se pudo cargar la química: {error}"),
            (_, Err(error)) => self.status = format!("No se pudo cargar la química: {error}"),
        }
        let total = self.quimica_duplas.len();
        let actual = self.quimica_table.selected().unwrap_or(0);
        self.quimica_table
            .select((total > 0).then(|| actual.min(total - 1)));
    }

    /// Teclas de la química.
    pub(crate) fn handle_quimica_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.quimica_table, self.quimica_duplas.len(), 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.quimica_table, self.quimica_duplas.len(), -1);
            }
            KeyCode::Char('n') => self.abrir_quimica_form(None),
            KeyCode::Char('e') => {
                if let Some(dupla) = self.dupla_seleccionada() {
                    self.abrir_quimica_form(Some(&dupla));
                }
            }
            KeyCode::Char('d') => {
                if let Some(dupla) = self.dupla_seleccionada() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar dupla",
                        format!(
                            "¿Eliminar la dupla {} + {}? (S = sí, N = no)",
                            dupla.jugador1, dupla.jugador2
                        ),
                        ConfirmAction::DeleteQuimica(dupla.id),
                    ));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_quimica();
                self.status = "Química actualizada".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn dupla_seleccionada(&self) -> Option<Dupla> {
        self.quimica_table
            .selected()
            .and_then(|indice| self.quimica_duplas.get(indice))
            .cloned()
    }

    fn abrir_quimica_form(&mut self, dupla: Option<&Dupla>) {
        let jugadores: Vec<(String, String)> = quimica_service::jugadores(self.db.conn())
            .unwrap_or_default()
            .into_iter()
            .map(|(id, nombre)| (id.to_string(), nombre))
            .collect();
        if jugadores.len() < 2 {
            self.status =
                "Cargá al menos dos jugadores en Equipos para registrar una dupla".to_string();
            return;
        }
        let elegido = |id: Option<i64>, defecto: usize| {
            id.map_or_else(|| jugadores[defecto].0.clone(), |id| id.to_string())
        };
        let numero = |clave: &'static str, etiqueta: &str, valor: i64| Field {
            key: clave,
            label: etiqueta.to_string(),
            value: valor.to_string(),
            kind: FieldKind::Int { min: 0, max: 9999 },
            required: false,
            help: None,
        };
        let campos = vec![
            Field::select(
                "jugador1",
                "Jugador 1",
                elegido(dupla.map(|d| d.jugador1_id), 0),
                jugadores.clone(),
            ),
            Field::select(
                "jugador2",
                "Jugador 2",
                elegido(dupla.map(|d| d.jugador2_id), 1),
                jugadores.clone(),
            ),
            numero(
                "partidos",
                "Partidos juntos",
                dupla.map_or(0, |d| d.partidos),
            ),
            numero("victorias", "Victorias", dupla.map_or(0, |d| d.victorias)),
            numero("goles", "Goles", dupla.map_or(0, |d| d.goles)),
            numero(
                "asistencias",
                "Asistencias",
                dupla.map_or(0, |d| d.asistencias),
            ),
            Field::text(
                "comentario",
                "Comentario",
                dupla.map_or_else(String::new, |d| d.comentario.clone()),
                200,
            ),
        ];
        self.quimica_edit_id = dupla.map(|d| d.id);
        self.form = Some(Form::new("Dupla", campos));
        self.push_screen(Screen::QuimicaForm(if dupla.is_some() {
            FormMode::Edit
        } else {
            FormMode::Create
        }));
    }

    /// Guarda el formulario de dupla.
    pub(crate) fn save_quimica_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let entero = |clave: &str| {
            form.value(clave)
                .and_then(|valor| valor.trim().parse::<i64>().ok())
                .unwrap_or(0)
        };
        let dupla = Dupla {
            id: self.quimica_edit_id.unwrap_or(0),
            jugador1_id: entero("jugador1"),
            jugador2_id: entero("jugador2"),
            jugador1: String::new(),
            jugador2: String::new(),
            partidos: entero("partidos"),
            victorias: entero("victorias"),
            goles: entero("goles"),
            asistencias: entero("asistencias"),
            comentario: form.value("comentario").unwrap_or("").to_string(),
        };
        let resultado = match mode {
            FormMode::Create => quimica_service::crear(self.db.conn(), &dupla).map(|_| ()),
            FormMode::Edit => quimica_service::actualizar(self.db.conn(), &dupla),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.quimica_edit_id = None;
                self.refresh_quimica();
                self.go_back();
                self.status = "Dupla guardada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}
