//! Progresión y planes (IMP-23).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Listado de mediciones.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.progresion.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin mediciones.\n\nPresioná n para cargar una semana; p administra los planes de entrenamiento.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Progresión ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .progresion
        .iter()
        .map(|fila| {
            Row::new(vec![
                Cell::from(fila.jugador.clone()),
                Cell::from(if fila.plan.is_empty() {
                    "—".to_string()
                } else {
                    fila.plan.clone()
                }),
                Cell::from(fila.semana.to_string()),
                Cell::from(fila.ataque.to_string()),
                Cell::from(fila.defensa.to_string()),
                Cell::from(fila.resistencia.to_string()),
                Cell::from(fila.velocidad.to_string()),
                Cell::from(fila.tecnica.to_string()),
                Cell::from(format!("{:.1}", fila.promedio())),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Min(20),
            Constraint::Min(16),
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(6),
        ],
    )
    .header(
        Row::new(vec![
            "Jugador", "Plan", "Sem", "Ata", "Def", "Res", "Vel", "Tec", "Prom",
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
            .title(format!(" Progresión · {} ", app.progresion.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.progresion_table);
}

/// Listado de planes.
pub fn draw_planes(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.planes.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin planes. Presioná n para crear uno.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Planes ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .planes
        .iter()
        .map(|plan| {
            Row::new(vec![
                Cell::from(plan.id.to_string()),
                Cell::from(plan.nombre.clone()),
                Cell::from(plan.duracion_semanas.to_string()),
                Cell::from(plan.sesiones_por_semana.to_string()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(28),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["ID", "Nombre", "Semanas", "Sesiones"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Planes · {} ", app.planes.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.planes_table);
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
