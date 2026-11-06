//! Pantallas del módulo Carrera (§16.7).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::app_carrera::CARRERA_SECCIONES;
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::form;
use crate::ui::widgets::{detail, scroll};

/// Hub de carrera.
pub fn draw_menu(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let items: Vec<ListItem> = CARRERA_SECCIONES
        .iter()
        .map(|(titulo, descripcion)| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    *titulo,
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("   {descripcion}"),
                    Style::default().fg(colors.muted),
                )),
            ])
        })
        .collect();

    let (lista_area, resumen_area) = match detail::split(area) {
        Some((lista, panel)) => (lista, Some(panel)),
        None => (area, None),
    };

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(" Carrera futbolística ")
                .border_style(Style::default().fg(colors.border))
                .style(Style::default().bg(colors.bg)),
        )
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, lista_area, &mut app.carrera_hub);

    if let Some(panel) = resumen_area {
        let mut lineas = Vec::new();
        if let Some(identidad) = &app.carrera_identidad {
            lineas.push(Line::from(Span::styled(
                identidad.nombre.clone(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lineas.push(Line::from(""));
        }
        if let Some(resumen) = &app.carrera_resumen {
            for (etiqueta, valor) in [
                ("Partidos", resumen.partidos.to_string()),
                ("Goles", resumen.goles.to_string()),
                ("Asistencias", resumen.asistencias.to_string()),
                ("Títulos", resumen.titulos.to_string()),
                ("Mejor racha", resumen.mejor_racha.to_string()),
            ] {
                lineas.push(Line::from(vec![
                    Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                    Span::styled(valor, Style::default().fg(colors.fg)),
                ]));
            }
        } else {
            lineas.push(Line::from(Span::styled(
                "Completá tu identidad para empezar la carrera.",
                Style::default().fg(colors.muted),
            )));
        }
        let badges = app
            .carrera_resumen
            .as_ref()
            .map_or(0, |resumen| resumen.badges.len());
        lineas.push(Line::from(""));
        lineas.push(Line::from(vec![
            Span::styled("Badges ", Style::default().fg(colors.muted)),
            Span::styled(
                badges.to_string(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lineas.push(Line::from(Span::styled(
            "Enter: abrir sección   ↑↓: mover",
            Style::default().fg(colors.muted),
        )));
        detail::panel(frame, panel, &colors, "Tu carrera", lineas);
    }
}

/// Resumen de carrera.
pub fn draw_resumen(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(resumen) = &app.carrera_resumen else {
        frame.render_widget(
            Paragraph::new("Sin datos de carrera todavía.")
                .block(Block::bordered().title(" Resumen "))
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    };
    let mut lines = Vec::new();
    linea(
        &mut lines,
        &colors,
        "Partidos",
        &resumen.partidos.to_string(),
    );
    linea(
        &mut lines,
        &colors,
        "Goles / asistencias",
        &format!("{} / {}", resumen.goles, resumen.asistencias),
    );
    linea(
        &mut lines,
        &colors,
        "V / E / D",
        &format!(
            "{} / {} / {}",
            resumen.victorias, resumen.empates, resumen.derrotas
        ),
    );
    linea(
        &mut lines,
        &colors,
        "Rendimiento promedio",
        &format!("{:.2}", resumen.rendimiento_promedio),
    );
    linea(
        &mut lines,
        &colors,
        "Mejor año",
        resumen.mejor_anio.as_deref().unwrap_or("—"),
    );
    linea(
        &mut lines,
        &colors,
        "Mejor racha",
        &format!("{} victorias", resumen.mejor_racha),
    );
    linea(&mut lines, &colors, "Títulos", &resumen.titulos.to_string());
    linea(&mut lines, &colors, "Podios", &resumen.podios.to_string());
    linea(
        &mut lines,
        &colors,
        "Temporadas",
        &resumen.temporadas.to_string(),
    );
    let obtenidos = resumen.badges.iter().filter(|badge| badge.obtenido).count();
    linea(
        &mut lines,
        &colors,
        "Logros",
        &format!("{obtenidos} de {}", resumen.badges.len()),
    );

    render_texto(frame, area, app, " Resumen de carrera ", lines, &colors);
}

/// Listado de hitos.
pub fn draw_hitos(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let block = Block::bordered()
        .title(" Hitos de carrera ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    if app.carrera_hitos.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin hitos registrados.\n\nn: marcar un partido como hito")
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    }
    let header = Row::new(vec!["ID", "Fecha", "Partido", "Tipo", "Nota"]).style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.carrera_hitos.iter().map(|hito| {
        Row::new(vec![
            Cell::from(hito.id.to_string()),
            Cell::from(crate::domain::dates::display_date(hito.fecha.date())),
            Cell::from(format!("#{} {}", hito.partido_id, hito.rival)),
            Cell::from(hito.tipo_hito.clone()),
            Cell::from(hito.nota.clone()),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(11),
            Constraint::Min(20),
            Constraint::Min(18),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(
        Style::default()
            .bg(colors.highlight_bg)
            .fg(colors.highlight_fg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(table, area, &mut app.carrera_hitos_table);
}

/// Timeline vertical.
pub fn draw_timeline(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = Vec::new();
    if app.carrera_timeline.is_empty() {
        lines.push(Line::from("Sin eventos para mostrar."));
    }
    for evento in &app.carrera_timeline {
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "{}  ",
                    crate::domain::dates::display_date(evento.fecha.date())
                ),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!("[{}] ", evento.categoria),
                Style::default().fg(colors.accent),
            ),
            Span::styled(evento.descripcion.clone(), Style::default().fg(colors.fg)),
        ]));
    }
    render_texto(frame, area, app, " Timeline de carrera ", lines, &colors);
}

/// Hall of Fame.
pub fn draw_hall(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = Vec::new();
    if let Some(hall) = &app.carrera_hall {
        linea(
            &mut lines,
            &colors,
            "Mejor racha",
            &format!("{} victorias", hall.mejor_racha),
        );
        let racha = hall.racha_actual.as_ref().map_or_else(
            || "—".to_string(),
            |(tipo, largo)| format!("{tipo} ({largo})"),
        );
        linea(&mut lines, &colors, "Racha actual", &racha);
        encabezado(&mut lines, &colors, "Top 5 partidos");
        for partido in &hall.partidos {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "  {}  #{} {}",
                        crate::domain::dates::display_date(partido.fecha.date()),
                        partido.id,
                        partido.rival
                    ),
                    Style::default().fg(colors.fg),
                ),
                Span::styled(
                    format!(
                        "   {} G · {} A · rend. {}",
                        partido.goles, partido.asistencias, partido.rendimiento
                    ),
                    Style::default().fg(colors.muted),
                ),
            ]));
        }
    } else {
        lines.push(Line::from("Sin datos."));
    }
    render_texto(frame, area, app, " Hall of Fame personal ", lines, &colors);
}

/// Perfil dinámico.
pub fn draw_perfil(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = Vec::new();
    if let Some(perfil) = &app.carrera_perfil {
        linea(&mut lines, &colors, "Etiqueta", &perfil.etiqueta);
        linea(
            &mut lines,
            &colors,
            "Delta de rendimiento",
            &format!("{:+.2}", perfil.delta_rendimiento),
        );
        linea(
            &mut lines,
            &colors,
            "Goles últimos 5",
            &perfil.goles_recientes.to_string(),
        );
        linea(
            &mut lines,
            &colors,
            "Goles anteriores 5",
            &perfil.goles_previos.to_string(),
        );
    } else {
        lines.push(Line::from("Sin datos."));
    }
    render_texto(frame, area, app, " Perfil dinámico ", lines, &colors);
}

/// Modo retro: misma fecha en años anteriores.
pub fn draw_retro(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = Vec::new();
    if app.carrera_retro.is_empty() {
        lines.push(Line::from(
            "Sin recuerdos para la fecha de hoy. Volvé en otro momento del año.",
        ));
    }
    for partido in &app.carrera_retro {
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "{}  ",
                    crate::domain::dates::display_date(partido.fecha.date())
                ),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!("#{} {}", partido.id, partido.rival),
                Style::default().fg(colors.fg),
            ),
            Span::styled(
                format!(
                    "   {} G · {} A · rend. {}",
                    partido.goles, partido.asistencias, partido.rendimiento
                ),
                Style::default().fg(colors.muted),
            ),
        ]));
    }
    render_texto(
        frame,
        area,
        app,
        " Modo retro · Hoy en tu historia ",
        lines,
        &colors,
    );
}

/// Logros (badges).
pub fn draw_logros(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = Vec::new();
    if let Some(resumen) = &app.carrera_resumen {
        for badge in &resumen.badges {
            lines.push(Line::from(vec![
                Span::styled(
                    if badge.obtenido { "  [✓] " } else { "  [ ] " },
                    Style::default().fg(if badge.obtenido {
                        colors.success
                    } else {
                        colors.muted
                    }),
                ),
                Span::styled(
                    format!("{:<20}", badge.nombre),
                    Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(badge.descripcion.clone(), Style::default().fg(colors.muted)),
            ]));
        }
    } else {
        lines.push(Line::from("Sin datos."));
    }
    render_texto(frame, area, app, " Logros de carrera ", lines, &colors);
}

/// Formularios.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}

fn render_texto(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    titulo: &str,
    lines: Vec<Line<'static>>,
    colors: &Palette,
) {
    let total = lines.len();
    let [texto_area, _] = Layout::vertical([Constraint::Min(3), Constraint::Length(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(titulo.to_string())
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .scroll((app.carrera_scroll, 0))
            .style(Style::default().bg(colors.bg)),
        texto_area,
    );

    let barra_area = Rect {
        x: texto_area.right().saturating_sub(1),
        y: texto_area.y + 1,
        width: 1,
        height: texto_area.height.saturating_sub(2),
    };
    scroll::vertical(
        frame,
        barra_area,
        colors,
        total,
        app.carrera_scroll as usize,
        texto_area.height.saturating_sub(2) as usize,
    );
}

fn linea(lines: &mut Vec<Line<'static>>, colors: &Palette, etiqueta: &str, valor: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("{etiqueta:<24}"), Style::default().fg(colors.muted)),
        Span::styled(valor.to_string(), Style::default().fg(colors.fg)),
    ]));
}

fn encabezado(lines: &mut Vec<Line<'static>>, colors: &Palette, titulo: &str) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        titulo.to_string(),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
}
