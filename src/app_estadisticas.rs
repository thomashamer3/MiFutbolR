//! Navegación del hub y del detalle de estadísticas.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::app::{App, Modal, Screen};
use crate::domain::models::{EstadisticaKind, Estadisticas};
use crate::services::estadisticas_service;

impl App {
    /// Carga (o recarga) el paquete de estadísticas.
    pub fn load_estadisticas(&mut self) -> Result<()> {
        match estadisticas_service::cargar(self.db.conn()) {
            Ok(estadisticas) => {
                self.estadisticas = Some(estadisticas);
                self.estadisticas_scroll = 0;
                let mut seleccion = ListState::default();
                seleccion.select(Some(self.estadisticas_hub.selected().unwrap_or(0)));
                self.estadisticas_hub = seleccion;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Teclas del hub.
    pub(crate) fn handle_estadisticas_hub_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let len = EstadisticaKind::ALL.len() as isize;
                let actual = self.estadisticas_hub.selected().unwrap_or(0) as isize;
                self.estadisticas_hub
                    .select(Some((actual + 1).rem_euclid(len) as usize));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let len = EstadisticaKind::ALL.len() as isize;
                let actual = self.estadisticas_hub.selected().unwrap_or(0) as isize;
                self.estadisticas_hub
                    .select(Some((actual - 1).rem_euclid(len) as usize));
            }
            KeyCode::Enter => {
                let kind = EstadisticaKind::ALL
                    .get(self.estadisticas_hub.selected().unwrap_or(0))
                    .copied()
                    .unwrap_or(EstadisticaKind::Generales);
                self.estadisticas_scroll = 0;
                self.push_screen(Screen::EstadisticasDetalle(kind));
            }
            KeyCode::Char('r') => {
                self.load_estadisticas()?;
                self.status = "Estadísticas recalculadas".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle.
    pub(crate) fn handle_estadisticas_detalle_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.estadisticas_scroll = self.estadisticas_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.estadisticas_scroll = self.estadisticas_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.estadisticas_scroll = self.estadisticas_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.estadisticas_scroll = self.estadisticas_scroll.saturating_sub(10);
            }
            KeyCode::Home => self.estadisticas_scroll = 0,
            KeyCode::End => self.estadisticas_scroll = u16::MAX,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Paquete cargado (o `None`).
    #[must_use]
    pub fn estadisticas(&self) -> Option<&Estadisticas> {
        self.estadisticas.as_ref()
    }
}
