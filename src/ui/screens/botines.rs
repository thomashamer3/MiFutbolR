//! Pantallas del módulo Botines (§10.5).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado de botines con vista previa lateral.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let title = format!(
        " Botines · {} · {} ",
        app.botin_filter.estado.label(),
        app.botin_filter.orden.label()
    );
    let (tabla_area, preview_area) = match detail::split(area) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (area, None),
    };

    if app.botines.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay botines registrados.\n\nn: crear uno nuevo      /: buscar      Esc: volver",
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(title)
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.fg)),
            tabla_area,
        );
        return;
    }

    let rows = app.botines.iter().map(|row| {
        let style = if row.activa {
            Style::default().fg(colors.fg)
        } else {
            Style::default().fg(colors.muted)
        };
        Row::new(vec![
            Cell::from(if row.predeterminado { "★" } else { " " }),
            Cell::from(row.id.to_string()),
            Cell::from(row.nombre.clone()),
            Cell::from(row.partidos.to_string()),
            Cell::from(if row.sorteada { "Sí" } else { "No" }),
            crate::ui::widgets::lista::celda_activa(row.activa, &colors, "Activo", "INACTIVO"),
        ])
        .style(style)
    });

    crate::ui::widgets::lista::render_tabla(
        frame,
        tabla_area,
        &colors,
        title,
        vec!["★", "ID", "Nombre", "P", "Sorteada", "Estado"],
        vec![
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(18),
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Length(9),
        ],
        rows.collect::<Vec<_>>(),
        &mut app.botines_table,
    );
    app.table_hits = crate::ui::widgets::hit::table_hits(
        tabla_area,
        app.botines_table.offset(),
        app.botines.len(),
    );
    app.table_hit_target = Some(crate::app::TableHitTarget::Botines);

    if let Some(panel) = preview_area {
        let seleccionado = app
            .botines_table
            .selected()
            .and_then(|indice| app.botines.get(indice));
        let mut lineas = Vec::new();
        if let Some(row) = seleccionado {
            lineas.push(Line::from(Span::styled(
                row.nombre.clone(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lineas.push(Line::from(""));
            lineas.extend(detail::kv_lineas(
                &colors,
                &[
                    ("ID", row.id.to_string()),
                    ("Partidos", row.partidos.to_string()),
                    (
                        "Sorteada",
                        if row.sorteada { "Sí" } else { "No" }.to_string(),
                    ),
                    (
                        "Estado",
                        if row.activa { "Activo" } else { "INACTIVO" }.to_string(),
                    ),
                    (
                        "Predeterminado",
                        if row.predeterminado { "★ Sí" } else { "No" }.to_string(),
                    ),
                ],
            ));
            lineas.push(Line::from(Span::styled(
                "Enter: detalle   p: predeterminado   s: sortear",
                Style::default().fg(colors.muted),
            )));
        }
        detail::panel(frame, panel, &colors, "Vista previa", lineas);
    }
}

/// Detalle de un botín.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(botin) = &app.botin_detail else {
        frame.render_widget(
            Paragraph::new(format!("Cargando botín {id}…"))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Botín "),
                )
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };
    let row = app.botines.iter().find(|row| row.id == botin.id);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{:<14}", "Nombre"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(botin.nombre.clone(), Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<14}", "Imagen"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(botin.imagen_ruta.clone(), Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<14}", "Activo"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                if botin.activa { "Sí" } else { "No" }.to_string(),
                Style::default().fg(colors.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<14}", "Sorteado"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                if botin.sorteada { "Sí" } else { "No" }.to_string(),
                Style::default().fg(colors.fg),
            ),
        ]),
    ];
    if let Some(row) = row {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<14}", "Partidos"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(row.partidos.to_string(), Style::default().fg(colors.fg)),
            Span::styled(
                format!("{:<18}", "  Predeterminado"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                if row.predeterminado { "★ Sí" } else { "No" }.to_string(),
                Style::default().fg(colors.fg),
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(" Botín · {} ", botin.nombre))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        area,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}
