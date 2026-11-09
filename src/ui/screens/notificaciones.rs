//! Bandeja de notificaciones (IMP-06).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Dibuja la bandeja.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [lista_area, pie_area] =
        Layout::vertical([Constraint::Min(4), Constraint::Length(1)]).areas(area);

    if app.notificaciones.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay notificaciones.\n\nSe generan al guardar partidos, con lesiones activas, presupuesto excedido o rachas.",
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Avisos ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .style(Style::default().fg(colors.muted))
            .wrap(Wrap { trim: true }),
            lista_area,
        );
    } else {
        let filas: Vec<Row> = app
            .notificaciones
            .iter()
            .map(|notificacion| {
                let color = match notificacion.severidad {
                    2 => colors.error,
                    1 => colors.warning,
                    _ => colors.accent,
                };
                Row::new(vec![
                    Cell::from(notificacion.severidad_label()),
                    Cell::from(notificacion.titulo.clone()),
                    Cell::from(notificacion.mensaje.clone()),
                    Cell::from(if notificacion.leida {
                        "leída"
                    } else {
                        "nueva"
                    }),
                ])
                .style(if notificacion.leida {
                    Style::default().fg(colors.muted)
                } else {
                    Style::default().fg(color)
                })
            })
            .collect();
        let tabla = Table::new(
            filas,
            [
                Constraint::Length(9),
                Constraint::Min(18),
                Constraint::Min(28),
                Constraint::Length(7),
            ],
        )
        .header(
            Row::new(vec!["Nivel", "Título", "Mensaje", "Estado"]).style(
                Style::default()
                    .fg(colors.muted)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Avisos ")
                .border_style(Style::default().fg(colors.border)),
        )
        .row_highlight_style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
        frame.render_stateful_widget(tabla, lista_area, &mut app.notificaciones_table);
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                " {} sin leer · {} en total ",
                app.no_leidas,
                app.notificaciones.len()
            ),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        ))),
        pie_area,
    );
}
