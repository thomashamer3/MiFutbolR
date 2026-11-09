//! Pantallas del módulo Equipos (§11.5).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado de equipos con vista previa lateral.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let title = format!(
        " Equipos · {} · {} ",
        app.equipo_filter.estado.label(),
        app.equipo_filter.orden.label()
    );
    let (tabla_area, preview_area) = match detail::split(area) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (area, None),
    };

    if app.equipos.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay equipos registrados.\n\nn: crear uno nuevo      /: buscar      Esc: volver",
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

    let header = Row::new(vec![
        "ID",
        "Nombre",
        "Tipo",
        "Modalidad",
        "Jugadores",
        "Estado",
    ])
    .style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.equipos.iter().map(|row| {
        let style = if row.activa {
            Style::default().fg(colors.fg)
        } else {
            Style::default().fg(colors.muted)
        };
        Row::new(vec![
            Cell::from(row.id.to_string()),
            Cell::from(row.nombre.clone()),
            Cell::from(row.tipo.label()),
            Cell::from(row.tipo_futbol.label()),
            Cell::from(format!("{}/{}", row.jugadores_count, row.num_jugadores)),
            Cell::from(if row.activa { "Activo" } else { "INACTIVO" }).style(
                Style::default()
                    .fg(if row.activa {
                        colors.success
                    } else {
                        colors.muted
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Min(18),
            Constraint::Length(11),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(9),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(colors.border))
            .style(Style::default().bg(colors.bg)),
    )
    .column_spacing(1)
    .highlight_spacing(HighlightSpacing::Always)
    .row_highlight_style(
        Style::default()
            .bg(colors.highlight_bg)
            .fg(colors.highlight_fg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, tabla_area, &mut app.equipos_table);
    app.table_hits = crate::ui::widgets::hit::table_hits(
        tabla_area,
        app.equipos_table.offset(),
        app.equipos.len(),
    );
    app.table_hit_target = Some(crate::app::TableHitTarget::Equipos);

    if let Some(panel) = preview_area {
        let seleccionado = app
            .equipos_table
            .selected()
            .and_then(|indice| app.equipos.get(indice));
        let mut lineas = Vec::new();
        if let Some(row) = seleccionado {
            lineas.push(Line::from(Span::styled(
                row.nombre.clone(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lineas.push(Line::from(""));
            for (etiqueta, valor) in [
                ("ID", row.id.to_string()),
                ("Tipo", row.tipo.label().to_string()),
                ("Modalidad", row.tipo_futbol.label().to_string()),
                (
                    "Jugadores",
                    format!("{}/{}", row.jugadores_count, row.num_jugadores),
                ),
                (
                    "Estado",
                    if row.activa { "Activo" } else { "INACTIVO" }.to_string(),
                ),
            ] {
                lineas.push(Line::from(vec![
                    Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                    Span::styled(valor, Style::default().fg(colors.fg)),
                ]));
            }
            lineas.push(Line::from(""));
            lineas.push(Line::from(Span::styled(
                "Enter: plantel   e: editar   d: eliminar",
                Style::default().fg(colors.muted),
            )));
        }
        let ruta_imagen = seleccionado.and_then(|row| {
            crate::ui::images::ImageCache::resolver(
                &row.imagen_ruta,
                &app.paths.images_dir,
                &app.paths.data_dir,
            )
        });
        let bloque = colors.panel(" Vista previa ");
        let interior = bloque.inner(panel);
        frame.render_widget(bloque, panel);
        let con_imagen = ruta_imagen.is_some() && interior.height >= 16;
        let [texto_area, imagen_area] = Layout::vertical([
            Constraint::Min(6),
            Constraint::Length(if con_imagen { 8 } else { 0 }),
        ])
        .areas(interior);
        frame.render_widget(Paragraph::new(lineas).wrap(Wrap { trim: true }), texto_area);
        if let (Some(ruta), true) = (ruta_imagen, con_imagen) {
            crate::ui::images::dibujar_miniatura(
                frame,
                imagen_area,
                &mut app.images,
                &ruta,
                &colors,
            );
        }
    }
}

/// Detalle con la formación del equipo.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &mut App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(equipo) = &app.equipo_detail else {
        frame.render_widget(
            Paragraph::new(format!("Cargando equipo {id}…"))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Equipo "),
                )
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };

    let ruta_imagen = crate::ui::images::ImageCache::resolver(
        &equipo.imagen_ruta,
        &app.paths.images_dir,
        &app.paths.data_dir,
    );
    let alto_info = if ruta_imagen.is_some() && area.width >= 100 {
        12
    } else {
        9
    };
    let [info_area, squad_area] =
        Layout::vertical([Constraint::Length(alto_info), Constraint::Min(5)]).areas(area);
    let (info_area, imagen_area) = if ruta_imagen.is_some() && area.width >= 100 {
        let [texto, imagen] = Layout::horizontal([Constraint::Min(44), Constraint::Length(34)])
            .spacing(1)
            .areas(info_area);
        (texto, Some(imagen))
    } else {
        (info_area, None)
    };

    let info = vec![
        Line::from(vec![
            Span::styled(
                format!("{:<16}", "Nombre"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(equipo.nombre.clone(), Style::default().fg(colors.fg)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<16}", "Tipo"), Style::default().fg(colors.muted)),
            Span::styled(
                equipo.tipo.label().to_string(),
                Style::default().fg(colors.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<16}", "Modalidad"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                equipo.tipo_futbol.label().to_string(),
                Style::default().fg(colors.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<16}", "Jugadores"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!("{}/{}", equipo.jugadores.len(), equipo.num_jugadores),
                Style::default().fg(colors.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<16}", "Imagen"),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                std::path::Path::new(&equipo.imagen_ruta)
                    .file_name()
                    .map_or_else(
                        || equipo.imagen_ruta.clone(),
                        |nombre| nombre.to_string_lossy().into_owned(),
                    ),
                Style::default().fg(colors.fg),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(info)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(" Equipo · {} ", equipo.nombre))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        info_area,
    );

    if let (Some(area_imagen), Some(ruta)) = (imagen_area, ruta_imagen) {
        crate::ui::images::dibujar(frame, area_imagen, &mut app.images, &ruta, &colors);
    }

    let header = Row::new(vec!["#", "Nombre", "Posición", "Capitán"]).style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.equipo_detail.as_ref().map(|equipo| {
        equipo
            .jugadores
            .iter()
            .map(|jugador| {
                Row::new(vec![
                    Cell::from(jugador.numero.to_string()),
                    Cell::from(jugador.nombre.clone()),
                    Cell::from(jugador.posicion.label()),
                    Cell::from(if jugador.es_capitan { "★" } else { "" }),
                ])
            })
            .collect::<Vec<_>>()
    });
    let rows = rows.unwrap_or_default();

    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin jugadores cargados.  a: agregar jugador")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Formación ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted)),
            squad_area,
        );
        return;
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(16),
            Constraint::Length(16),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Formación ")
            .border_style(Style::default().fg(colors.border))
            .style(Style::default().bg(colors.bg)),
    )
    .row_highlight_style(
        Style::default()
            .bg(colors.highlight_bg)
            .fg(colors.highlight_fg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, squad_area, &mut app.jugadores_table);
}

/// Formulario (equipo o jugador).
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}
