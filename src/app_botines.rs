//! Navegación, formularios y acciones del módulo Botines.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, Form, Modal, Screen, SearchState, SearchTarget};
use crate::domain::models::{Botin, FormMode};
use crate::domain::validation::DomainError;
use crate::services::botin_service::{self, ResultadoEliminacion};

impl App {
    /// Teclas del listado de botines.
    pub(crate) fn handle_botines_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_botin_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_botin_selection(-1),
            KeyCode::Enter => self.open_botin_detail()?,
            KeyCode::Char('n') => self.open_botin_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_botin_id() {
                    self.open_botin_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_botin(),
            KeyCode::Char('a') => self.toggle_botin_activa(),
            KeyCode::Char('s') => self.sortear_botin()?,
            KeyCode::Char('R') => self.confirm_reset_sorteo(),
            KeyCode::Char('p') => self.fijar_predeterminado_botin(),
            KeyCode::Char('f') => {
                self.botin_filter.estado = self.botin_filter.estado.next();
                self.refresh_botines()?;
            }
            KeyCode::Char('o') => {
                self.botin_filter.orden = self.botin_filter.orden.next();
                self.refresh_botines()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Botines,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_botines()?;
                self.status = "Listado de botines actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle de botín.
    pub(crate) fn handle_botin_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('e') => {
                if let Some(botin) = &self.botin_detail {
                    let id = botin.id;
                    self.open_botin_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_botin(),
            KeyCode::Char('p') => self.fijar_predeterminado_botin(),
            KeyCode::Char('a') => self.toggle_botin_activa(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Recarga el listado.
    pub fn refresh_botines(&mut self) -> Result<()> {
        let rows = botin_service::listar(self.db.conn(), &self.botin_filter)?;
        self.botines = rows;
        if self.botines.is_empty() {
            self.botines_table.select(None);
        } else {
            let index = self
                .botines_table
                .selected()
                .unwrap_or(0)
                .min(self.botines.len() - 1);
            self.botines_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_botin_id(&self) -> Option<i64> {
        let index = self.botines_table.selected()?;
        self.botines.get(index).map(|row| row.id)
    }

    fn move_botin_selection(&mut self, delta: isize) {
        if self.botines.is_empty() {
            return;
        }
        let len = self.botines.len() as isize;
        let current = self
            .botines_table
            .selected()
            .unwrap_or(0)
            .min(self.botines.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.botines_table.select(Some(next));
    }

    fn open_botin_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_botin_id() else {
            self.status = "No hay botines para mostrar".to_string();
            return Ok(());
        };
        match botin_service::obtener(self.db.conn(), id) {
            Ok(botin) => {
                self.botin_detail = Some(botin);
                self.push_screen(Screen::BotinDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_botin_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                self.form_original_botin = None;
                self.form = Some(botin_form(None));
                self.push_screen(Screen::BotinForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_botin_id() {
                    self.open_botin_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_botin_form_for(&mut self, id: i64) -> Result<()> {
        match botin_service::obtener(self.db.conn(), id) {
            Ok(botin) => {
                self.form = Some(botin_form(Some(&botin)));
                self.form_original_botin = Some(botin);
                self.push_screen(Screen::BotinForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_botin(&mut self) {
        let Some(id) = self.selected_botin_id() else {
            self.status = "No hay botín seleccionado".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar botín",
            "¿Eliminar el botín seleccionado? Si tiene partidos se desactivará. (S = sí, N = no)",
            ConfirmAction::DeleteBotin(id),
        ));
    }

    /// Elimina o desactiva un botín.
    pub(crate) fn delete_botin(&mut self, id: i64) -> Result<()> {
        match botin_service::eliminar(self.db.conn(), id) {
            Ok(ResultadoEliminacion::Eliminada) => {
                self.status = "Botín eliminado".to_string();
            }
            Ok(ResultadoEliminacion::Desactivada(dependencias)) => {
                self.status = format!(
                    "Botín desactivado ({} partidos asociados)",
                    dependencias.partidos
                );
            }
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        }
        if matches!(self.current_screen(), Screen::BotinDetail(_)) {
            self.stack.pop();
        }
        self.botin_detail = None;
        self.refresh_botines()?;
        Ok(())
    }

    fn toggle_botin_activa(&mut self) {
        let Some(id) = self.selected_botin_id() else {
            return;
        };
        let Some(row) = self.botines.get(self.botines_table.selected().unwrap_or(0)) else {
            return;
        };
        let nuevo = !row.activa;
        match botin_service::set_activa(self.db.conn(), id, nuevo) {
            Ok(()) => {
                self.status = format!("Botín {}", if nuevo { "activado" } else { "desactivado" });
                if let Err(error) = self.refresh_botines() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    fn sortear_botin(&mut self) -> Result<()> {
        match botin_service::sortear(self.db.conn()) {
            Ok(Some(botin)) => {
                self.status = format!("Sorteado: {}", botin.nombre);
                self.modal = Some(Modal::Info(format!("Botín sorteado: {}", botin.nombre)));
                self.refresh_botines()?;
            }
            Ok(None) => {
                self.modal = Some(Modal::Info(
                    "No hay botines activos para sortear.".to_string(),
                ));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_reset_sorteo(&mut self) {
        self.modal = Some(Modal::confirm(
            "Reiniciar sorteo",
            "Se reiniciará el ciclo de sorteo de todos los botines activos. (S = sí, N = no)",
            ConfirmAction::ResetSorteoBotines,
        ));
    }

    fn fijar_predeterminado_botin(&mut self) {
        let Some(id) = self.selected_botin_id() else {
            return;
        };
        match botin_service::fijar_predeterminado(self.db.conn(), id) {
            Ok(()) => {
                self.status = "Botín predeterminado actualizado".to_string();
                if let Err(error) = self.refresh_botines() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Guarda el formulario de botín.
    pub(crate) fn save_botin_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let botin = match self.botin_desde_form() {
            Ok(botin) => botin,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nuevo = botin;
                botin_service::crear(self.db.conn(), &mut nuevo).map(|_| ())
            }
            FormMode::Edit => botin_service::actualizar(self.db.conn(), &botin),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_botin = None;
                self.stack.pop();
                self.refresh_botines()?;
                if let Screen::BotinDetail(id) = self.current_screen()
                    && let Ok(botin) = botin_service::obtener(self.db.conn(), id)
                {
                    self.botin_detail = Some(botin);
                }
                self.status = "Botín guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn botin_desde_form(&self) -> Result<Botin, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let original = self.form_original_botin.as_ref();
        Ok(Botin {
            id: original.map_or(0, |botin| botin.id),
            nombre: get("nombre"),
            imagen_ruta: get("imagen"),
            activa: original.is_none_or(|botin| botin.activa),
            sorteada: original.is_some_and(|botin| botin.sorteada),
        })
    }
}

/// Formulario de botín.
fn botin_form(botin: Option<&Botin>) -> Form {
    let nombre = botin.map_or_else(String::new, |botin| botin.nombre.clone());
    let imagen = botin.map_or_else(String::new, |botin| botin.imagen_ruta.clone());
    Form::new(
        if botin.is_some() {
            "Editar botín"
        } else {
            "Nuevo botín"
        },
        vec![
            Field::text("nombre", "Nombre", nombre, 100).required(),
            Field::text("imagen", "Imagen", imagen, 255),
        ],
    )
}
