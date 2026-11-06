//! Ayuda contextual con los atajos globales (§19.4).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Atajos documentados.
const ATAJOS: &[(&str, &str)] = &[
    ("↑/↓ o k/j", "Mover la selección"),
    ("Enter", "Abrir / editar / confirmar"),
    ("Esc", "Volver / cancelar"),
    ("n", "Nuevo registro (en listados)"),
    ("e", "Editar seleccionado"),
    ("d", "Eliminar con confirmación"),
    ("/", "Buscar"),
    ("f", "Filtros"),
    ("s", "Ordenar"),
    ("r", "Refrescar"),
    ("Tab / Shift+Tab", "Campo siguiente / anterior"),
    ("Espacio", "Marcar / desmarcar en selects múltiples"),
    ("Ctrl+S", "Guardar formulario"),
    ("?", "Esta ayuda"),
    ("q", "Salir / volver"),
];

/// Dibuja la pantalla de ayuda.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = vec![Line::from(Span::styled(
        "Atajos globales",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    ))];
    for (key, action) in ATAJOS {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<16}"), Style::default().fg(colors.warning)),
            Span::styled(*action, Style::default().fg(colors.fg)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Esc: volver al menú",
        Style::default().fg(colors.muted),
    )));

    let block = Block::bordered()
        .title(" Ayuda ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
