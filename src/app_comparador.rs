//! Navegación del comparador A/B (IMP-14).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Screen};
use crate::domain::comparador::TipoComparacion;
use crate::services::comparador_service;

impl App {
    /// Abre el comparador.
    pub(crate) fn open_comparador(&mut self) -> Result<()> {
        self.cargar_comparador()?;
        if !matches!(self.current_screen(), Screen::Comparador) {
            self.push_screen(Screen::Comparador);
        }
        Ok(())
    }

    /// Recarga opciones y limpia el resultado.
    pub(crate) fn cargar_comparador(&mut self) -> Result<()> {
        match comparador_service::opciones(self.db.conn(), self.comparador_tipo) {
            Ok(opciones) => {
                self.comparador_opciones = opciones;
                self.comparador_a = 0;
                self.comparador_b = usize::from(self.comparador_opciones.len() > 1);
                self.comparador_activo = 0;
                self.comparador_resultado = None;
            }
            Err(error) => self.status = format!("No se pudo cargar el comparador: {error}"),
        }
        Ok(())
    }

    /// Teclas del comparador.
    pub(crate) fn handle_comparador_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('1') => {
                self.comparador_tipo = TipoComparacion::Camiseta;
                self.cargar_comparador()?;
            }
            KeyCode::Char('2') => {
                self.comparador_tipo = TipoComparacion::Cancha;
                self.cargar_comparador()?;
            }
            KeyCode::Tab => {
                self.comparador_activo = 1 - self.comparador_activo;
            }
            KeyCode::Down | KeyCode::Char('j') => self.mover_comparador(1),
            KeyCode::Up | KeyCode::Char('k') => self.mover_comparador(-1),
            KeyCode::Enter => self.comparar_actuales()?,
            KeyCode::Char('r') => {
                self.cargar_comparador()?;
                self.status = "Comparador recargado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn mover_comparador(&mut self, delta: i32) {
        let total = self.comparador_opciones.len();
        if total == 0 {
            return;
        }
        let actual = if self.comparador_activo == 0 {
            self.comparador_a
        } else {
            self.comparador_b
        };
        let nuevo = (actual as i32 + delta).clamp(0, total as i32 - 1) as usize;
        if self.comparador_activo == 0 {
            self.comparador_a = nuevo;
        } else {
            self.comparador_b = nuevo;
        }
    }

    fn comparar_actuales(&mut self) -> Result<()> {
        let (Some(a), Some(b)) = (
            self.comparador_opciones.get(self.comparador_a),
            self.comparador_opciones.get(self.comparador_b),
        ) else {
            self.status = "Hacen falta al menos dos opciones para comparar".to_string();
            return Ok(());
        };
        match comparador_service::comparar(self.db.conn(), self.comparador_tipo, a.id, b.id) {
            Ok(comparacion) => {
                self.comparador_resultado = Some(comparacion);
                self.status = "Comparación calculada".to_string();
            }
            Err(error) => {
                self.modal = Some(crate::app::Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}
