//! Recordatorios y agenda (IMP-18).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Listado de recordatorios.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.recordatorios.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay recordatorios. Presioná n para crear uno; a abre la agenda.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Recordatorios ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .recordatorios
        .iter()
        .map(|item| {
            Row::new(vec![
                Cell::from(item.fecha.format("%d/%m/%Y").to_string()),
                Cell::from(item.hora.clone()),
                Cell::from(item.titulo.clone()),
                Cell::from(item.periodicidad.label()),
                Cell::from(item.tematica.clone()),
                Cell::from(if item.completado {
                    "Hecho"
                } else {
                    "Pendiente"
                }),
            ])
            .style(if item.completado {
                Style::default().fg(colors.muted)
            } else {
                Style::default().fg(colors.fg)
            })
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Min(24),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec![
            "Fecha",
            "Hora",
            "Título",
            "Repite",
            "Temática",
            "Estado",
        ])
        .style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Recordatorios · {} ", app.recordatorios.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.recordatorios_table);
}

/// Agenda fusionada.
pub fn draw_agenda(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.agenda.is_empty() {
        frame.render_widget(
            Paragraph::new("Agenda vacía.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Agenda ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .agenda
        .iter()
        .map(|evento| {
            Row::new(vec![
                Cell::from(evento.fecha.format("%d/%m/%Y").to_string()),
                Cell::from(evento.hora.clone()),
                Cell::from(evento.tipo),
                Cell::from(evento.titulo.clone()),
            ])
            .style(Style::default().fg(colors.fg))
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(14),
            Constraint::Min(30),
        ],
    )
    .header(
        Row::new(vec!["Fecha", "Hora", "Tipo", "Evento"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Agenda · {} ", app.agenda.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.agenda_table);
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
