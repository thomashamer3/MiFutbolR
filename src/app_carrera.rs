//! Navegación y formularios del módulo Carrera Futbolística.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::db::helpers::Page;
use crate::domain::dates;
use crate::domain::models::{CarreraIdentidad, FormMode, PartidoFilter};
use crate::domain::validation::DomainError;
use crate::services::carrera_service;
use crate::services::partido_service;

/// Secciones del hub de carrera.
pub const CARRERA_SECCIONES: &[(&str, &str)] = &[
    ("Resumen", "Totales, promedios, rachas, títulos y podios."),
    (
        "Ficha de identidad",
        "Nombre, edad, pie hábil, medidas y objetivos.",
    ),
    ("Hitos", "Partidos que marcaron tu carrera."),
    ("Timeline", "Línea de tiempo de partidos, hitos y lesiones."),
    ("Hall of Fame", "Top 5 partidos, rachas y logros."),
    ("Perfil dinámico", "Últimos 5 partidos vs los 5 anteriores."),
    (
        "Modo retro",
        "Partidos de la misma fecha en años anteriores.",
    ),
    ("Logros", "Badges calculados de la carrera."),
];

impl App {
    /// Carga resumen, hitos, timeline y hall of fame.
    pub fn refresh_carrera(&mut self) -> Result<()> {
        self.carrera_resumen = Some(carrera_service::resumen(self.db.conn())?);
        self.carrera_hitos = carrera_service::hitos(self.db.conn())?;
        self.carrera_timeline = carrera_service::timeline(self.db.conn())?;
        self.carrera_hall = Some(carrera_service::hall_of_fame(self.db.conn())?);
        self.carrera_perfil = Some(carrera_service::perfil(self.db.conn())?);
        self.carrera_retro = carrera_service::retro(self.db.conn())?;
        if self.carrera_hitos.is_empty() {
            self.carrera_hitos_table.select(None);
        } else {
            let index = self
                .carrera_hitos_table
                .selected()
                .unwrap_or(0)
                .min(self.carrera_hitos.len() - 1);
            self.carrera_hitos_table.select(Some(index));
        }
        Ok(())
    }

    /// Teclas del hub de carrera.
    pub(crate) fn handle_carrera_menu_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let len = CARRERA_SECCIONES.len() as isize;
                let actual = self.carrera_hub.selected().unwrap_or(0) as isize;
                self.carrera_hub
                    .select(Some((actual + 1).rem_euclid(len) as usize));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = CARRERA_SECCIONES.len() as isize;
                let actual = self.carrera_hub.selected().unwrap_or(0) as isize;
                self.carrera_hub
                    .select(Some((actual - 1).rem_euclid(len) as usize));
            }
            KeyCode::Enter => {
                self.carrera_scroll = 0;
                match self.carrera_hub.selected().unwrap_or(0) {
                    0 => self.push_screen(Screen::CarreraResumen),
                    1 => self.open_carrera_identidad_form()?,
                    2 => self.push_screen(Screen::CarreraHitos),
                    3 => self.push_screen(Screen::CarreraTimeline),
                    4 => self.push_screen(Screen::CarreraHallOfFame),
                    5 => self.push_screen(Screen::CarreraPerfil),
                    6 => self.push_screen(Screen::CarreraRetro),
                    _ => self.push_screen(Screen::CarreraLogros),
                }
            }
            KeyCode::Char('r') => {
                self.refresh_carrera()?;
                self.status = "Carrera recalculada".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas de pantallas de solo lectura.
    pub(crate) fn handle_carrera_scroll_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.carrera_scroll = self.carrera_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.carrera_scroll = self.carrera_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.carrera_scroll = self.carrera_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.carrera_scroll = self.carrera_scroll.saturating_sub(10);
            }
            KeyCode::Home => self.carrera_scroll = 0,
            KeyCode::End => self.carrera_scroll = u16::MAX,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del listado de hitos.
    pub(crate) fn handle_carrera_hitos_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.carrera_hitos.is_empty() {
                    let len = self.carrera_hitos.len() as isize;
                    let actual =
                        self.carrera_hitos_table
                            .selected()
                            .unwrap_or(0)
                            .min(self.carrera_hitos.len() - 1) as isize;
                    self.carrera_hitos_table
                        .select(Some((actual + 1).rem_euclid(len) as usize));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.carrera_hitos.is_empty() {
                    let len = self.carrera_hitos.len() as isize;
                    let actual =
                        self.carrera_hitos_table
                            .selected()
                            .unwrap_or(0)
                            .min(self.carrera_hitos.len() - 1) as isize;
                    self.carrera_hitos_table
                        .select(Some((actual - 1).rem_euclid(len) as usize));
                }
            }
            KeyCode::Char('n') => self.open_carrera_hito_form(None)?,
            KeyCode::Char('e') => {
                if let Some(hito) = self.selected_hito() {
                    self.open_carrera_hito_form(Some(hito))?;
                }
            }
            KeyCode::Char('d') => {
                if let Some(hito) = self.selected_hito() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar hito",
                        format!("¿Eliminar el hito «{}»? (S = sí, N = no)", hito.tipo_hito),
                        ConfirmAction::DeleteHito(hito.id),
                    ));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn selected_hito(&self) -> Option<crate::domain::models::HitoRow> {
        let index = self.carrera_hitos_table.selected()?;
        self.carrera_hitos.get(index).cloned()
    }

    /// Elimina un hito.
    pub(crate) fn delete_carrera_hito(&mut self, id: i64) -> Result<()> {
        match carrera_service::eliminar_hito(self.db.conn(), id) {
            Ok(()) => {
                self.status = "Hito eliminado".to_string();
                self.refresh_carrera()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_carrera_identidad_form(&mut self) -> Result<()> {
        match carrera_service::identidad(self.db.conn()) {
            Ok(identidad) => {
                self.form = Some(identidad_form(&identidad));
                self.carrera_identidad = Some(identidad);
                self.push_screen(Screen::CarreraIdentidadForm);
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Guarda la ficha de identidad.
    pub(crate) fn save_carrera_identidad(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let identidad = match self.identidad_desde_form() {
            Ok(identidad) => identidad,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };
        match carrera_service::guardar_identidad(self.db.conn(), &identidad) {
            Ok(()) => {
                self.form = None;
                self.stack.pop();
                self.status = "Identidad guardada".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn identidad_desde_form(&self) -> Result<CarreraIdentidad, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let numero = |key: &str| -> Result<i64, DomainError> {
            get(key)
                .parse()
                .map_err(|_| DomainError::Message(format!("Valor inválido en {key}")))
        };
        let decimal = |key: &str| -> Result<Option<f64>, DomainError> {
            if get(key).is_empty() {
                Ok(None)
            } else {
                Ok(Some(get(key).parse().map_err(|_| {
                    DomainError::Message(format!("Valor inválido en {key}"))
                })?))
            }
        };
        Ok(CarreraIdentidad {
            nombre: get("nombre"),
            edad: numero("edad")?,
            pie_habil: get("pie_habil"),
            posiciones: get("posiciones"),
            altura_cm: decimal("altura")?,
            peso_kg: decimal("peso")?,
            estilo: get("estilo"),
            dorsal_favorito: numero("dorsal")?,
            objetivos: get("objetivos"),
            historia: get("historia"),
        })
    }

    fn open_carrera_hito_form(
        &mut self,
        hito: Option<crate::domain::models::HitoRow>,
    ) -> Result<()> {
        self.form = Some(self.hito_form(hito.as_ref()));
        self.carrera_hito_edit = hito;
        self.push_screen(Screen::CarreraHitoForm(FormMode::Create));
        Ok(())
    }

    fn hito_form(&self, hito: Option<&crate::domain::models::HitoRow>) -> Form {
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
        })
        .collect::<Vec<_>>();
        let partido_actual = hito.map_or_else(
            || {
                partidos
                    .first()
                    .map(|(valor, _)| valor.clone())
                    .unwrap_or_default()
            },
            |hito| hito.partido_id.to_string(),
        );

        Form::new(
            if hito.is_some() {
                "Editar hito"
            } else {
                "Nuevo hito"
            },
            vec![
                Field::select("partido_id", "Partido", partido_actual, partidos),
                Field::text(
                    "tipo_hito",
                    "Tipo de hito",
                    hito.map_or_else(String::new, |hito| hito.tipo_hito.clone()),
                    100,
                )
                .required(),
                Field::text(
                    "nota",
                    "Nota",
                    hito.map_or_else(String::new, |hito| hito.nota.clone()),
                    300,
                ),
            ],
        )
    }

    /// Guarda un hito.
    pub(crate) fn save_carrera_hito(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let get = |key: &str| {
            self.form
                .as_ref()
                .and_then(|form| form.value(key))
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let partido_id: i64 = match get("partido_id").parse() {
            Ok(valor) => valor,
            Err(_) => {
                self.modal = Some(Modal::Error("Partido inválido".to_string()));
                return Ok(());
            }
        };
        let tipo = get("tipo_hito");
        let nota = get("nota");
        let id = self.carrera_hito_edit.as_ref().map(|hito| hito.id);

        let result = match (mode, id) {
            (FormMode::Edit, Some(id)) => {
                carrera_service::actualizar_hito(self.db.conn(), id, partido_id, &tipo, &nota)
            }
            _ => carrera_service::crear_hito(self.db.conn(), partido_id, &tipo, &nota).map(|_| ()),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.carrera_hito_edit = None;
                self.stack.pop();
                self.refresh_carrera()?;
                self.status = "Hito guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }
}

fn identidad_form(identidad: &CarreraIdentidad) -> Form {
    let pie_options = ["Derecho", "Izquierdo", "Ambidiestro"]
        .iter()
        .map(|pie| (pie.to_string(), pie.to_string()))
        .collect();
    let pie_actual = if identidad.pie_habil.trim().is_empty() {
        "Derecho".to_string()
    } else {
        identidad.pie_habil.clone()
    };
    Form::new(
        "Ficha de identidad",
        vec![
            Field::text("nombre", "Nombre", identidad.nombre.clone(), 128),
            Field {
                key: "edad",
                label: "Edad".to_string(),
                value: identidad.edad.to_string(),
                kind: FieldKind::Int { min: 0, max: 120 },
                required: true,
                help: None,
            },
            Field::select("pie_habil", "Pie hábil", pie_actual, pie_options),
            Field::text(
                "posiciones",
                "Posiciones",
                identidad.posiciones.clone(),
                200,
            ),
            Field {
                key: "altura",
                label: "Altura (cm)".to_string(),
                value: identidad
                    .altura_cm
                    .map(|valor| valor.to_string())
                    .unwrap_or_default(),
                kind: FieldKind::Decimal {
                    min: 50.0,
                    max: 250.0,
                },
                required: false,
                help: None,
            },
            Field {
                key: "peso",
                label: "Peso (kg)".to_string(),
                value: identidad
                    .peso_kg
                    .map(|valor| valor.to_string())
                    .unwrap_or_default(),
                kind: FieldKind::Decimal {
                    min: 20.0,
                    max: 250.0,
                },
                required: false,
                help: None,
            },
            Field::text("estilo", "Estilo", identidad.estilo.clone(), 200),
            Field {
                key: "dorsal",
                label: "Dorsal favorito".to_string(),
                value: identidad.dorsal_favorito.to_string(),
                kind: FieldKind::Int { min: 0, max: 99 },
                required: true,
                help: None,
            },
            Field::text("objetivos", "Objetivos", identidad.objetivos.clone(), 320),
            Field::text("historia", "Historia", identidad.historia.clone(), 512),
        ],
    )
}
