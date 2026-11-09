//! Progresión y planes (IMP-23).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::models::FormMode;
use crate::domain::progresion::{PlanEntrenamiento, ProgresionFila};
use crate::services::progresion_service;

impl App {
    /// Abre progresión.
    pub(crate) fn open_progresion(&mut self) -> Result<()> {
        self.refresh_progresion()?;
        if !matches!(self.current_screen(), Screen::ProgresionList) {
            self.push_screen(Screen::ProgresionList);
        }
        Ok(())
    }

    /// Recarga mediciones, jugadores y planes.
    pub(crate) fn refresh_progresion(&mut self) -> Result<()> {
        match progresion_service::listar(self.db.conn()) {
            Ok(lista) => {
                self.progresion = lista;
                self.progresion_table.select(if self.progresion.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudo cargar la progresión: {error}"),
        }
        self.progresion_jugadores =
            progresion_service::jugadores(self.db.conn()).unwrap_or_default();
        self.progresion_planes = progresion_service::planes(self.db.conn()).unwrap_or_default();
        Ok(())
    }

    /// Abre planes.
    pub(crate) fn open_planes(&mut self) -> Result<()> {
        self.refresh_planes()?;
        if !matches!(self.current_screen(), Screen::PlanesList) {
            self.push_screen(Screen::PlanesList);
        }
        Ok(())
    }

    /// Recarga planes.
    pub(crate) fn refresh_planes(&mut self) -> Result<()> {
        match progresion_service::planes(self.db.conn()) {
            Ok(planes) => {
                self.planes = planes;
                self.progresion_planes = self.planes.clone();
                self.planes_table.select(if self.planes.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron cargar los planes: {error}"),
        }
        Ok(())
    }

    /// Teclas de mediciones.
    pub(crate) fn handle_progresion_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.progresion.len();
                if total > 0 {
                    let indice = (self.progresion_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.progresion_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self
                    .progresion_table
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                self.progresion_table.select(Some(indice));
            }
            KeyCode::Char('n') => {
                if self.progresion_jugadores.is_empty() {
                    self.status = "Antes de medir cargá un equipo con jugadores".to_string();
                } else {
                    self.progresion_edit_id = None;
                    self.form = Some(progresion_form(
                        None,
                        &self.progresion_jugadores,
                        &self.progresion_planes,
                    ));
                    self.push_screen(Screen::ProgresionForm(FormMode::Create));
                }
            }
            KeyCode::Char('e') => {
                if let Some(fila) = self
                    .progresion_table
                    .selected()
                    .and_then(|indice| self.progresion.get(indice))
                    .cloned()
                {
                    self.progresion_edit_id = Some(fila.id);
                    self.form = Some(progresion_form(
                        Some(&fila),
                        &self.progresion_jugadores,
                        &self.progresion_planes,
                    ));
                    self.push_screen(Screen::ProgresionForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .progresion_table
                    .selected()
                    .and_then(|indice| self.progresion.get(indice))
                    .map(|fila| fila.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar medición",
                        "¿Eliminar esta medición semanal?",
                        ConfirmAction::DeleteProgresion(id),
                    ));
                }
            }
            KeyCode::Char('p') => self.open_planes()?,
            KeyCode::Char('r') => {
                self.refresh_progresion()?;
                self.status = "Progresión actualizada".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas de planes.
    pub(crate) fn handle_planes_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.planes.len();
                if total > 0 {
                    let indice = (self.planes_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.planes_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.planes_table.selected().unwrap_or(0).saturating_sub(1);
                self.planes_table.select(Some(indice));
            }
            KeyCode::Char('n') => {
                self.plan_edit_id = None;
                self.form = Some(plan_form(None));
                self.push_screen(Screen::PlanForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(plan) = self
                    .planes_table
                    .selected()
                    .and_then(|indice| self.planes.get(indice))
                    .cloned()
                {
                    self.plan_edit_id = Some(plan.id);
                    self.form = Some(plan_form(Some(&plan)));
                    self.push_screen(Screen::PlanForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .planes_table
                    .selected()
                    .and_then(|indice| self.planes.get(indice))
                    .map(|plan| plan.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar plan",
                        "Las mediciones quedarán sin plan. ¿Continuar?",
                        ConfirmAction::DeletePlan(id),
                    ));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_planes()?;
                self.status = "Planes actualizados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Guarda una medición.
    pub(crate) fn save_progresion_form(&mut self, mode: FormMode) -> Result<()> {
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
        let jugador_id = entero("jugador_id");
        let plan_id = entero("plan_id");
        let fila = ProgresionFila {
            id: self.progresion_edit_id.unwrap_or(0),
            jugador_id,
            jugador: String::new(),
            plan_id: if plan_id > 0 { Some(plan_id) } else { None },
            plan: String::new(),
            semana: entero("semana"),
            ataque: entero("ataque"),
            defensa: entero("defensa"),
            resistencia: entero("resistencia"),
            velocidad: entero("velocidad"),
            tecnica: entero("tecnica"),
        };
        let resultado = match mode {
            FormMode::Create => progresion_service::crear(self.db.conn(), &fila).map(|_| ()),
            FormMode::Edit => progresion_service::actualizar(self.db.conn(), &fila),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_progresion()?;
                self.go_back();
                self.status = "Medición guardada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }

    /// Guarda un plan.
    pub(crate) fn save_plan_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let plan = PlanEntrenamiento {
            id: self.plan_edit_id.unwrap_or(0),
            nombre: form.value("nombre").unwrap_or("").to_string(),
            descripcion: form.value("descripcion").unwrap_or("").to_string(),
            duracion_semanas: form
                .value("duracion_semanas")
                .and_then(|valor| valor.trim().parse::<i64>().ok())
                .unwrap_or(4),
            sesiones_por_semana: form
                .value("sesiones_por_semana")
                .and_then(|valor| valor.trim().parse::<i64>().ok())
                .unwrap_or(3),
        };
        let resultado = match mode {
            FormMode::Create => progresion_service::crear_plan(self.db.conn(), &plan).map(|_| ()),
            FormMode::Edit => progresion_service::actualizar_plan(self.db.conn(), &plan),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_planes()?;
                self.go_back();
                self.status = "Plan guardado".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn numero(key: &'static str, label: &str, valor: i64, max: i64) -> Field {
    Field {
        key,
        label: label.to_string(),
        value: valor.to_string(),
        kind: FieldKind::Int { min: 0, max },
        required: true,
        help: None,
    }
}

fn progresion_form(
    fila: Option<&ProgresionFila>,
    jugadores: &[(i64, String)],
    planes: &[PlanEntrenamiento],
) -> Form {
    let jugador_actual = fila.map_or(0, |fila| fila.jugador_id).to_string();
    let plan_actual = fila.and_then(|fila| fila.plan_id).unwrap_or(0).to_string();
    let mut opciones_plan = vec![("0".to_string(), "Sin plan".to_string())];
    opciones_plan.extend(
        planes
            .iter()
            .map(|plan| (plan.id.to_string(), plan.nombre.clone())),
    );
    let opciones_jugador = jugadores
        .iter()
        .map(|(id, nombre)| (id.to_string(), nombre.clone()))
        .collect();
    let campos = vec![
        Field::select("jugador_id", "Jugador", jugador_actual, opciones_jugador),
        Field::select("plan_id", "Plan", plan_actual, opciones_plan),
        numero("semana", "Semana", fila.map_or(1, |fila| fila.semana), 520),
        numero(
            "ataque",
            "Ataque (0-100)",
            fila.map_or(50, |fila| fila.ataque),
            100,
        ),
        numero(
            "defensa",
            "Defensa (0-100)",
            fila.map_or(50, |fila| fila.defensa),
            100,
        ),
        numero(
            "resistencia",
            "Resistencia (0-100)",
            fila.map_or(50, |fila| fila.resistencia),
            100,
        ),
        numero(
            "velocidad",
            "Velocidad (0-100)",
            fila.map_or(50, |fila| fila.velocidad),
            100,
        ),
        numero(
            "tecnica",
            "Técnica (0-100)",
            fila.map_or(50, |fila| fila.tecnica),
            100,
        ),
    ];
    Form::new("Progresión semanal", campos)
}

fn plan_form(plan: Option<&PlanEntrenamiento>) -> Form {
    let campos = vec![
        Field::text(
            "nombre",
            "Nombre",
            plan.map_or(String::new(), |plan| plan.nombre.clone()),
            120,
        )
        .required(),
        Field::text(
            "descripcion",
            "Descripción",
            plan.map_or(String::new(), |plan| plan.descripcion.clone()),
            240,
        ),
        numero(
            "duracion_semanas",
            "Duración (semanas)",
            plan.map_or(4, |plan| plan.duracion_semanas),
            520,
        ),
        numero(
            "sesiones_por_semana",
            "Sesiones por semana",
            plan.map_or(3, |plan| plan.sesiones_por_semana),
            14,
        ),
    ];
    Form::new("Plan de entrenamiento", campos)
}
