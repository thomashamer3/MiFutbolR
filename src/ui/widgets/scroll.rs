//! Barra de desplazamiento vertical reutilizable.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::ui::theme::Palette;

/// Dibuja una barra vertical solo si el contenido excede el alto visible.
pub fn vertical(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    total: usize,
    position: usize,
    viewport: usize,
) {
    if total <= viewport || area.width == 0 || area.height == 0 {
        return;
    }
    let mut estado = ScrollbarState::new(total)
        .position(position.min(total.saturating_sub(1)))
        .viewport_content_length(viewport);
    let barra = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(colors.accent))
        .track_style(Style::default().fg(colors.border))
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(barra, area, &mut estado);
}
