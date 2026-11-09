//! Tiendas (IMP-20).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, Form, Modal, Screen};
use crate::domain::models::FormMode;
use crate::domain::tiendas::Tienda;
use crate::services::tiendas_service;

impl App {
    /// Abre tiendas.
    pub(crate) fn open_tiendas(&mut self) -> Result<()> {
        self.refresh_tiendas()?;
        if !matches!(self.current_screen(), Screen::TiendasList) {
            self.push_screen(Screen::TiendasList);
        }
        Ok(())
    }

    /// Recarga tiendas.
    pub(crate) fn refresh_tiendas(&mut self) -> Result<()> {
        match tiendas_service::listar(self.db.conn()) {
            Ok(tiendas) => {
                self.tiendas = tiendas;
                self.tiendas_table.select(if self.tiendas.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron cargar las tiendas: {error}"),
        }
        Ok(())
    }

    /// Teclas del listado.
    pub(crate) fn handle_tiendas_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.tiendas.len();
                if total > 0 {
                    let indice = (self.tiendas_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.tiendas_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self.tiendas_table.selected().unwrap_or(0).saturating_sub(1);
                self.tiendas_table.select(Some(indice));
            }
            KeyCode::Char('n') => self.open_tienda_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(tienda) = self
                    .tiendas_table
                    .selected()
                    .and_then(|indice| self.tiendas.get(indice))
                    .cloned()
                {
                    self.tienda_edit_id = Some(tienda.id);
                    self.form = Some(tienda_form(Some(&tienda)));
                    self.push_screen(Screen::TiendaForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .tiendas_table
                    .selected()
                    .and_then(|indice| self.tiendas.get(indice))
                    .map(|tienda| tienda.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar tienda",
                        "¿Eliminar esta tienda?",
                        ConfirmAction::DeleteTienda(id),
                    ));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_tiendas()?;
                self.status = "Tiendas actualizadas".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn open_tienda_form(&mut self, mode: FormMode) -> Result<()> {
        if mode == FormMode::Create {
            self.tienda_edit_id = None;
            self.form = Some(tienda_form(None));
        }
        self.push_screen(Screen::TiendaForm(mode));
        Ok(())
    }

    /// Guarda el formulario.
    pub(crate) fn save_tienda_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let texto = |clave: &str| form.value(clave).unwrap_or("").to_string();
        let tienda = Tienda {
            id: self.tienda_edit_id.unwrap_or(0),
            nombre: texto("nombre"),
            tipo: texto("tipo"),
            url: texto("url"),
            telefono: texto("telefono"),
            instagram: texto("instagram"),
            productos: texto("productos"),
            rango_precio: texto("rango_precio"),
            tiene_envio: form.value("tiene_envio") == Some("Sí"),
            favorito: form.value("favorito") == Some("Sí"),
            notas: texto("notas"),
        };
        let resultado = match mode {
            FormMode::Create => tiendas_service::crear(self.db.conn(), &tienda).map(|_| ()),
            FormMode::Edit => tiendas_service::actualizar(self.db.conn(), &tienda),
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_tiendas()?;
                self.go_back();
                self.status = "Tienda guardada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn tienda_form(tienda: Option<&Tienda>) -> Form {
    let valor = |clave: &str| {
        tienda.map_or_else(String::new, |tienda| match clave {
            "nombre" => tienda.nombre.clone(),
            "tipo" => tienda.tipo.clone(),
            "url" => tienda.url.clone(),
            "telefono" => tienda.telefono.clone(),
            "instagram" => tienda.instagram.clone(),
            "productos" => tienda.productos.clone(),
            "rango_precio" => tienda.rango_precio.clone(),
            "notas" => tienda.notas.clone(),
            _ => String::new(),
        })
    };
    let campos = vec![
        Field::text("nombre", "Nombre", valor("nombre"), 100).required(),
        Field::text("tipo", "Tipo (física/online)", valor("tipo"), 40),
        Field::text("url", "URL", valor("url"), 200),
        Field::text("telefono", "Teléfono", valor("telefono"), 40),
        Field::text("instagram", "Instagram", valor("instagram"), 80),
        Field::text("productos", "Productos", valor("productos"), 160),
        Field::text("rango_precio", "Rango de precio", valor("rango_precio"), 40),
        Field::boolean(
            "tiene_envio",
            "Tiene envío",
            tienda.is_some_and(|t| t.tiene_envio),
        ),
        Field::boolean("favorito", "Favorita", tienda.is_some_and(|t| t.favorito)),
        Field::text("notas", "Notas", valor("notas"), 240),
    ];
    Form::new("Tienda", campos)
}
