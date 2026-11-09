//! Ayuda contextual con los atajos globales (§19.4).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Atajos documentados.
const ATAJOS: &[(&str, &str)] = &[
    ("↑/↓ o k/j", "Mover la selección"),
    ("Enter", "Abrir / editar / confirmar"),
    ("Esc", "Volver / cancelar"),
    ("Ctrl+S", "Guardar formulario"),
    ("Tab / Shift+Tab", "Campo siguiente / anterior"),
    ("n / e / d", "Nuevo / editar / eliminar"),
    ("/", "Buscar en el listado"),
    ("f / o", "Filtros / ordenar"),
    ("r", "Refrescar"),
    ("x", "Exportar listado (CSV/JSON/HTML/MD)"),
    ("Ctrl+K / Ctrl+F", "Paleta / búsqueda global"),
    ("g / c / !", "Inicio / calendario / avisos"),
    ("l / R / T / U", "Logros / récords / torneos / últimos"),
    ("C", "Comparador A/B"),
    ("D / A / M", "Tiendas / amigos / multimedia"),
    ("Y", "Recordatorios"),
    ("P", "Progresión"),
    ("t / Ctrl+Z", "Tutorial / deshacer"),
    ("Espacio", "Marcar / desmarcar"),
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
        .border_type(BorderType::Rounded)
        .title(" Ayuda ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
