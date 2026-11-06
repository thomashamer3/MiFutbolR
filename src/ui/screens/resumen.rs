//! Vista previa del Resumen Compartible (§17.5).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::scroll;

/// Dibuja la vista previa desplazable.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let titulo = format!(" Resumen compartible · {} ", app.resumen_formato.label());
    let block = Block::bordered()
        .title(titulo)
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));

    if app.resumen_datos.is_none() {
        frame.render_widget(
            Paragraph::new(
                "Sin datos suficientes; registra partidos para ver estadísticas y generar el resumen.",
            )
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }

    let [texto_area, pie_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(app.resumen_texto.clone())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.resumen_scroll, 0))
            .style(Style::default().bg(colors.bg).fg(colors.fg)),
        texto_area,
    );
    let barra_area = Rect {
        x: texto_area.right().saturating_sub(1),
        y: texto_area.y + 1,
        width: 1,
        height: texto_area.height.saturating_sub(2),
    };
    scroll::vertical(
        frame,
        barra_area,
        &colors,
        app.resumen_texto.lines().count(),
        app.resumen_scroll as usize,
        texto_area.height.saturating_sub(2) as usize,
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                " {} · {} líneas ",
                app.resumen_formato.label(),
                app.resumen_texto.lines().count()
            ),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        ))),
        pie_area,
    );
}
