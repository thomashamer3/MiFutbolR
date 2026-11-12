//! Panel lateral de vista previa para listados anchos.

use ratatui::Frame;
use std::path::PathBuf;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::ui::images::{ImageCache, dibujar_miniatura};
use crate::ui::theme::Palette;

/// Ancho mínimo para mostrar el panel lateral.
pub const ANCHO_MINIMO: u16 = 110;

/// Divide el área en tabla + panel de vista previa cuando hay ancho suficiente.
#[must_use]
pub fn split(area: Rect) -> Option<(Rect, Rect)> {
    if area.width < ANCHO_MINIMO {
        return None;
    }
    let [tabla, panel] = Layout::horizontal([Constraint::Min(60), Constraint::Length(38)])
        .spacing(1)
        .areas(area);
    Some((tabla, panel))
}

/// Dibuja el panel de vista previa.
pub fn panel(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: &str,
    lineas: Vec<Line<'static>>,
) {
    frame.render_widget(
        Paragraph::new(lineas)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(" {titulo} "))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        area,
    );
}

/// Convierte pares etiqueta/valor en líneas alineadas para la vista previa.
#[must_use]
pub fn kv_lineas(colors: &Palette, pares: &[(&str, String)]) -> Vec<Line<'static>> {
    pares
        .iter()
        .map(|(etiqueta, valor)| {
            Line::from(vec![
                Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                Span::styled(valor.clone(), Style::default().fg(colors.fg)),
            ])
        })
        .collect()
}

/// Dibuja el panel de vista previa con miniatura opcional.
pub fn preview_con_imagen(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: &str,
    lineas: Vec<Line<'static>>,
    ruta_imagen: Option<PathBuf>,
    images: &mut ImageCache,
) {
    let bloque = colors.panel(titulo);
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let con_imagen = ruta_imagen.is_some() && interior.height >= 16;
    let [texto_area, imagen_area] = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(if con_imagen { 8 } else { 0 }),
    ])
    .areas(interior);
    frame.render_widget(Paragraph::new(lineas).wrap(Wrap { trim: true }), texto_area);
    if let (Some(ruta), true) = (ruta_imagen, con_imagen) {
        dibujar_miniatura(frame, imagen_area, images, &ruta, colors);
    }
}
