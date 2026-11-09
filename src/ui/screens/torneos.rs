//! Torneos: listado, detalle, fixture, tabla y formularios (IMP-11).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::{App, Screen};
use crate::domain::torneos::PartidoTorneoRow;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Dibuja el listado.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    if app.torneos.is_empty() {
        frame.render_widget(
            Paragraph::new("Todavía no hay torneos.\n\nPresioná n para crear uno; se asocian automáticamente los equipos activos.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Torneos ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .torneos
        .iter()
        .map(|torneo| {
            Row::new(vec![
                Cell::from(torneo.id.to_string()),
                Cell::from(torneo.nombre.clone()),
                Cell::from(torneo.tipo.label()),
                Cell::from(torneo.cantidad_equipos.to_string()),
                Cell::from(torneo.fase_actual.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(24),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Min(14),
        ],
    )
    .header(
        Row::new(vec!["ID", "Nombre", "Tipo", "Equipos", "Fase"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" Torneos · {} ", app.torneos.len()))
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.torneos_table);
}

/// Dibuja el detalle.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let Some(ficha) = app.torneo_detail.clone() else {
        frame.render_widget(
            Paragraph::new("Cargando torneo…").style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    };

    let [cabecera, cuerpo] =
        Layout::vertical([Constraint::Length(5), Constraint::Min(6)]).areas(area);
    let equipos: Vec<String> = ficha
        .equipos
        .iter()
        .map(|equipo| equipo.nombre.clone())
        .collect();
    let lineas = vec![
        Line::from(Span::styled(
            format!(" {} ", ficha.torneo.nombre),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("Tipo: ", Style::default().fg(colors.muted)),
            Span::styled(ficha.torneo.tipo.label(), Style::default().fg(colors.fg)),
            Span::styled("   Equipos: ", Style::default().fg(colors.muted)),
            Span::styled(
                ficha.equipos.len().to_string(),
                Style::default().fg(colors.fg),
            ),
            Span::styled("   Jugados: ", Style::default().fg(colors.muted)),
            Span::styled(
                format!("{}/{}", ficha.jugados(), ficha.partidos.len()),
                Style::default().fg(colors.fg),
            ),
        ]),
        Line::from(Span::styled(
            if equipos.is_empty() {
                "Sin equipos: a asocia los equipos activos.".to_string()
            } else {
                format!("Equipos: {}", equipos.join(", "))
            },
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lineas).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border)),
        ),
        cabecera,
    );

    if area.width >= 120 {
        let [izquierda, derecha] =
            Layout::horizontal([Constraint::Percentage(62), Constraint::Min(40)])
                .spacing(1)
                .areas(cuerpo);
        draw_fixture(frame, izquierda, app, &ficha, &colors);
        draw_tabla(frame, derecha, &ficha, &colors);
    } else {
        draw_fixture(frame, cuerpo, app, &ficha, &colors);
    }
}

fn draw_fixture(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    ficha: &crate::domain::torneos::TorneoFicha,
    colors: &crate::ui::theme::Palette,
) {
    if ficha.partidos.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "Sin fixture.\n\nPresioná g para generarlo con los equipos asociados.\nSi faltan equipos, a los asocia.",
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Fixture ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .style(Style::default().fg(colors.muted))
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let filas: Vec<Row> = ficha
        .partidos
        .iter()
        .enumerate()
        .map(|(indice, partido)| {
            let marcador = if partido.estado == "Jugado" {
                format!("{} - {}", partido.goles_local, partido.goles_visitante)
            } else {
                "-".to_string()
            };
            Row::new(vec![
                Cell::from((indice + 1).to_string()),
                Cell::from(partido.local.clone()),
                Cell::from(partido.visitante.clone()),
                Cell::from(marcador),
                Cell::from(partido.estado.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(4),
            Constraint::Min(14),
            Constraint::Min(14),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["#", "Local", "Visitante", "Marcador", "Estado"]).style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Fixture ")
            .border_style(Style::default().fg(colors.border)),
    )
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, area, &mut app.torneo_partidos_table);
}

fn draw_tabla(
    frame: &mut Frame,
    area: Rect,
    ficha: &crate::domain::torneos::TorneoFicha,
    colors: &crate::ui::theme::Palette,
) {
    let filas: Vec<Row> = ficha
        .tabla
        .iter()
        .enumerate()
        .map(|(indice, fila)| {
            Row::new(vec![
                Cell::from((indice + 1).to_string()),
                Cell::from(fila.equipo.clone()),
                Cell::from(fila.pj.to_string()),
                Cell::from(fila.pg.to_string()),
                Cell::from(fila.pe.to_string()),
                Cell::from(fila.pp.to_string()),
                Cell::from(fila.gf.to_string()),
                Cell::from(fila.gc.to_string()),
                Cell::from(fila.dif.to_string()),
                Cell::from(fila.puntos.to_string()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(4),
        ],
    )
    .header(
        Row::new(vec![
            "#", "Equipo", "PJ", "PG", "PE", "PP", "GF", "GC", "DG", "Pts",
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
            .title(" Tabla ")
            .border_style(Style::default().fg(colors.border)),
    );
    frame.render_widget(tabla, area);
}

/// Formulario genérico.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}

/// Compatibilidad con el dispatch de pantallas.
#[must_use]
pub fn es_formulario(screen: Screen) -> bool {
    matches!(
        screen,
        Screen::TorneoForm(_) | Screen::TorneoResultadoForm(_)
    )
}

/// Filas del fixture para tests de lógica.
#[must_use]
pub fn marcador(partido: &PartidoTorneoRow) -> String {
    format!("{} - {}", partido.goles_local, partido.goles_visitante)
}
