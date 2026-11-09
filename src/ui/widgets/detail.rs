//! Panel lateral de vista previa para listados anchos.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

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
