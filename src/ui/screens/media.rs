//! Referencias multimedia (IMP-21).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Listado.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.media.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay referencias multimedia. Presioná n para agregar una; f cambia el filtro.",
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Multimedia ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .style(Style::default().fg(colors.muted))
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .media
        .iter()
        .map(|item| {
            Row::new(vec![
                Cell::from(item.id.to_string()),
                Cell::from(item.tipo.label()),
                Cell::from(item.titulo.clone()),
                Cell::from(
                    item.partido_id
                        .map_or_else(|| "—".to_string(), |id| format!("#{id}")),
                ),
                Cell::from(item.fecha.get(..10).unwrap_or(&item.fecha).to_string()),
            ])
        })
        .collect();
    let filtro = app.media_filter.map_or("Todos", |tipo| tipo.label());
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(26),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["ID", "Tipo", "Título", "Partido", "Fecha"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Multimedia · {filtro} · {} ", app.media.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.media_table);
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
