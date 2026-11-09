//! Ranking de amigos (IMP-22).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Ranking.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.amigos_ranking.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin amigos cargados. Presioná n para agregar uno; el usuario local aparece siempre.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Ranking de amigos ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .amigos_ranking
        .iter()
        .enumerate()
        .map(|(indice, fila)| {
            Row::new(vec![
                Cell::from((indice + 1).to_string()),
                Cell::from(if fila.local {
                    format!("{} (vos)", fila.nombre)
                } else {
                    fila.nombre.clone()
                }),
                Cell::from(fila.goles.to_string()),
                Cell::from(fila.asistencias.to_string()),
                Cell::from(fila.partidos.to_string()),
                Cell::from(fila.mejor_rendimiento.to_string()),
            ])
            .style(if fila.local {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            })
        })
        .collect();
    let [tabla_area, pie_area] =
        Layout::vertical([Constraint::Min(4), Constraint::Length(1)]).areas(area);
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(22),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["#", "Nombre", "Goles", "Asist.", "Partidos", "Mejor"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Ranking · {} ", app.amigos_ranking.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, tabla_area, &mut app.amigos_table);
    frame.render_widget(
        Paragraph::new(Line::from(
            " n: nuevo · e: editar · d: borrar · r: refrescar · Esc: volver ",
        ))
        .style(Style::default().fg(colors.muted)),
        pie_area,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
