//! Amigos y ranking (IMP-22).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::amigos::Amigo;
use crate::domain::models::FormMode;
use crate::services::amigos_service;

impl App {
    /// Abre el ranking.
    pub(crate) fn open_amigos(&mut self) -> Result<()> {
        self.refresh_amigos()?;
        if !matches!(self.current_screen(), Screen::AmigosList) {
            self.push_screen(Screen::AmigosList);
        }
        Ok(())
    }

    /// Recarga amigos y ranking.
    pub(crate) fn refresh_amigos(&mut self) -> Result<()> {
        match amigos_service::listar(self.db.conn()) {
            Ok(amigos) => self.amigos = amigos,
            Err(error) => self.status = format!("No se pudieron cargar los amigos: {error}"),
        }
        match amigos_service::ranking(self.db.conn()) {
            Ok(ranking) => {
                self.amigos_ranking = ranking;
                self.amigos_table.select(if self.amigos_ranking.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudo cargar el ranking: {error}"),
        }
        Ok(())
    }

    /// Teclas del ranking.
    pub(crate) fn handle_amigos_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.amigos_ranking.len();
                if total > 0 {
                    let indice = (self.amigos_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.amigos_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.amigos_table.selected().unwrap_or(0).saturating_sub(1);
                self.amigos_table.select(Some(indice));
            }
            KeyCode::Char('n') => {
                self.amigo_edit_id = None;
                self.form = Some(amigo_form(None));
                self.push_screen(Screen::AmigoForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(amigo) = self.amigo_seleccionado() {
                    self.amigo_edit_id = Some(amigo.id);
                    self.form = Some(amigo_form(Some(&amigo)));
                    self.push_screen(Screen::AmigoForm(FormMode::Edit));
                } else {
                    self.status = "El usuario local no se edita desde el ranking".to_string();
                }
            }
            KeyCode::Char('d') => {
                if let Some(amigo) = self.amigo_seleccionado() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar amigo",
                        format!("¿Eliminar a {}?", amigo.nombre),
                        ConfirmAction::DeleteAmigo(amigo.id),
                    ));
                } else {
                    self.status = "El usuario local no se elimina".to_string();
                }
            }
            KeyCode::Char('r') => {
                self.refresh_amigos()?;
                self.status = "Ranking actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn amigo_seleccionado(&self) -> Option<Amigo> {
        let fila = self
            .amigos_table
            .selected()
            .and_then(|indice| self.amigos_ranking.get(indice))?;
        if fila.local {
            return None;
        }
        self.amigos
            .iter()
            .find(|amigo| amigo.nombre == fila.nombre)
            .cloned()
    }

    /// Guarda el formulario.
    pub(crate) fn save_amigo_form(&mut self, mode: FormMode) -> Result<()> {
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
        let amigo = Amigo {
            id: self.amigo_edit_id.unwrap_or(0),
            nombre: form.value("nombre").unwrap_or("").to_string(),
            posicion: form.value("posicion").unwrap_or("").to_string(),
            goles: entero("goles"),
            asistencias: entero("asistencias"),
            partidos: entero("partidos"),
            mejor_rendimiento: entero("mejor_rendimiento"),
            notas: form.value("notas").unwrap_or("").to_string(),
        };
        let resultado = match mode {
            FormMode::Create => amigos_service::crear(self.db.conn(), &amigo).map(|_| ()),
            FormMode::Edit => amigos_service::actualizar(self.db.conn(), &amigo),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_amigos()?;
                self.go_back();
                self.status = "Amigo guardado".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn amigo_form(amigo: Option<&Amigo>) -> Form {
    let valor = |clave: &str| {
        amigo.map_or_else(String::new, |amigo| match clave {
            "nombre" => amigo.nombre.clone(),
            "posicion" => amigo.posicion.clone(),
            "notas" => amigo.notas.clone(),
            _ => String::new(),
        })
    };
    let numero = |clave: &str, defecto: i64| {
        amigo.map_or(defecto, |amigo| match clave {
            "goles" => amigo.goles,
            "asistencias" => amigo.asistencias,
            "partidos" => amigo.partidos,
            "mejor_rendimiento" => amigo.mejor_rendimiento,
            _ => defecto,
        })
    };
    let campo = |clave: &'static str, etiqueta: &str, valor: i64| Field {
        key: clave,
        label: etiqueta.to_string(),
        value: valor.to_string(),
        kind: FieldKind::Int { min: 0, max: 9999 },
        required: false,
        help: None,
    };
    let campos = vec![
        Field::text("nombre", "Nombre", valor("nombre"), 100).required(),
        Field::text("posicion", "Posición", valor("posicion"), 40),
        campo("goles", "Goles", numero("goles", 0)),
        campo("asistencias", "Asistencias", numero("asistencias", 0)),
        campo("partidos", "Partidos", numero("partidos", 0)),
        campo(
            "mejor_rendimiento",
            "Mejor rendimiento",
            numero("mejor_rendimiento", 0),
        ),
        Field::text("notas", "Notas", valor("notas"), 240),
    ];
    Form::new("Amigo", campos)
}
