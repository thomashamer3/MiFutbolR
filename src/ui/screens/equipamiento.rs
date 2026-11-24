//! Historial de equipamiento.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Cell, Paragraph, Row, Wrap};

use crate::app::App;
use crate::domain::equipamiento::{ESTADOS, TIPOS, etiqueta};
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Listado.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.equipamiento.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay equipamiento. Presioná n para registrar un ítem.")
                .block(colors.panel(" Historial de equipamiento "))
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .equipamiento
        .iter()
        .map(|e| {
            Row::new(vec![
                Cell::from(e.id.to_string()),
                Cell::from(etiqueta(TIPOS, e.tipo)),
                Cell::from(format!("{} {}", e.marca, e.modelo)),
                Cell::from(e.partidos_usados.to_string()),
                Cell::from(etiqueta(ESTADOS, e.estado_fisico)),
                Cell::from(format!("{}/10", e.rating)),
                Cell::from(e.precio.display_ars()),
                Cell::from(if e.activo { "Sí" } else { "No" }),
            ])
        })
        .collect();
    crate::ui::widgets::lista::render_tabla(
        frame,
        area,
        &colors,
        format!(" Historial de equipamiento · {} ", app.equipamiento.len()),
        vec![
            "ID",
            "Tipo",
            "Marca / modelo",
            "PJ",
            "Estado",
            "Rating",
            "Precio",
            "Activo",
        ],
        vec![
            Constraint::Length(4),
            Constraint::Length(11),
            Constraint::Min(20),
            Constraint::Length(5),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Length(6),
        ],
        filas,
        &mut app.equipamiento_table,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
