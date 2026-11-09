//! Efectos sutiles de la TUI con tachyonfx.
//!
//! Los efectos solo alteran colores (nunca símbolos), así que no interfieren
//! con la legibilidad ni con las pruebas de texto.

use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tachyonfx::{EffectManager, Interpolation, fx};

use crate::ui::theme::Palette;

/// Estado de las animaciones de la interfaz.
#[derive(Debug)]
pub struct UiAnimations {
    manager: EffectManager<&'static str>,
    last_tick: Instant,
    /// Permite desactivarlas en capturas o tests visuales.
    pub enabled: bool,
}

impl Default for UiAnimations {
    fn default() -> Self {
        Self {
            manager: EffectManager::default(),
            last_tick: Instant::now(),
            enabled: true,
        }
    }
}

impl UiAnimations {
    /// Fundido de entrada al cambiar de pantalla.
    pub fn fade_in(&mut self, colors: &Palette) {
        if !self.enabled {
            return;
        }
        self.last_tick = Instant::now();
        self.manager.add_effect(fx::fade_from(
            colors.bg,
            colors.bg,
            (210, Interpolation::QuadOut),
        ));
    }

    /// Aparición rápida de modales.
    pub fn modal_in(&mut self, colors: &Palette) {
        if !self.enabled {
            return;
        }
        self.last_tick = Instant::now();
        self.manager.add_effect(fx::fade_from(
            colors.bg,
            colors.bg,
            (130, Interpolation::QuadOut),
        ));
    }

    /// Pulso sutil sobre la fila seleccionada.
    pub fn selection(&mut self, colors: &Palette, area: Rect) {
        if !self.enabled {
            return;
        }
        self.last_tick = Instant::now();
        self.manager.add_effect(
            fx::fade_from_fg(colors.accent, (150, Interpolation::QuadOut)).with_area(area),
        );
    }

    /// Procesa los efectos con el tiempo transcurrido desde el frame anterior.
    pub fn process(&mut self, buffer: &mut Buffer, area: Rect) {
        let ahora = Instant::now();
        let delta = ahora.saturating_duration_since(self.last_tick);
        self.last_tick = ahora;
        self.process_elapsed(buffer, area, delta);
    }

    /// Procesa un delta explícito (útil para tests y capturas).
    pub fn process_elapsed(&mut self, buffer: &mut Buffer, area: Rect, delta: std::time::Duration) {
        self.manager.process_effects(delta, buffer, area);
    }

    /// Indica si hay animaciones activas (para pedir frames más seguidos).
    #[must_use]
    pub fn running(&self) -> bool {
        self.manager.is_running()
    }

    /// Cancela todas las animaciones en curso.
    pub fn clear(&mut self) {
        self.manager = EffectManager::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::Theme;
    use crate::ui::theme::palette;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn fade_completes_and_stops() {
        let mut animaciones = UiAnimations::default();
        let colores = palette(Theme::Dark);
        animaciones.fade_in(&colores);
        assert!(animaciones.running());
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        animaciones.process_elapsed(&mut buffer, area, std::time::Duration::from_millis(500));
        assert!(!animaciones.running());
    }

    #[test]
    fn selection_effect_runs_and_finishes() {
        let mut animaciones = UiAnimations::default();
        let area = Rect::new(0, 0, 20, 1);
        animaciones.selection(&palette(Theme::Dark), area);
        assert!(animaciones.running());
        let mut buffer = Buffer::empty(area);
        animaciones.process_elapsed(&mut buffer, area, std::time::Duration::from_millis(500));
        assert!(!animaciones.running());
    }

    #[test]
    fn disabled_does_not_start_effects() {
        let mut animaciones = UiAnimations {
            enabled: false,
            ..UiAnimations::default()
        };
        animaciones.fade_in(&palette(Theme::Light));
        assert!(!animaciones.running());
    }
}
