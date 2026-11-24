//! Recordatorios y agenda (IMP-18).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::dates;
use crate::domain::models::FormMode;
use crate::domain::recordatorios::{Periodicidad, Recordatorio};
use crate::services::recordatorios_service;

impl App {
    /// Abre recordatorios.
    pub(crate) fn open_recordatorios(&mut self) -> Result<()> {
        self.refresh_recordatorios()?;
        if !matches!(self.current_screen(), Screen::RecordatoriosList) {
            self.push_screen(Screen::RecordatoriosList);
        }
        Ok(())
    }

    /// Recarga recordatorios.
    pub(crate) fn refresh_recordatorios(&mut self) -> Result<()> {
        match recordatorios_service::listar(self.db.conn()) {
            Ok(mut lista) => {
                if let Some(tema) = &self.recordatorios_tema {
                    let tema = tema.to_lowercase();
                    lista.retain(|item| item.tematica.trim().to_lowercase() == tema);
                }
                self.recordatorios = lista;
                self.recordatorios_table
                    .select(if self.recordatorios.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
            }
            Err(error) => self.status = format!("No se pudieron cargar los recordatorios: {error}"),
        }
        Ok(())
    }

    /// Teclas del listado.
    pub(crate) fn handle_recordatorios_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(
                    &mut self.recordatorios_table,
                    self.recordatorios.len(),
                    1,
                );
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(
                    &mut self.recordatorios_table,
                    self.recordatorios.len(),
                    -1,
                );
            }
            KeyCode::Char('n') => {
                self.recordatorio_edit_id = None;
                self.form = Some(recordatorio_form(None));
                self.push_screen(Screen::RecordatorioForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(item) = self
                    .recordatorios_table
                    .selected()
                    .and_then(|indice| self.recordatorios.get(indice))
                    .cloned()
                {
                    self.recordatorio_edit_id = Some(item.id);
                    self.form = Some(recordatorio_form(Some(&item)));
                    self.push_screen(Screen::RecordatorioForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .recordatorios_table
                    .selected()
                    .and_then(|indice| self.recordatorios.get(indice))
                    .map(|item| item.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar recordatorio",
                        "¿Eliminar este recordatorio?",
                        ConfirmAction::DeleteRecordatorio(id),
                    ));
                }
            }
            KeyCode::Char('c') => {
                if let Some(item) = self
                    .recordatorios_table
                    .selected()
                    .and_then(|indice| self.recordatorios.get(indice))
                    .cloned()
                {
                    let completado = !item.completado;
                    match recordatorios_service::completar(self.db.conn(), &item, completado) {
                        Ok(()) => {
                            self.refresh_recordatorios()?;
                            self.status = if completado {
                                "Recordatorio completado".to_string()
                            } else {
                                "Recordatorio reabierto".to_string()
                            };
                        }
                        Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                    }
                }
            }
            KeyCode::Char('a') => self.open_agenda()?,
            KeyCode::Char('f') => self.siguiente_tema_recordatorios()?,
            KeyCode::Char('x') => {
                match recordatorios_service::exportar(self.db.conn(), &self.paths.export_dir) {
                    Ok(ruta) => {
                        self.modal = Some(Modal::Info(format!(
                            "Recordatorios exportados a:
{}",
                            ruta.display()
                        )));
                    }
                    Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                }
            }
            KeyCode::Char('i') => self.importar_recordatorios(false),
            KeyCode::Char('I') => {
                self.modal = Some(Modal::confirm(
                    "Importar recordatorios",
                    format!(
                        "Se borrarán los recordatorios actuales y se cargarán los de Importaciones/{}. Se hace un backup antes. (S = sí, N = no)",
                        recordatorios_service::ARCHIVO
                    ),
                    ConfirmAction::ImportarRecordatoriosReemplazo,
                ));
            }
            KeyCode::Char('r') => {
                self.refresh_recordatorios()?;
                self.status = "Recordatorios actualizados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `f`: todas → cada temática existente → todas (C pedía escribirla).
    fn siguiente_tema_recordatorios(&mut self) -> Result<()> {
        let todos = match crate::db::recordatorio_repo::list(self.db.conn()) {
            Ok(todos) => todos,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };
        let temas = recordatorios_service::tematicas(&todos);
        let siguiente = match &self.recordatorios_tema {
            None => 0,
            Some(actual) => temas
                .iter()
                .position(|t| t.to_lowercase() == actual.to_lowercase())
                .map_or(temas.len(), |i| i + 1),
        };
        self.recordatorios_tema = temas.get(siguiente).cloned();
        self.refresh_recordatorios()?;
        self.status = match &self.recordatorios_tema {
            Some(tema) => format!(
                "Temática: {tema} ({} recordatorios)",
                self.recordatorios.len()
            ),
            None if temas.is_empty() => "No hay recordatorios con temática".to_string(),
            None => "Todas las temáticas".to_string(),
        };
        Ok(())
    }

    /// Importa `Importaciones/recordatorios.json` (`i` une, `I` reemplaza con
    /// backup previo).
    pub(crate) fn importar_recordatorios(&mut self, reemplazar: bool) {
        let ruta = self.paths.import_dir.join(recordatorios_service::ARCHIVO);
        if !ruta.exists() {
            self.modal = Some(Modal::Error(format!("No existe {}", ruta.display())));
            return;
        }
        let previo = if reemplazar {
            match self.backup_pre_import() {
                Some(previo) => format!(
                    "
Backup previo: {}",
                    previo.display()
                ),
                None => return,
            }
        } else {
            String::new()
        };
        match recordatorios_service::importar(self.db.conn_mut(), &ruta, reemplazar) {
            Ok((importados, errores)) => {
                let _ = self.refresh_recordatorios();
                let mut texto = format!(
                    "{importados} recordatorios importados ({}).",
                    if reemplazar { "reemplazados" } else { "unidos" }
                );
                for error in errores.iter().take(5) {
                    texto.push_str(&format!(
                        "
  {error}"
                    ));
                }
                if errores.len() > 5 {
                    texto.push_str(&format!(
                        "
  … y {} más",
                        errores.len() - 5
                    ));
                }
                self.modal = Some(Modal::Info(format!("{texto}{previo}")));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Abre la agenda.
    pub(crate) fn open_agenda(&mut self) -> Result<()> {
        match recordatorios_service::agenda(self.db.conn()) {
            Ok(agenda) => {
                self.agenda = agenda;
                self.agenda_table.select(Some(0));
                self.push_screen(Screen::Agenda);
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Teclas de la agenda.
    pub(crate) fn handle_agenda_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.agenda_table, self.agenda.len(), 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.agenda_table, self.agenda.len(), -1);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Guarda el formulario.
    pub(crate) fn save_recordatorio_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let Some(fecha) = form.value("fecha").and_then(dates::parse_date) else {
            self.form = Some(form);
            self.modal = Some(Modal::Error(
                "La fecha debe tener el formato DD/MM/AAAA".to_string(),
            ));
            return Ok(());
        };
        let hora = form.value("hora").unwrap_or("").trim().to_string();
        let item = Recordatorio {
            id: self.recordatorio_edit_id.unwrap_or(0),
            titulo: form.value("titulo").unwrap_or("").to_string(),
            nota: form.value("nota").unwrap_or("").to_string(),
            fecha,
            hora,
            periodicidad: Periodicidad::from_i64(
                form.value("periodicidad")
                    .and_then(|valor| valor.parse::<i64>().ok())
                    .unwrap_or(0),
            ),
            tematica: form.value("tematica").unwrap_or("").to_string(),
            completado: false,
            ultima_gen: String::new(),
        };
        let resultado = match mode {
            FormMode::Create => recordatorios_service::crear(self.db.conn(), &item).map(|_| ()),
            FormMode::Edit => recordatorios_service::actualizar(self.db.conn(), &item),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_recordatorios()?;
                self.go_back();
                self.status = "Recordatorio guardado".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn recordatorio_form(item: Option<&Recordatorio>) -> Form {
    let hoy = dates::today();
    let fecha = item.map_or(hoy, |item| item.fecha);
    let hora = item.map_or("09:00", |item| item.hora.as_str()).to_string();
    let periodicidad = item
        .map_or(Periodicidad::UnaVez, |item| item.periodicidad)
        .as_i64()
        .to_string();
    let campos = vec![
        Field::text(
            "titulo",
            "Título",
            item.map_or(String::new(), |item| item.titulo.clone()),
            120,
        )
        .required(),
        Field {
            key: "fecha",
            label: "Fecha".to_string(),
            value: dates::display_date(fecha),
            kind: FieldKind::Date,
            required: true,
            help: None,
        },
        Field {
            key: "hora",
            label: "Hora (HH:MM)".to_string(),
            value: hora,
            kind: FieldKind::Time,
            required: false,
            help: None,
        },
        Field::select(
            "periodicidad",
            "Periodicidad",
            periodicidad,
            vec![
                ("0".to_string(), "Una vez".to_string()),
                ("1".to_string(), "Diaria".to_string()),
                ("2".to_string(), "Semanal".to_string()),
                ("3".to_string(), "Mensual".to_string()),
            ],
        ),
        Field::text(
            "tematica",
            "Temática",
            item.map_or(String::new(), |item| item.tematica.clone()),
            60,
        ),
        Field::text(
            "nota",
            "Nota",
            item.map_or(String::new(), |item| item.nota.clone()),
            240,
        ),
    ];
    Form::new("Recordatorio", campos)
}
