//! Tiendas (IMP-20).

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
    if app.tiendas.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay tiendas. Presioná n para crear una.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Tiendas ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .tiendas
        .iter()
        .map(|tienda| {
            let contacto = [
                tienda.url.as_str(),
                tienda.telefono.as_str(),
                tienda.instagram.as_str(),
            ]
            .into_iter()
            .find(|valor| !valor.is_empty())
            .unwrap_or("—")
            .to_string();
            Row::new(vec![
                Cell::from(tienda.id.to_string()),
                Cell::from(tienda.nombre.clone()),
                Cell::from(tienda.tipo.clone()),
                Cell::from(contacto),
                Cell::from(if tienda.favorito { "★" } else { "" }),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(22),
            Constraint::Length(14),
            Constraint::Min(24),
            Constraint::Length(4),
        ],
    )
    .header(
        Row::new(vec!["ID", "Nombre", "Tipo", "Contacto", "Fav"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Tiendas · {} ", app.tiendas.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.tiendas_table);
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
