//! Listado de cuentas locales.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::{form, lista};

/// Dibuja el listado de cuentas.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let actual = app.usuario_actual.as_ref().map(|usuario| usuario.id);
    if app.usuarios.is_empty() {
        lista::render_vacio(
            frame,
            area,
            &colors,
            " Usuario ",
            "No hay cuentas. Presioná n para crear una o l para iniciar sesión.",
        );
        return;
    }
    let filas = app
        .usuarios
        .iter()
        .map(|usuario| {
            let es_actual = actual == Some(usuario.id);
            Row::new(vec![
                Cell::from(usuario.id.to_string()),
                Cell::from(usuario.nombre.clone()),
                Cell::from(if usuario.con_password { "Sí" } else { "No" }),
                Cell::from(usuario.creado_en.clone()),
                Cell::from(if es_actual { "★" } else { "" }),
            ])
            .style(if es_actual {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            })
        })
        .collect::<Vec<_>>();
    lista::render_tabla(
        frame,
        area,
        &colors,
        format!(" Usuario · {} ", app.usuarios.len()),
        vec!["ID", "Nombre", "Contraseña", "Creado", "Actual"],
        vec![
            ratatui::layout::Constraint::Length(4),
            ratatui::layout::Constraint::Min(20),
            ratatui::layout::Constraint::Length(11),
            ratatui::layout::Constraint::Length(20),
            ratatui::layout::Constraint::Length(7),
        ],
        filas,
        &mut app.usuarios_table,
    );
}

/// Formulario compartido de cuenta y login.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
