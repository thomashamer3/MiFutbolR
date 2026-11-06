//! Navegación, formularios y acciones del módulo Lesiones.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    App, ConfirmAction, Field, FieldKind, Form, Modal, Screen, SearchState, SearchTarget,
};
use crate::db::helpers::Page;
use crate::domain::dates;
use crate::domain::models::{EstadoLesion, FormMode, Lesion, PartidoFilter};
use crate::domain::validation::{self, DomainError};
use crate::services::camiseta_service;
use crate::services::lesion_service;
use crate::services::partido_service;

impl App {
    /// Teclas del listado.
    pub(crate) fn handle_lesiones_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_lesion_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_lesion_selection(-1),
            KeyCode::Enter => self.open_lesion_detail()?,
            KeyCode::Char('n') => self.open_lesion_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_lesion_id() {
                    self.open_lesion_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_lesion(),
            KeyCode::Char('f') => {
                self.lesion_filter.estado = match self.lesion_filter.estado {
                    None => Some(EstadoLesion::Activa),
                    Some(EstadoLesion::Activa) => Some(EstadoLesion::EnTratamiento),
                    Some(EstadoLesion::EnTratamiento) => Some(EstadoLesion::Mejorando),
                    Some(EstadoLesion::Mejorando) => Some(EstadoLesion::Recuperado),
                    Some(EstadoLesion::Recuperado) => Some(EstadoLesion::Recaida),
                    _ => None,
                };
                self.refresh_lesiones()?;
            }
            KeyCode::Char('o') => {
                self.lesion_filter.orden = self.lesion_filter.orden.next();
                self.refresh_lesiones()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Lesiones,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_lesiones()?;
                self.status = "Listado de lesiones actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle.
    pub(crate) fn handle_lesion_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('e') => {
                if let Some(lesion) = &self.lesion_detail {
                    let id = lesion.id;
                    self.open_lesion_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_lesion(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Recarga listado y estadísticas.
    pub fn refresh_lesiones(&mut self) -> Result<()> {
        self.lesiones = lesion_service::listar(self.db.conn(), &self.lesion_filter)?;
        self.lesion_stats = lesion_service::estadisticas(self.db.conn())?;
        if self.lesiones.is_empty() {
            self.lesiones_table.select(None);
        } else {
            let index = self
                .lesiones_table
                .selected()
                .unwrap_or(0)
                .min(self.lesiones.len() - 1);
            self.lesiones_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_lesion_id(&self) -> Option<i64> {
        let index = self.lesiones_table.selected()?;
        self.lesiones.get(index).map(|row| row.id)
    }

    fn move_lesion_selection(&mut self, delta: isize) {
        if self.lesiones.is_empty() {
            return;
        }
        let len = self.lesiones.len() as isize;
        let current = self
            .lesiones_table
            .selected()
            .unwrap_or(0)
            .min(self.lesiones.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.lesiones_table.select(Some(next));
    }

    fn open_lesion_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_lesion_id() else {
            self.status = "No hay lesiones para mostrar".to_string();
            return Ok(());
        };
        match lesion_service::obtener(self.db.conn(), id) {
            Ok(lesion) => {
                self.lesion_detail = Some(lesion);
                self.push_screen(Screen::LesionDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_lesion_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                let camisetas = camiseta_service::listar(self.db.conn(), &Default::default());
                if camisetas.as_ref().is_ok_and(|rows| rows.is_empty()) {
                    self.modal = Some(Modal::Error(
                        "Antes de registrar una lesión cree al menos una camiseta.".to_string(),
                    ));
                    return Ok(());
                }
                self.form_original_lesion = None;
                self.form = Some(self.lesion_form(None));
                self.push_screen(Screen::LesionForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_lesion_id() {
                    self.open_lesion_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_lesion_form_for(&mut self, id: i64) -> Result<()> {
        match lesion_service::obtener(self.db.conn(), id) {
            Ok(lesion) => {
                self.form = Some(self.lesion_form(Some(&lesion)));
                self.form_original_lesion = Some(lesion);
                self.push_screen(Screen::LesionForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_lesion(&mut self) {
        let Some(id) = self.selected_lesion_id() else {
            self.status = "No hay lesión seleccionada".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar lesión",
            "¿Eliminar la lesión seleccionada? El partido asociado no se modifica. (S = sí, N = no)",
            ConfirmAction::DeleteLesion(id),
        ));
    }

    /// Elimina una lesión.
    pub(crate) fn delete_lesion(&mut self, id: i64) -> Result<()> {
        match lesion_service::eliminar(self.db.conn(), id) {
            Ok(()) => {
                self.status = "Lesión eliminada".to_string();
                if matches!(self.current_screen(), Screen::LesionDetail(_)) {
                    self.stack.pop();
                }
                self.lesion_detail = None;
                self.refresh_lesiones()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Guarda el formulario de lesión.
    pub(crate) fn save_lesion_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let lesion = match self.lesion_desde_form() {
            Ok(lesion) => lesion,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };
        if lesion_service::fecha_futura(&lesion) {
            self.status = "Advertencia: la fecha de la lesión es futura".to_string();
        }

        let result = match mode {
            FormMode::Create => {
                let mut nueva = lesion;
                lesion_service::crear(self.db.conn(), &mut nueva).map(|_| ())
            }
            FormMode::Edit => lesion_service::actualizar(self.db.conn(), &lesion),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_lesion = None;
                self.stack.pop();
                self.refresh_lesiones()?;
                if let Screen::LesionDetail(id) = self.current_screen()
                    && let Ok(lesion) = lesion_service::obtener(self.db.conn(), id)
                {
                    self.lesion_detail = Some(lesion);
                }
                self.status = "Lesión guardada".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn lesion_form(&self, lesion: Option<&Lesion>) -> Form {
        let camisetas = camiseta_service::listar(self.db.conn(), &Default::default())
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row.id.to_string(), row.nombre))
            .collect::<Vec<_>>();
        let partidos = partido_service::listar(
            self.db.conn(),
            &PartidoFilter::default(),
            Page::numbered(1, 50),
        )
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            (
                row.id.to_string(),
                format!(
                    "#{} {} {}",
                    row.id,
                    dates::display_datetime(row.fecha_hora),
                    row.rival
                ),
            )
        });
        let mut partido_opciones = vec![("0".to_string(), "Ninguno".to_string())];
        partido_opciones.extend(partidos);

        let camiseta_actual = lesion.map_or(0, |lesion| lesion.camiseta_id);
        let camiseta_valor = if camisetas
            .iter()
            .any(|(value, _)| value == &camiseta_actual.to_string())
        {
            camiseta_actual.to_string()
        } else {
            camisetas
                .first()
                .map(|(value, _)| value.clone())
                .unwrap_or_default()
        };
        let partido_actual = lesion.map_or(0, |lesion| lesion.partido_id.unwrap_or(0));
        let estado_actual = lesion.map_or(EstadoLesion::Activa, |lesion| lesion.estado);

        Form::new(
            if lesion.is_some() {
                "Editar lesión"
            } else {
                "Nueva lesión"
            },
            vec![
                Field::text(
                    "jugador",
                    "Jugador",
                    lesion.map_or_else(String::new, |lesion| lesion.jugador.clone()),
                    128,
                )
                .required(),
                Field::text(
                    "tipo",
                    "Tipo",
                    lesion.map_or_else(String::new, |lesion| lesion.tipo.clone()),
                    100,
                )
                .required(),
                Field::text(
                    "descripcion",
                    "Descripción",
                    lesion.map_or_else(String::new, |lesion| lesion.descripcion.clone()),
                    200,
                )
                .required(),
                Field {
                    key: "fecha",
                    label: "Fecha".to_string(),
                    value: lesion.map_or_else(
                        || dates::display_datetime(dates::now_local()),
                        |lesion| dates::display_datetime(lesion.fecha),
                    ),
                    kind: FieldKind::DateTime,
                    required: true,
                    help: Some("DD/MM/AAAA HH:MM".to_string()),
                },
                Field::select("camiseta_id", "Camiseta", camiseta_valor, camisetas),
                Field::select(
                    "estado",
                    "Estado",
                    estado_actual.as_str().to_string(),
                    EstadoLesion::ALL
                        .iter()
                        .map(|estado| (estado.as_str().to_string(), estado.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "partido_id",
                    "Partido",
                    partido_actual.to_string(),
                    partido_opciones,
                ),
            ],
        )
    }

    fn lesion_desde_form(&self) -> Result<Lesion, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let camiseta_id: i64 = get("camiseta_id")
            .parse()
            .map_err(|_| DomainError::Message("Camiseta inválida".to_string()))?;
        let partido_id: i64 = get("partido_id")
            .parse()
            .map_err(|_| DomainError::Message("Partido inválido".to_string()))?;
        let original = self.form_original_lesion.as_ref();
        Ok(Lesion {
            id: original.map_or(0, |lesion| lesion.id),
            jugador: get("jugador"),
            tipo: get("tipo"),
            descripcion: get("descripcion"),
            fecha: validation::datetime(&get("fecha"), "fecha")?,
            camiseta_id,
            estado: EstadoLesion::from_str(&get("estado"))
                .map_err(|_| DomainError::Message("Estado inválido".to_string()))?,
            partido_id: if partido_id == 0 {
                None
            } else {
                Some(partido_id)
            },
        })
    }
}
