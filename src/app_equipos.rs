//! Navegación, formularios y acciones del módulo Equipos.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    App, ConfirmAction, Field, FieldKind, Form, Modal, Screen, SearchState, SearchTarget,
};
use crate::domain::models::{Equipo, FormMode, Jugador, Posicion, TipoEquipo, TipoFutbol};
use crate::domain::validation::DomainError;
use crate::services::equipo_service::{self, ResultadoEliminacion};

impl App {
    /// Teclas del listado de equipos.
    pub(crate) fn handle_equipos_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_equipo_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_equipo_selection(-1),
            KeyCode::Enter => self.open_equipo_detail()?,
            KeyCode::Char('n') => self.open_equipo_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_equipo_id() {
                    self.open_equipo_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_equipo(),
            KeyCode::Char('a') => self.toggle_equipo_activa(),
            KeyCode::Char('f') => {
                self.equipo_filter.estado = self.equipo_filter.estado.next();
                self.refresh_equipos()?;
            }
            KeyCode::Char('o') => {
                self.equipo_filter.orden = self.equipo_filter.orden.next();
                self.refresh_equipos()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Equipos,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_equipos()?;
                self.status = "Listado de equipos actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle: gestión de jugadores.
    pub(crate) fn handle_equipo_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_jugador_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_jugador_selection(-1),
            KeyCode::Char('a') => self.open_jugador_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if self.selected_jugador().is_some() {
                    self.open_jugador_form(FormMode::Edit)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_jugador(),
            KeyCode::Char('v') => {
                if let Some(equipo) = &self.equipo_detail {
                    let ruta = equipo.imagen_ruta.clone();
                    self.abrir_imagen(&ruta);
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Recarga el listado.
    pub fn refresh_equipos(&mut self) -> Result<()> {
        let rows = equipo_service::listar(self.db.conn(), &self.equipo_filter)?;
        self.equipos = rows;
        if self.equipos.is_empty() {
            self.equipos_table.select(None);
        } else {
            let index = self
                .equipos_table
                .selected()
                .unwrap_or(0)
                .min(self.equipos.len() - 1);
            self.equipos_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_equipo_id(&self) -> Option<i64> {
        let index = self.equipos_table.selected()?;
        self.equipos.get(index).map(|row| row.id)
    }

    /// Jugador seleccionado en el detalle.
    #[must_use]
    pub fn selected_jugador(&self) -> Option<Jugador> {
        let equipo = self.equipo_detail.as_ref()?;
        let index = self.jugadores_table.selected()?;
        equipo.jugadores.get(index).cloned()
    }

    fn move_equipo_selection(&mut self, delta: isize) {
        if self.equipos.is_empty() {
            return;
        }
        let len = self.equipos.len() as isize;
        let current = self
            .equipos_table
            .selected()
            .unwrap_or(0)
            .min(self.equipos.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.equipos_table.select(Some(next));
    }

    fn move_jugador_selection(&mut self, delta: isize) {
        let Some(equipo) = &self.equipo_detail else {
            return;
        };
        if equipo.jugadores.is_empty() {
            return;
        }
        let len = equipo.jugadores.len() as isize;
        let current = self
            .jugadores_table
            .selected()
            .unwrap_or(0)
            .min(equipo.jugadores.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.jugadores_table.select(Some(next));
    }

    fn open_equipo_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_equipo_id() else {
            self.status = "No hay equipos para mostrar".to_string();
            return Ok(());
        };
        match equipo_service::obtener(self.db.conn(), id) {
            Ok(equipo) => {
                self.equipo_detail = Some(equipo);
                self.jugadores_table.select(Some(0));
                self.push_screen(Screen::EquipoDetail(id));
                self.reload_equipo_detail()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn reload_equipo_detail(&mut self) -> Result<()> {
        let Screen::EquipoDetail(id) = self.current_screen() else {
            return Ok(());
        };
        if let Ok(equipo) = equipo_service::obtener(self.db.conn(), id) {
            let len = equipo.jugadores.len();
            self.equipo_detail = Some(equipo);
            if len == 0 {
                self.jugadores_table.select(None);
            } else {
                let index = self.jugadores_table.selected().unwrap_or(0).min(len - 1);
                self.jugadores_table.select(Some(index));
            }
        }
        Ok(())
    }

    fn open_equipo_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                self.form_original_equipo = None;
                self.form = Some(equipo_form(None));
                self.push_screen(Screen::EquipoForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_equipo_id() {
                    self.open_equipo_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_equipo_form_for(&mut self, id: i64) -> Result<()> {
        match equipo_service::obtener(self.db.conn(), id) {
            Ok(equipo) => {
                self.form = Some(equipo_form(Some(&equipo)));
                self.form_original_equipo = Some(equipo);
                self.push_screen(Screen::EquipoForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_equipo(&mut self) {
        let Some(id) = self.selected_equipo_id() else {
            self.status = "No hay equipo seleccionado".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar equipo",
            "¿Eliminar el equipo seleccionado? Si tiene historial se desactivará. (S = sí, N = no)",
            ConfirmAction::DeleteEquipo(id),
        ));
    }

    /// Elimina o desactiva un equipo.
    pub(crate) fn delete_equipo(&mut self, id: i64) -> Result<()> {
        match equipo_service::eliminar(self.db.conn(), id) {
            Ok(ResultadoEliminacion::Eliminada) => {
                self.status = "Equipo eliminado".to_string();
            }
            Ok(ResultadoEliminacion::Desactivada(dependencias)) => {
                self.status = format!(
                    "Equipo desactivado ({} partidos de torneo, {} torneos)",
                    dependencias.partido_torneo, dependencias.torneos
                );
            }
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        }
        if matches!(self.current_screen(), Screen::EquipoDetail(_)) {
            self.stack.pop();
        }
        self.equipo_detail = None;
        self.refresh_equipos()?;
        Ok(())
    }

    fn toggle_equipo_activa(&mut self) {
        let Some(id) = self.selected_equipo_id() else {
            return;
        };
        let Some(row) = self.equipos.get(self.equipos_table.selected().unwrap_or(0)) else {
            return;
        };
        let nuevo = !row.activa;
        match equipo_service::set_activa(self.db.conn(), id, nuevo) {
            Ok(()) => {
                self.status = format!("Equipo {}", if nuevo { "activado" } else { "desactivado" });
                if let Err(error) = self.refresh_equipos() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    fn open_jugador_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(equipo) = &self.equipo_detail else {
            return Ok(());
        };
        let equipo_id = equipo.id;
        match mode {
            FormMode::Create => {
                self.jugador_edit_id = None;
                self.form = Some(jugador_form(None, equipo_id, None));
                self.push_screen(Screen::JugadorForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(jugador) = self.selected_jugador() {
                    self.form = Some(jugador_form(None, equipo_id, Some(&jugador)));
                    self.jugador_edit_id = Some(jugador.id);
                    self.push_screen(Screen::JugadorForm(FormMode::Edit));
                }
            }
        }
        Ok(())
    }

    fn confirm_delete_jugador(&mut self) {
        let Some(jugador) = self.selected_jugador() else {
            self.status = "No hay jugador seleccionado".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar jugador",
            format!(
                "¿Eliminar a {} (#{}) del equipo? (S = sí, N = no)",
                jugador.nombre, jugador.numero
            ),
            ConfirmAction::DeleteJugador(jugador.id),
        ));
    }

    /// Elimina un jugador y recarga el detalle.
    pub(crate) fn delete_jugador(&mut self, id: i64) -> Result<()> {
        match equipo_service::eliminar_jugador(self.db.conn(), id) {
            Ok(()) => {
                self.status = "Jugador eliminado".to_string();
                self.reload_equipo_detail()?;
                self.refresh_equipos()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Guarda el formulario de equipo.
    pub(crate) fn save_equipo_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let equipo = match self.equipo_desde_form() {
            Ok(equipo) => equipo,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nuevo = equipo;
                equipo_service::crear(self.db.conn_mut(), &mut nuevo).map(|_| ())
            }
            FormMode::Edit => equipo_service::actualizar(self.db.conn(), &equipo),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_equipo = None;
                self.stack.pop();
                self.refresh_equipos()?;
                if matches!(self.current_screen(), Screen::EquipoDetail(_)) {
                    self.reload_equipo_detail()?;
                }
                self.status = "Equipo guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn equipo_desde_form(&self) -> Result<Equipo, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let tipo_code: i64 = get("tipo")
            .parse()
            .map_err(|_| DomainError::Message("Tipo de equipo inválido".to_string()))?;
        let modalidad_code: i64 = get("tipo_futbol")
            .parse()
            .map_err(|_| DomainError::Message("Modalidad inválida".to_string()))?;
        let num_jugadores: i64 = get("num_jugadores")
            .parse()
            .map_err(|_| DomainError::Message("Cantidad de jugadores inválida".to_string()))?;
        let original = self.form_original_equipo.as_ref();
        Ok(Equipo {
            id: original.map_or(0, |equipo| equipo.id),
            nombre: get("nombre"),
            tipo: TipoEquipo::try_from(tipo_code)?,
            tipo_futbol: TipoFutbol::try_from(modalidad_code)?,
            num_jugadores,
            partido_id: original.and_then(|equipo| equipo.partido_id),
            activa: original.is_none_or(|equipo| equipo.activa),
            imagen_ruta: get("imagen"),
            jugadores: original.map_or_else(Vec::new, |equipo| equipo.jugadores.clone()),
        })
    }

    /// Guarda el formulario de jugador.
    pub(crate) fn save_jugador_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let jugador = match self.jugador_desde_form() {
            Ok(jugador) => jugador,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nuevo = jugador;
                equipo_service::agregar_jugador(self.db.conn_mut(), &mut nuevo).map(|_| ())
            }
            FormMode::Edit => equipo_service::actualizar_jugador(self.db.conn_mut(), &jugador),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.jugador_edit_id = None;
                self.stack.pop();
                self.reload_equipo_detail()?;
                self.refresh_equipos()?;
                self.status = "Jugador guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn jugador_desde_form(&self) -> Result<Jugador, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let equipo = self
            .equipo_detail
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay equipo cargado".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let numero: i64 = get("numero")
            .parse()
            .map_err(|_| DomainError::Message("Dorsal inválido".to_string()))?;
        let posicion_code: i64 = get("posicion")
            .parse()
            .map_err(|_| DomainError::Message("Posición inválida".to_string()))?;
        Ok(Jugador {
            id: self.jugador_edit_id.unwrap_or(0),
            equipo_id: equipo.id,
            nombre: get("nombre"),
            numero,
            posicion: Posicion::try_from(posicion_code)?,
            es_capitan: get("es_capitan") == "Sí",
        })
    }
}

/// Formulario de equipo.
fn equipo_form(equipo: Option<&Equipo>) -> Form {
    let nombre = equipo.map_or_else(String::new, |equipo| equipo.nombre.clone());
    let imagen = equipo.map_or_else(String::new, |equipo| equipo.imagen_ruta.clone());
    let tipo = equipo.map_or(0, |equipo| equipo.tipo.as_i64()).to_string();
    let tipo_futbol = equipo
        .map_or(0, |equipo| equipo.tipo_futbol.as_i64())
        .to_string();
    let num_jugadores = equipo.map_or(5, |equipo| equipo.num_jugadores).to_string();

    let tipo_options = TipoEquipo::ALL
        .iter()
        .map(|tipo| (tipo.as_i64().to_string(), tipo.label().to_string()))
        .collect();
    let modalidad_options = TipoFutbol::ALL
        .iter()
        .map(|tipo| (tipo.as_i64().to_string(), tipo.label().to_string()))
        .collect();

    Form::new(
        if equipo.is_some() {
            "Editar equipo"
        } else {
            "Nuevo equipo"
        },
        vec![
            Field::text("nombre", "Nombre", nombre, 128).required(),
            Field::select("tipo", "Tipo", tipo, tipo_options),
            Field::select("tipo_futbol", "Modalidad", tipo_futbol, modalidad_options),
            Field {
                key: "num_jugadores",
                label: "Jugadores".to_string(),
                value: num_jugadores,
                kind: FieldKind::Int { min: 1, max: 11 },
                required: true,
                help: Some("Debe alcanzar la modalidad (5/7/8/11)".to_string()),
            },
            Field::text("imagen", "Imagen", imagen, 255),
        ],
    )
}

/// Formulario de jugador.
fn jugador_form(_equipo: Option<&Equipo>, _equipo_id: i64, jugador: Option<&Jugador>) -> Form {
    let nombre = jugador.map_or_else(String::new, |jugador| jugador.nombre.clone());
    let numero = jugador.map_or_else(String::new, |jugador| jugador.numero.to_string());
    let posicion = jugador
        .map_or(0, |jugador| jugador.posicion.as_i64())
        .to_string();
    let capitan = jugador.is_some_and(|jugador| jugador.es_capitan);

    let posicion_options = Posicion::ALL
        .iter()
        .map(|posicion| (posicion.as_i64().to_string(), posicion.label().to_string()))
        .collect();

    Form::new(
        if jugador.is_some() {
            "Editar jugador"
        } else {
            "Nuevo jugador"
        },
        vec![
            Field::text("nombre", "Nombre", nombre, 128).required(),
            Field {
                key: "numero",
                label: "Dorsal".to_string(),
                value: numero,
                kind: FieldKind::Int { min: 0, max: 99 },
                required: true,
                help: None,
            },
            Field::select("posicion", "Posición", posicion, posicion_options),
            Field::boolean("es_capitan", "¿Capitán?", capitan),
        ],
    )
}
