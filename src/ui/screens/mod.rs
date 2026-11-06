//! Pantallas de la TUI.

pub mod ajustes;
pub mod ayuda;
pub mod botines;
pub mod camisetas;
pub mod canchas;
pub mod carrera;
pub mod equipos;
pub mod estadisticas;
pub mod financiamiento;
pub mod lesiones;
pub mod main_menu;
pub mod partidos;
pub mod resumen;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::{App, Module};
use crate::ui::theme::palette;

/// Pantalla provisional de un módulo todavía no implementado.
///
/// Se reemplaza por la pantalla real a medida que avanza el roadmap (§23).
pub fn module_placeholder(frame: &mut Frame, area: Rect, app: &App, module: Module) {
    let colors = palette(app.settings.theme);
    let block = Block::bordered()
        .title(format!(" {} ", module.label()))
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));

    let lines = vec![
        Line::from(Span::styled(
            module.description(),
            Style::default().fg(colors.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Este módulo todavía no está implementado en esta fase del roadmap.",
            Style::default()
                .fg(colors.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "La especificación completa define listados, formularios y validaciones.",
            Style::default().fg(colors.muted),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Esc: volver   q: volver   ?: ayuda",
            Style::default().fg(colors.muted),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
