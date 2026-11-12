//! Pantallas del módulo Lesiones (§13.6).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado con vista previa y estadísticas al pie.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let title = format!(
        " Lesiones · {} · {} ",
        app.lesion_filter
            .estado
            .map_or("Todos", |estado| estado.label()),
        app.lesion_filter.orden.label()
    );

    let [cuerpo, summary_area] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).areas(area);
    let (tabla_area, preview_area) = match detail::split(cuerpo) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (cuerpo, None),
    };

    if app.lesiones.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay lesiones registradas.\n\nn: registrar lesión      /: buscar      Esc: volver",
            )
            .block(
                Block::bordered().border_type(BorderType::Rounded)
                    .title(title)
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.fg)),
            tabla_area,
        );
    } else {
        let rows = app.lesiones.iter().map(|row| {
            let style = if row.estado == crate::domain::models::EstadoLesion::Recuperado {
                Style::default().fg(colors.muted)
            } else {
                Style::default().fg(colors.fg)
            };
            Row::new(vec![
                Cell::from(row.id.to_string()),
                Cell::from(crate::domain::dates::display_datetime(row.fecha)),
                Cell::from(row.jugador.clone()),
                Cell::from(row.tipo.clone()),
                Cell::from(row.estado.label()).style(
                    Style::default()
                        .fg(match row.estado {
                            crate::domain::models::EstadoLesion::Recuperado => colors.muted,
                            crate::domain::models::EstadoLesion::Activa
                            | crate::domain::models::EstadoLesion::Recaida => colors.error,
                            crate::domain::models::EstadoLesion::EnTratamiento => colors.warning,
                            crate::domain::models::EstadoLesion::Mejorando => colors.accent,
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(row.camiseta.clone()),
                Cell::from(row.partido.clone()),
            ])
            .style(style)
        });

        crate::ui::widgets::lista::render_tabla(
            frame,
            tabla_area,
            &colors,
            title,
            vec![
                "ID", "Fecha", "Jugador", "Tipo", "Estado", "Camiseta", "Partido",
            ],
            vec![
                Constraint::Length(4),
                Constraint::Length(16),
                Constraint::Min(12),
                Constraint::Min(14),
                Constraint::Length(15),
                Constraint::Min(10),
                Constraint::Length(18),
            ],
            rows.collect::<Vec<_>>(),
            &mut app.lesiones_table,
        );
        app.table_hits = crate::ui::widgets::hit::table_hits(
            tabla_area,
            app.lesiones_table.offset(),
            app.lesiones.len(),
        );
        app.table_hit_target = Some(crate::app::TableHitTarget::Lesiones);

        if let Some(panel) = preview_area {
            let seleccionada = app
                .lesiones_table
                .selected()
                .and_then(|indice| app.lesiones.get(indice));
            let mut lineas = Vec::new();
            if let Some(row) = seleccionada {
                lineas.push(Line::from(Span::styled(
                    format!("{} · {}", row.jugador, row.tipo),
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )));
                lineas.push(Line::from(""));
                for (etiqueta, valor) in [
                    ("Fecha", crate::domain::dates::display_datetime(row.fecha)),
                    ("Estado", row.estado.label().to_string()),
                    ("Camiseta", row.camiseta.clone()),
                    ("Partido", row.partido.clone()),
                ] {
                    lineas.push(Line::from(vec![
                        Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                        Span::styled(valor, Style::default().fg(colors.fg)),
                    ]));
                }
                lineas.push(Line::from(""));
                lineas.push(Line::from(Span::styled(
                    "Enter: detalle   e: editar   d: eliminar",
                    Style::default().fg(colors.muted),
                )));
            }
            detail::panel(frame, panel, &colors, "Vista previa", lineas);
        }
    }

    let stats = &app.lesion_stats;
    let promedio = stats
        .promedio_dias
        .map_or_else(|| "—".to_string(), |dias| format!("{dias:.1} días"));
    let summary = Line::from(vec![
        Span::styled(
            format!(
                " Total: {}  Activas: {}  Recuperadas: {}  Promedio entre lesiones: {}",
                stats.total, stats.activas, stats.recuperadas, promedio
            ),
            Style::default().fg(colors.fg),
        ),
        Span::styled(
            format!("    Tipos: {}", stats.por_tipo.len()),
            Style::default().fg(colors.muted),
        ),
    ]);
    frame.render_widget(Paragraph::new(vec![summary]), summary_area);
}

/// Detalle de una lesión.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(lesion) = &app.lesion_detail else {
        frame.render_widget(
            Paragraph::new(format!("Cargando lesión {id}…"))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Lesión "),
                )
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };
    let fila = app.lesiones.iter().find(|row| row.id == lesion.id);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut push = |label: &str, value: &str| {
        if value.trim().is_empty() {
            return;
        }
        lines.push(Line::from(vec![
            Span::styled(format!("{label:<16}"), Style::default().fg(colors.muted)),
            Span::styled(value.to_string(), Style::default().fg(colors.fg)),
        ]));
    };
    push("Jugador", &lesion.jugador);
    push("Tipo", &lesion.tipo);
    push("Descripción", &lesion.descripcion);
    push(
        "Fecha",
        &crate::domain::dates::display_datetime(lesion.fecha),
    );
    push("Estado", lesion.estado.label());
    if let Some(fila) = fila {
        push("Camiseta", &fila.camiseta);
        push("Partido", &fila.partido);
    }
    if let Some(partido_id) = lesion.partido_id {
        push("Partido ID", &partido_id.to_string());
    }
    if lesion.fecha > chrono::Local::now().naive_local() {
        lines.push(Line::from(Span::styled(
            "[!] La fecha es futura",
            Style::default().fg(colors.warning),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(
                        " Lesión · {} ",
                        crate::domain::dates::display_datetime(lesion.fecha)
                    ))
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
