//! Entrenador IA (paridad con MiFutbolC).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Modal, Screen};
use crate::domain::dates;
use crate::services::entrenador_service;

impl App {
    /// Abre el Entrenador IA.
    pub(crate) fn open_entrenador(&mut self) -> Result<()> {
        self.refresh_entrenador();
        if !matches!(self.current_screen(), Screen::EntrenadorIa) {
            self.push_screen(Screen::EntrenadorIa);
        }
        Ok(())
    }

    fn refresh_entrenador(&mut self) {
        match entrenador_service::analizar(self.db.conn(), dates::today()) {
            Ok(analisis) => self.entrenador = Some(analisis),
            Err(error) => self.status = format!("No se pudo analizar: {error}"),
        }
    }

    /// Teclas del Entrenador IA.
    pub(crate) fn handle_entrenador_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(tecla @ ('s' | 'i')) => {
                let consejos = self
                    .entrenador
                    .as_ref()
                    .map(|analisis| analisis.consejos.clone())
                    .unwrap_or_default();
                match entrenador_service::registrar(self.db.conn(), &consejos, tecla == 's') {
                    Ok(()) => {
                        self.refresh_entrenador();
                        self.status = if tecla == 's' {
                            "Consejos marcados como seguidos"
                        } else {
                            "Consejos marcados como ignorados"
                        }
                        .to_string();
                    }
                    Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                }
            }
            KeyCode::Char('v') => match entrenador_service::ciclar_nivel(self.db.conn()) {
                Ok(nivel) => {
                    self.refresh_entrenador();
                    self.status = format!("Nivel de intervención: {nivel}");
                }
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            },
            KeyCode::Char('r') => {
                self.refresh_entrenador();
                self.status = "Análisis actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
