//! Navegación y formularios de temporadas (IMP-12).

use anyhow::Result;
use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::dates;
use crate::domain::models::FormMode;
use crate::domain::temporadas::Temporada;
use crate::services::temporadas_service;

impl App {
    /// Abre el listado de temporadas.
    pub(crate) fn open_temporadas(&mut self) -> Result<()> {
        self.refresh_temporadas()?;
        if !matches!(self.current_screen(), Screen::TemporadasList) {
            self.push_screen(Screen::TemporadasList);
        }
        Ok(())
    }

    /// Recarga el listado.
    pub(crate) fn refresh_temporadas(&mut self) -> Result<()> {
        match temporadas_service::listar(self.db.conn()) {
            Ok(temporadas) => {
                self.temporadas = temporadas;
                self.temporadas_table.select(if self.temporadas.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron cargar las temporadas: {error}"),
        }
        Ok(())
    }

    /// Abre el detalle.
    pub(crate) fn open_temporada_detail(&mut self, id: i64) -> Result<()> {
        match temporadas_service::ficha(self.db.conn(), id) {
            Ok(ficha) => {
                self.temporada_detail = Some(ficha);
                self.temporada_scroll = 0;
                self.temporada_comparacion = None;
                self.push_screen(Screen::TemporadaDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Recarga la ficha abierta.
    pub(crate) fn refresh_temporada_detail(&mut self) -> Result<()> {
        let Screen::TemporadaDetail(id) = self.current_screen() else {
            return Ok(());
        };
        match temporadas_service::ficha(self.db.conn(), id) {
            Ok(ficha) => self.temporada_detail = Some(ficha),
            Err(error) => self.status = format!("No se pudo recargar la temporada: {error}"),
        }
        Ok(())
    }

    /// Teclas del listado.
    pub(crate) fn handle_temporadas_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.temporadas.len();
                if total > 0 {
                    let indice = (self.temporadas_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.temporadas_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self
                    .temporadas_table
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                self.temporadas_table.select(Some(indice));
            }
            KeyCode::Enter => {
                if let Some(id) = self
                    .temporadas_table
                    .selected()
                    .and_then(|indice| self.temporadas.get(indice))
                    .map(|temporada| temporada.id)
                {
                    self.open_temporada_detail(id)?;
                }
            }
            KeyCode::Char('n') => self.open_temporada_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(temporada) = self
                    .temporadas_table
                    .selected()
                    .and_then(|indice| self.temporadas.get(indice))
                    .cloned()
                {
                    self.open_temporada_form_edit(&temporada)?;
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .temporadas_table
                    .selected()
                    .and_then(|indice| self.temporadas.get(indice))
                    .map(|temporada| temporada.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar temporada",
                        "Se eliminarán sus fases y asociaciones. ¿Continuar?",
                        ConfirmAction::DeleteTemporada(id),
                    ));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_temporadas()?;
                self.status = "Temporadas actualizadas".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle.
    pub(crate) fn handle_temporada_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(ficha) = self.temporada_detail.clone() else {
            return Ok(false);
        };
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.temporada_scroll = self.temporada_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.temporada_scroll = self.temporada_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.temporada_scroll = self.temporada_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.temporada_scroll = self.temporada_scroll.saturating_sub(10);
            }
            KeyCode::Char('e') => self.open_temporada_form_edit(&ficha.temporada)?,
            KeyCode::Char('d') => {
                self.modal = Some(Modal::confirm(
                    "Eliminar temporada",
                    "Se eliminarán sus fases y asociaciones. ¿Continuar?",
                    ConfirmAction::DeleteTemporada(ficha.temporada.id),
                ));
            }
            KeyCode::Char('c') => {
                let actual = ficha.temporada.id;
                let indice = self.temporadas.iter().position(|t| t.id == actual);
                let otra = indice
                    .and_then(|indice| self.temporadas.get(indice + 1))
                    .map(|temporada| temporada.id);
                match otra {
                    Some(otra) => {
                        match temporadas_service::comparar(self.db.conn(), actual, otra) {
                            Ok(comparacion) => {
                                self.temporada_comparacion = Some(comparacion);
                                self.status = "Comparación calculada".to_string();
                            }
                            Err(error) => {
                                self.modal = Some(Modal::Error(error.to_string()));
                            }
                        }
                    }
                    None => {
                        self.status = "No hay otra temporada para comparar".to_string();
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn open_temporada_form(&mut self, mode: FormMode) -> Result<()> {
        self.temporada_edit_id = None;
        self.form = Some(temporada_form(None));
        self.push_screen(Screen::TemporadaForm(mode));
        Ok(())
    }

    fn open_temporada_form_edit(&mut self, temporada: &Temporada) -> Result<()> {
        self.temporada_edit_id = Some(temporada.id);
        self.form = Some(temporada_form(Some(temporada)));
        self.push_screen(Screen::TemporadaForm(FormMode::Edit));
        Ok(())
    }

    /// Guarda el formulario de temporada.
    pub(crate) fn save_temporada_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let nombre = form.value("nombre").unwrap_or("").to_string();
        let inicio = form.value("fecha_inicio").and_then(dates::parse_date);
        let fin = form.value("fecha_fin").and_then(dates::parse_date);
        let (Some(inicio), Some(fin)) = (inicio, fin) else {
            self.form = Some(form);
            self.modal = Some(Modal::Error(
                "Las fechas deben tener el formato DD/MM/AAAA".to_string(),
            ));
            return Ok(());
        };
        let anio = form
            .value("anio")
            .and_then(|valor| valor.trim().parse::<i64>().ok())
            .filter(|valor| *valor > 0)
            .unwrap_or_else(|| i64::from(inicio.year()));
        let descripcion = form.value("descripcion").unwrap_or("").to_string();
        let ficha = Temporada {
            id: self.temporada_edit_id.unwrap_or(0),
            nombre: nombre.clone(),
            anio,
            fecha_inicio: inicio,
            fecha_fin: fin,
            estado: "Activa".to_string(),
            descripcion,
        };
        let resultado = match mode {
            FormMode::Create => temporadas_service::crear(self.db.conn(), &ficha).map(Some),
            FormMode::Edit => {
                temporadas_service::actualizar(self.db.conn(), &ficha).map(|()| Some(ficha.id))
            }
        };
        match resultado {
            Ok(id) => {
                self.form = None;
                self.refresh_temporadas()?;
                self.go_back();
                match (mode, id) {
                    (FormMode::Edit, _) => {
                        self.refresh_temporada_detail()?;
                    }
                    (FormMode::Create, Some(id)) => {
                        self.open_temporada_detail(id)?;
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
}

fn temporada_form(temporada: Option<&Temporada>) -> Form {
    let (nombre, anio, inicio, fin, descripcion) = temporada.map_or_else(
        || {
            let hoy = dates::today();
            let fin = chrono::NaiveDate::from_ymd_opt(hoy.year(), 12, 31).unwrap_or(hoy);
            (
                String::new(),
                i64::from(hoy.year()),
                hoy,
                fin,
                String::new(),
            )
        },
        |temporada| {
            (
                temporada.nombre.clone(),
                temporada.anio,
                temporada.fecha_inicio,
                temporada.fecha_fin,
                temporada.descripcion.clone(),
            )
        },
    );
    let campo = |key: &'static str, label: &str, valor: String, kind: FieldKind| Field {
        key,
        label: label.to_string(),
        value: valor,
        kind,
        required: true,
        help: None,
    };
    let campos = vec![
        Field::text("nombre", "Nombre", nombre, 100).required(),
        campo(
            "anio",
            "Año",
            anio.to_string(),
            FieldKind::Int {
                min: 1900,
                max: 2100,
            },
        ),
        campo(
            "fecha_inicio",
            "Fecha de inicio",
            dates::display_date(inicio),
            FieldKind::Date,
        ),
        campo(
            "fecha_fin",
            "Fecha de fin",
            dates::display_date(fin),
            FieldKind::Date,
        ),
        Field::text("descripcion", "Descripción", descripcion, 240),
    ];
    Form::new("Temporada", campos)
}
