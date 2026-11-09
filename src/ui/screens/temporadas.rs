//! Temporadas: listado, detalle y formulario (IMP-12).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Listado.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.temporadas.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay temporadas.\n\nPresioná n para crear una; se generan sus fases y se asocian los torneos existentes.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Temporadas ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .temporadas
        .iter()
        .map(|temporada| {
            Row::new(vec![
                Cell::from(temporada.id.to_string()),
                Cell::from(temporada.nombre.clone()),
                Cell::from(temporada.anio.to_string()),
                Cell::from(temporada.fecha_inicio.format("%d/%m/%Y").to_string()),
                Cell::from(temporada.fecha_fin.format("%d/%m/%Y").to_string()),
                Cell::from(temporada.estado.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(22),
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(
        Row::new(vec!["ID", "Nombre", "Año", "Inicio", "Fin", "Estado"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Temporadas · {} ", app.temporadas.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.temporadas_table);
}

/// Detalle desplazable.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(ficha) = &app.temporada_detail else {
        frame.render_widget(
            Paragraph::new("Cargando temporada…").style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    };
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {} ", ficha.temporada.nombre),
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            " {} · {} → {} · {}",
            ficha.temporada.anio,
            ficha.temporada.fecha_inicio.format("%d/%m/%Y"),
            ficha.temporada.fecha_fin.format("%d/%m/%Y"),
            ficha.temporada.estado
        ),
        Style::default().fg(colors.muted),
    )));
    if !ficha.temporada.descripcion.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {}", ficha.temporada.descripcion),
            Style::default().fg(colors.fg),
        )));
    }
    lines.push(Line::from(""));
    let r = &ficha.resumen;
    lines.push(Line::from(Span::styled(
        "Resumen",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "  {} PJ · {} G · {} A · {}V {}E {}D · winrate {:.1}% · rend. {:.2} · {} lesiones",
            r.partidos,
            r.goles,
            r.asistencias,
            r.victorias,
            r.empates,
            r.derrotas,
            r.winrate,
            r.rendimiento,
            r.lesiones
        ),
        Style::default().fg(colors.fg),
    )));
    lines.push(Line::from(Span::styled(
        format!("  Fatiga: {:.0}/100", ficha.fatiga),
        Style::default().fg(if ficha.fatiga >= 70.0 {
            colors.warning
        } else {
            colors.fg
        }),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Fases",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    if ficha.fases.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Sin fases",
            Style::default().fg(colors.muted),
        )));
    }
    for fase in &ficha.fases {
        lines.push(Line::from(Span::styled(
            format!(
                "  {} ({}) · {} → {}",
                fase.nombre,
                fase.tipo,
                if fase.fecha_inicio.is_empty() {
                    "—"
                } else {
                    &fase.fecha_inicio
                },
                if fase.fecha_fin.is_empty() {
                    "—"
                } else {
                    &fase.fecha_fin
                }
            ),
            Style::default().fg(colors.fg),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Torneos",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    if ficha.torneos.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Sin torneos asociados",
            Style::default().fg(colors.muted),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {}", ficha.torneos.join(" · ")),
            Style::default().fg(colors.fg),
        )));
    }
    if let Some(comparacion) = &app.temporada_comparacion {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Comparación · {} vs {}", comparacion.a, comparacion.b),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "  {:<16} {:>12} {:>12}",
                "Métrica", comparacion.a, comparacion.b
            ),
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        )));
        for (metrica, a, b) in &comparacion.filas {
            lines.push(Line::from(Span::styled(
                format!("  {metrica:<16} {a:>12} {b:>12}"),
                Style::default().fg(colors.fg),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "e: editar · d: borrar · c: comparar · ↑↓: desplazar · Esc: volver",
        Style::default().fg(colors.muted),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Temporada ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.temporada_scroll, 0))
            .style(Style::default().bg(colors.bg).fg(colors.fg)),
        area,
    );
}

/// Formulario genérico.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
