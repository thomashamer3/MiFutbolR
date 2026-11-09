//! Referencias multimedia (IMP-21).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::media::{MediaItem, TipoMedia};
use crate::domain::models::FormMode;
use crate::services::media_service;

impl App {
    /// Abre multimedia.
    pub(crate) fn open_media(&mut self) -> Result<()> {
        self.refresh_media()?;
        if !matches!(self.current_screen(), Screen::MediaList) {
            self.push_screen(Screen::MediaList);
        }
        Ok(())
    }

    /// Recarga referencias.
    pub(crate) fn refresh_media(&mut self) -> Result<()> {
        let filtro = self.media_filter.map(TipoMedia::as_str);
        match media_service::listar(self.db.conn(), filtro) {
            Ok(media) => {
                self.media = media;
                self.media_table
                    .select(if self.media.is_empty() { None } else { Some(0) });
            }
            Err(error) => self.status = format!("No se pudo cargar multimedia: {error}"),
        }
        Ok(())
    }

    /// Teclas del listado.
    pub(crate) fn handle_media_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.media.len();
                if total > 0 {
                    let indice = (self.media_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.media_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.media_table.selected().unwrap_or(0).saturating_sub(1);
                self.media_table.select(Some(indice));
            }
            KeyCode::Char('n') => {
                self.media_edit_id = None;
                self.form = Some(media_form(None));
                self.push_screen(Screen::MediaForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(item) = self
                    .media_table
                    .selected()
                    .and_then(|indice| self.media.get(indice))
                    .cloned()
                {
                    self.media_edit_id = Some(item.id);
                    self.form = Some(media_form(Some(&item)));
                    self.push_screen(Screen::MediaForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .media_table
                    .selected()
                    .and_then(|indice| self.media.get(indice))
                    .map(|item| item.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar referencia",
                        "¿Eliminar esta referencia multimedia?",
                        ConfirmAction::DeleteMedia(id),
                    ));
                }
            }
            KeyCode::Char('f') => {
                self.media_filter = match self.media_filter {
                    None => Some(TipoMedia::Video),
                    Some(TipoMedia::Video) => Some(TipoMedia::Foto),
                    Some(TipoMedia::Foto) => Some(TipoMedia::Articulo),
                    Some(TipoMedia::Articulo) => Some(TipoMedia::Audio),
                    Some(TipoMedia::Audio) => None,
                };
                self.refresh_media()?;
            }
            KeyCode::Char('r') => {
                self.refresh_media()?;
                self.status = "Multimedia actualizada".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Guarda el formulario.
    pub(crate) fn save_media_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let tipo = TipoMedia::from_codigo(form.value("tipo").unwrap_or("video"));
        let partido = form
            .value("partido_id")
            .and_then(|valor| valor.trim().parse::<i64>().ok())
            .filter(|valor| *valor > 0);
        let item = MediaItem {
            id: self.media_edit_id.unwrap_or(0),
            partido_id: partido,
            tipo,
            titulo: form.value("titulo").unwrap_or("").to_string(),
            url: form.value("url").unwrap_or("").to_string(),
            descripcion: form.value("descripcion").unwrap_or("").to_string(),
            fecha: String::new(),
        };
        let resultado = match mode {
            FormMode::Create => media_service::crear(self.db.conn(), &item).map(|_| ()),
            FormMode::Edit => media_service::actualizar(self.db.conn(), &item),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_media()?;
                self.go_back();
                self.status = "Referencia guardada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn media_form(item: Option<&MediaItem>) -> Form {
    let valor = |clave: &str| {
        item.map_or_else(String::new, |item| match clave {
            "titulo" => item.titulo.clone(),
            "url" => item.url.clone(),
            "descripcion" => item.descripcion.clone(),
            _ => String::new(),
        })
    };
    let tipo = item.map_or("video", |item| item.tipo.as_str()).to_string();
    let partido = item
        .and_then(|item| item.partido_id)
        .map_or(String::new(), |id| id.to_string());
    let campos = vec![
        Field::select(
            "tipo",
            "Tipo",
            tipo,
            vec![
                ("video".to_string(), "Video".to_string()),
                ("foto".to_string(), "Foto".to_string()),
                ("articulo".to_string(), "Artículo".to_string()),
                ("audio".to_string(), "Audio".to_string()),
            ],
        ),
        Field::text("titulo", "Título", valor("titulo"), 160).required(),
        Field::text("url", "URL", valor("url"), 300),
        Field {
            key: "partido_id",
            label: "Partido (0 = ninguno)".to_string(),
            value: partido,
            kind: FieldKind::Int {
                min: 0,
                max: 999_999,
            },
            required: false,
            help: None,
        },
        Field::text("descripcion", "Descripción", valor("descripcion"), 240),
    ];
    Form::new("Multimedia", campos)
}
