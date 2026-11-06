//! Vista previa y exportación del Resumen Compartible.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Modal};
use crate::domain::resumen::ResumenFormato;
use crate::services::resumen_service;

impl App {
    /// Regenera los datos y el texto del resumen.
    pub fn regenerar_resumen(&mut self) -> Result<()> {
        match resumen_service::generar(self.db.conn()) {
            Ok(datos) => {
                self.resumen_texto = resumen_service::render(&datos, self.resumen_formato);
                self.resumen_datos = Some(datos);
                self.resumen_scroll = 0;
            }
            Err(resumen_service::ServiceError::SinDatos) => {
                self.resumen_datos = None;
                self.resumen_texto.clear();
                self.status =
                    "Sin datos suficientes; registra partidos para generar el resumen.".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn cambiar_formato(&mut self, formato: ResumenFormato) -> Result<()> {
        self.resumen_formato = formato;
        self.resumen_scroll = 0;
        if let Some(datos) = &self.resumen_datos {
            self.resumen_texto = resumen_service::render(datos, formato);
        }
        Ok(())
    }

    fn guardar_resumen(&mut self) -> Result<()> {
        if self.resumen_datos.is_none() {
            self.status = "No hay resumen para guardar".to_string();
            return Ok(());
        }
        let dir = self.paths.export_dir.clone();
        match resumen_service::exportar(self.db.conn(), self.resumen_formato, &dir) {
            Ok(ruta) => {
                self.status = format!("Guardado en {}", ruta.display());
                self.modal = Some(Modal::Info(format!(
                    "Resumen guardado en:\n{}",
                    ruta.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Teclas de la vista previa.
    pub(crate) fn handle_resumen_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.resumen_scroll = self.resumen_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.resumen_scroll = self.resumen_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.resumen_scroll = self.resumen_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.resumen_scroll = self.resumen_scroll.saturating_sub(10);
            }
            KeyCode::Home => self.resumen_scroll = 0,
            KeyCode::End => self.resumen_scroll = u16::MAX,
            KeyCode::Char('m') => self.cambiar_formato(ResumenFormato::Markdown)?,
            KeyCode::Char('h') => self.cambiar_formato(ResumenFormato::Html)?,
            KeyCode::Char('t') => self.cambiar_formato(ResumenFormato::Texto)?,
            KeyCode::Char('g') => self.guardar_resumen()?,
            KeyCode::Char('r') => {
                self.regenerar_resumen()?;
                self.status = "Resumen regenerado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
