//! Récords y rankings (IMP-05).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Dibuja la lista de récords y el detalle del seleccionado.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [lista, detalle] =
        Layout::vertical([Constraint::Min(8), Constraint::Length(5)]).areas(area);

    if app.records.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay partidos para calcular récords.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Récords ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted)),
            lista,
        );
        return;
    }

    let filas: Vec<Row> = app
        .records
        .iter()
        .map(|record| {
            Row::new(vec![
                Cell::from(record.titulo.clone()),
                Cell::from(record.detalle.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(filas, [Constraint::Length(36), Constraint::Min(30)])
        .header(
            Row::new(vec!["Reporte", "Detalle"]).style(
                Style::default()
                    .fg(colors.muted)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(format!(" Récords · {} ", app.records.len()))
                .border_style(Style::default().fg(colors.border)),
        )
        .row_highlight_style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, lista, &mut app.records_table);

    let seleccionado = app
        .records_table
        .selected()
        .and_then(|indice| app.records.get(indice));
    let texto = seleccionado.map_or_else(
        || "Seleccioná un reporte para ver el detalle.".to_string(),
        |record| format!("{}\n{}", record.titulo, record.detalle),
    );
    frame.render_widget(
        Paragraph::new(texto)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Detalle ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.fg)),
        detalle,
    );
}
