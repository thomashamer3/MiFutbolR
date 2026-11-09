//! Últimos partidos por resultado y aportes (IMP-16).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Dibuja los cinco "últimos".
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.ultimos.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay partidos registrados.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Últimos ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }

    let [lista, pie] = Layout::vertical([Constraint::Min(6), Constraint::Length(1)]).areas(area);
    let filas: Vec<Row> = app
        .ultimos
        .iter()
        .map(|item| {
            let (partido, detalle) = item.partido.as_ref().map_or_else(
                || ("—".to_string(), "Sin registros".to_string()),
                |partido| {
                    (
                        format!(
                            "vs {}",
                            if partido.rival.trim().is_empty() {
                                "rival"
                            } else {
                                &partido.rival
                            }
                        ),
                        format!(
                            "{} · {} G · {} A",
                            partido.fecha_hora.format("%d/%m/%Y"),
                            partido.goles,
                            partido.asistencias
                        ),
                    )
                },
            );
            Row::new(vec![
                Cell::from(item.etiqueta),
                Cell::from(partido),
                Cell::from(detalle),
                Cell::from(item.hace.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(20),
            Constraint::Min(18),
            Constraint::Min(22),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(vec!["Último", "Partido", "Detalle", "Cuándo"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Últimos ")
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, lista, &mut app.ultimos_table);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Enter: abrir partido · r: recalcular · Esc: volver ",
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )))
        .wrap(Wrap { trim: true }),
        pie,
    );
}
