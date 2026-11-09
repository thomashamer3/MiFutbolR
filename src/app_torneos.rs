//! Navegación, formularios y fixture de torneos (IMP-11).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::models::FormMode;
use crate::domain::torneos::{PartidoTorneoRow, TipoTorneo, Torneo};
use crate::services::torneos_service;

impl App {
    /// Abre el listado de torneos.
    pub(crate) fn open_torneos(&mut self) -> Result<()> {
        self.refresh_torneos()?;
        if !matches!(self.current_screen(), Screen::TorneosList) {
            self.push_screen(Screen::TorneosList);
        }
        Ok(())
    }

    /// Recarga el listado.
    pub(crate) fn refresh_torneos(&mut self) -> Result<()> {
        match torneos_service::listar(self.db.conn()) {
            Ok(torneos) => {
                self.torneos = torneos;
                self.torneos_table.select(if self.torneos.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron cargar los torneos: {error}"),
        }
        Ok(())
    }

    /// Abre el detalle de un torneo.
    pub(crate) fn open_torneo_detail(&mut self, id: i64) -> Result<()> {
        match torneos_service::ficha(self.db.conn(), id) {
            Ok(ficha) => {
                self.torneo_detail = Some(ficha);
                self.torneo_partidos_table.select(Some(0));
                self.push_screen(Screen::TorneoDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Recarga la ficha abierta.
    pub(crate) fn refresh_torneo_detail(&mut self) -> Result<()> {
        let Screen::TorneoDetail(id) = self.current_screen() else {
            return Ok(());
        };
        match torneos_service::ficha(self.db.conn(), id) {
            Ok(ficha) => {
                let total = ficha.partidos.len();
                self.torneo_detail = Some(ficha);
                if total == 0 {
                    self.torneo_partidos_table.select(None);
                } else {
                    let indice = self
                        .torneo_partidos_table
                        .selected()
                        .unwrap_or(0)
                        .min(total - 1);
                    self.torneo_partidos_table.select(Some(indice));
                }
            }
            Err(error) => self.status = format!("No se pudo recargar el torneo: {error}"),
        }
        Ok(())
    }

    /// Teclas del listado.
    pub(crate) fn handle_torneos_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.torneos.len();
                if total > 0 {
                    let indice = (self.torneos_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.torneos_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.torneos_table.selected().unwrap_or(0).saturating_sub(1);
                self.torneos_table.select(Some(indice));
            }
            KeyCode::Enter => {
                if let Some(id) = self
                    .torneos_table
                    .selected()
                    .and_then(|indice| self.torneos.get(indice))
                    .map(|torneo| torneo.id)
                {
                    self.open_torneo_detail(id)?;
                }
            }
            KeyCode::Char('n') => self.open_torneo_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(torneo) = self
                    .torneos_table
                    .selected()
                    .and_then(|indice| self.torneos.get(indice))
                    .cloned()
                {
                    self.open_torneo_form_edit(&torneo)?;
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .torneos_table
                    .selected()
                    .and_then(|indice| self.torneos.get(indice))
                    .map(|torneo| torneo.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar torneo",
                        "Se eliminarán el fixture y las asociaciones. ¿Continuar?",
                        ConfirmAction::DeleteTorneo(id),
                    ));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_torneos()?;
                self.status = "Torneos actualizados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle.
    pub(crate) fn handle_torneo_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(ficha) = self.torneo_detail.clone() else {
            return Ok(false);
        };
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = ficha.partidos.len();
                if total > 0 {
                    let indice =
                        (self.torneo_partidos_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.torneo_partidos_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self
                    .torneo_partidos_table
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                self.torneo_partidos_table.select(Some(indice));
            }
            KeyCode::Char('g') => {
                self.modal = Some(Modal::confirm(
                    "Generar fixture",
                    "Se reemplazará el fixture actual por uno nuevo. ¿Continuar?",
                    ConfirmAction::GenerarFixture(ficha.torneo.id),
                ));
            }
            KeyCode::Char('r') => {
                if let Some(partido) = self
                    .torneo_partidos_table
                    .selected()
                    .and_then(|indice| ficha.partidos.get(indice))
                    .cloned()
                {
                    self.open_torneo_resultado(&partido)?;
                }
            }
            KeyCode::Char('a') => {
                match torneos_service::asociar_todos(self.db.conn(), ficha.torneo.id) {
                    Ok(0) => {
                        self.status = "No hay equipos activos nuevos para asociar".to_string();
                    }
                    Ok(nuevos) => {
                        self.status = format!("{nuevos} equipo(s) asociado(s)");
                        self.refresh_torneo_detail()?;
                    }
                    Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                }
            }
            KeyCode::Char('e') => self.open_torneo_form_edit(&ficha.torneo)?,
            KeyCode::Char('d') => {
                self.modal = Some(Modal::confirm(
                    "Eliminar torneo",
                    "Se eliminarán el fixture y las asociaciones. ¿Continuar?",
                    ConfirmAction::DeleteTorneo(ficha.torneo.id),
                ));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn open_torneo_form(&mut self, mode: FormMode) -> Result<()> {
        self.torneo_edit_id = None;
        self.form = Some(torneo_form(None));
        self.push_screen(Screen::TorneoForm(mode));
        Ok(())
    }

    fn open_torneo_form_edit(&mut self, torneo: &Torneo) -> Result<()> {
        self.torneo_edit_id = Some(torneo.id);
        self.form = Some(torneo_form(Some(torneo)));
        self.push_screen(Screen::TorneoForm(FormMode::Edit));
        Ok(())
    }

    fn open_torneo_resultado(&mut self, partido: &PartidoTorneoRow) -> Result<()> {
        self.torneo_resultado_partido = Some(partido.id);
        self.form = Some(resultado_form(partido));
        self.push_screen(Screen::TorneoResultadoForm(partido.id));
        Ok(())
    }

    /// Guarda el formulario de torneo.
    pub(crate) fn save_torneo_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let nombre = form.value("nombre").unwrap_or("").to_string();
        let tipo = TipoTorneo::from_i64(
            form.value("tipo")
                .and_then(|valor| valor.parse::<i64>().ok())
                .unwrap_or(0),
        );
        let resultado = match mode {
            FormMode::Create => torneos_service::crear(self.db.conn(), &nombre, tipo).map(Some),
            FormMode::Edit => {
                let Some(id) = self.torneo_edit_id else {
                    self.form = Some(form);
                    self.modal = Some(Modal::Error("Falta el torneo a editar".to_string()));
                    return Ok(());
                };
                torneos_service::actualizar(self.db.conn(), id, &nombre, tipo).map(|()| Some(id))
            }
        };
        match resultado {
            Ok(id) => {
                self.form = None;
                self.refresh_torneos()?;
                self.go_back();
                match (mode, id) {
                    (FormMode::Edit, _) => {
                        self.refresh_torneo_detail()?;
                    }
                    (FormMode::Create, Some(id)) => {
                        self.open_torneo_detail(id)?;
                    }
                    (FormMode::Create, None) => {}
                }
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }

    /// Guarda el resultado de un partido del fixture.
    pub(crate) fn save_torneo_resultado_form(&mut self) -> Result<()> {
        let Screen::TorneoResultadoForm(partido_id) = self.current_screen() else {
            return Ok(());
        };
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let local = form
            .value("local")
            .and_then(|valor| valor.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let visitante = form
            .value("visitante")
            .and_then(|valor| valor.trim().parse::<i64>().ok())
            .unwrap_or(0);
        match torneos_service::registrar_resultado(self.db.conn(), partido_id, local, visitante) {
            Ok(()) => {
                self.form = None;
                self.go_back();
                self.refresh_torneo_detail()?;
                self.status = "Resultado guardado".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn torneo_form(torneo: Option<&Torneo>) -> Form {
    let nombre = torneo.map_or_else(String::new, |torneo| torneo.nombre.clone());
    let tipo = torneo.map_or(0, |torneo| torneo.tipo.as_i64()).to_string();
    let campos = vec![
        Field::text("nombre", "Nombre", nombre, 100).required(),
        Field::select(
            "tipo",
            "Tipo de torneo",
            tipo,
            vec![
                ("0".to_string(), "Solo ida".to_string()),
                ("1".to_string(), "Ida y vuelta".to_string()),
            ],
        ),
    ];
    Form::new("Torneo", campos)
}

fn resultado_form(partido: &PartidoTorneoRow) -> Form {
    let campo = |key: &'static str, label: &str, valor: i64| Field {
        key,
        label: label.to_string(),
        value: valor.to_string(),
        kind: FieldKind::Int { min: 0, max: 99 },
        required: true,
        help: None,
    };
    let campos = vec![
        campo(
            "local",
            &format!("Goles de {}", partido.local),
            partido.goles_local,
        ),
        campo(
            "visitante",
            &format!("Goles de {}", partido.visitante),
            partido.goles_visitante,
        ),
    ];
    Form::new(
        format!("Resultado · {} vs {}", partido.local, partido.visitante),
        campos,
    )
}
