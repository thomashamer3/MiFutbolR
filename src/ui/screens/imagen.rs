//! Visor de imagen a pantalla completa (ratatui-image).

use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::App;
use crate::ui::images;
use crate::ui::theme::palette;

/// Dibuja la imagen abierta.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let Some(ruta) = app.imagen_abierta.clone() else {
        return;
    };
    images::dibujar_grande(frame, area, &mut app.images, Path::new(&ruta), &colors);
}
