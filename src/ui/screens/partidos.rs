//! Pantallas del módulo Partidos (§12.9).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Tabs, Wrap};

use crate::app::App;
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::{detail, form};

/// Listado paginado con resumen y vista previa lateral.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let page_size = app.partido_page_size.max(1);
    let max_page = ((app.partido_total + page_size - 1) / page_size).max(1);
    let title = format!(
        " Partidos · Página {}/{} · {} registros ",
        app.partido_page, max_page, app.partido_total
    );

    let [cuerpo, summary_area] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).areas(area);
    let (tabla_area, preview_area) = match detail::split(cuerpo) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (cuerpo, None),
    };

    if app.partidos.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No hay partidos para el filtro actual.\n\nn: registrar partido      /: buscar      Esc: volver",
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
        let rows = app.partidos.iter().map(|row| {
            let rival = if row.rival.trim().is_empty() {
                "—".to_string()
            } else {
                row.rival.clone()
            };
            Row::new(vec![
                Cell::from(row.id.to_string()),
                Cell::from(crate::domain::dates::display_datetime(row.fecha_hora)),
                Cell::from(rival),
                Cell::from(row.cancha.clone()),
                Cell::from(row.camiseta.clone()),
                Cell::from(row.goles.to_string()),
                Cell::from(row.asistencias.to_string()),
                Cell::from(row.resultado.label()).style(
                    Style::default()
                        .fg(match row.resultado {
                            crate::domain::models::Resultado::Victoria => colors.success,
                            crate::domain::models::Resultado::Empate => colors.warning,
                            crate::domain::models::Resultado::Derrota => colors.error,
                            crate::domain::models::Resultado::NoAplica => colors.muted,
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(row.precio.display_ars()),
                Cell::from(row.tipo_partido.label()),
                Cell::from(if row.favorito { "★" } else { "" })
                    .style(Style::default().fg(colors.warning)),
            ])
        });

        crate::ui::widgets::lista::render_tabla(
            frame,
            tabla_area,
            &colors,
            title,
            vec![
                "ID", "Fecha", "Rival", "Cancha", "Camiseta", "G", "A", "Res.", "$", "Tipo", "★",
            ],
            vec![
                Constraint::Length(4),
                Constraint::Length(16),
                Constraint::Min(12),
                Constraint::Min(12),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Length(11),
                Constraint::Length(12),
                Constraint::Length(2),
            ],
            rows.collect::<Vec<_>>(),
            &mut app.partidos_table,
        );
        let offset = app.partidos_table.offset();
        let visibles = tabla_area.height.saturating_sub(3) as usize;
        app.table_hits = (0..visibles)
            .filter_map(|fila| {
                let indice = offset + fila;
                (indice < app.partidos.len()).then_some((
                    Rect {
                        x: tabla_area.x + 1,
                        y: tabla_area.y + 2 + fila as u16,
                        width: tabla_area.width.saturating_sub(2),
                        height: 1,
                    },
                    indice,
                ))
            })
            .collect();
        app.table_hit_target = Some(crate::app::TableHitTarget::Partidos);

        if let Some(panel) = preview_area {
            let seleccionado = app
                .partidos_table
                .selected()
                .and_then(|indice| app.partidos.get(indice));
            if let Some(row) = seleccionado {
                let mut lineas = Vec::new();
                lineas.push(Line::from(Span::styled(
                    format!(
                        "{}  ·  #{}",
                        crate::domain::dates::display_datetime(row.fecha_hora),
                        row.id
                    ),
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )));
                lineas.push(Line::from(""));
                let mut push = |etiqueta: &str, valor: String| {
                    lineas.push(Line::from(vec![
                        Span::styled(format!("{etiqueta:<14}"), Style::default().fg(colors.muted)),
                        Span::styled(valor, Style::default().fg(colors.fg)),
                    ]));
                };
                push(
                    "Rival",
                    if row.rival.trim().is_empty() {
                        "—".to_string()
                    } else {
                        row.rival.clone()
                    },
                );
                push("Cancha", row.cancha.clone());
                push("Camiseta", row.camiseta.clone());
                push(
                    "Marcador",
                    format!("{} G · {} A", row.goles, row.asistencias),
                );
                push("Resultado", row.resultado.label().to_string());
                push("Tipo", row.tipo_partido.label().to_string());
                push("Precio", row.precio.display_ars());
                if row.favorito {
                    push("Favorito", "★ Sí".to_string());
                }
                lineas.push(Line::from(""));
                lineas.push(Line::from(Span::styled(
                    "Enter: ficha   e: editar   d: eliminar",
                    Style::default().fg(colors.muted),
                )));
                detail::panel(frame, panel, &colors, "Vista previa", lineas);
            }
        }
    }

    let resumen = app.partido_resumen;
    let filtros = format!(
        "Tipo: {} · Resultado: {} · Favoritos: {} · Orden: {}",
        app.partido_filter
            .tipo_partido
            .map_or("Todos".to_string(), |tipo| tipo.label().to_string()),
        app.partido_filter
            .resultado
            .map_or("Todos".to_string(), |resultado| resultado
                .label()
                .to_string()),
        if app.partido_filter.favorito {
            "Sí"
        } else {
            "No"
        },
        app.partido_filter.orden.label()
    );
    let summary = vec![Line::from(vec![
        Span::styled(
            format!(
                " V: {}  E: {}  D: {}  Goles: {}  Asistencias: {}",
                resumen.victorias,
                resumen.empates,
                resumen.derrotas,
                resumen.goles,
                resumen.asistencias
            ),
            Style::default().fg(colors.fg),
        ),
        Span::styled(format!("    {filtros}"), Style::default().fg(colors.muted)),
    ])];
    frame.render_widget(Paragraph::new(summary), summary_area);
}

/// Detalle del partido.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, id: i64) {
    let colors = palette(app.settings.theme);
    let Some(partido) = &app.partido_detail else {
        frame.render_widget(
            Paragraph::new(format!("Cargando partido {id}…"))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Partido "),
                )
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(
            "#{}  {}  {}",
            partido.id,
            crate::domain::dates::display_datetime(partido.fecha_hora),
            partido.tipo_partido.label()
        ),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    if partido.favorito {
        lines.push(Line::from(Span::styled(
            "★ Favorito",
            Style::default().fg(colors.warning),
        )));
    }

    push_heading(&mut lines, &colors, "Marcador");
    push_line(&mut lines, &colors, "Resultado", partido.resultado.label());
    push_line(
        &mut lines,
        &colors,
        "Goles / Asistencias",
        &format!("{} / {}", partido.goles, partido.asistencias),
    );
    push_line(
        &mut lines,
        &colors,
        "Goles en contra",
        &partido.goles_en_contra.to_string(),
    );
    if partido.goles_equipo >= 0 || partido.goles_rival >= 0 {
        push_line(
            &mut lines,
            &colors,
            "Marcador global",
            &format!("{}-{}", partido.goles_equipo, partido.goles_rival),
        );
    }
    push_line(
        &mut lines,
        &colors,
        "Diferencia",
        &partido.diferencia_gol.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Formato",
        partido.formato_partido.as_str(),
    );

    push_heading(&mut lines, &colors, "Contexto");
    push_line(&mut lines, &colors, "Rival", &partido.rival_nombre);
    push_line(&mut lines, &colors, "Tipo de rival", &partido.tipo_rival);
    push_line(&mut lines, &colors, "Posición", &partido.posicion_jugada);
    push_line(
        &mut lines,
        &colors,
        "Minutos",
        &partido.minutos_jugados.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Estado de cancha",
        partido.estado_cancha.label(),
    );
    push_line(&mut lines, &colors, "Clima", partido.clima.label());
    push_line(&mut lines, &colors, "Arbitraje", &partido.arbitraje);
    push_line(&mut lines, &colors, "Tarjeta", partido.tarjeta.label());

    push_heading(&mut lines, &colors, "Métricas");
    push_line(
        &mut lines,
        &colors,
        "Rendimiento",
        &partido.rendimiento_general.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Cansancio",
        &partido.cansancio.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Estado de ánimo",
        &partido.estado_animo.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Intensidad",
        &partido.intensidad.to_string(),
    );
    push_line(
        &mut lines,
        &colors,
        "Ratings",
        &format!(
            "T {} · F {} · M {}",
            partido.rating_tecnico, partido.rating_fisico, partido.rating_mental
        ),
    );
    push_line(
        &mut lines,
        &colors,
        "Dolor físico",
        partido.dolor_fisico.label(),
    );
    if let Some(temperatura) = partido.temperatura_c {
        push_line(
            &mut lines,
            &colors,
            "Temperatura",
            &format!("{temperatura:.1} °C"),
        );
    }

    push_heading(&mut lines, &colors, "Notas");
    push_line(
        &mut lines,
        &colors,
        "Comentario",
        &partido.comentario_personal,
    );
    push_line(&mut lines, &colors, "Eventos clave", &partido.eventos_clave);
    push_line(&mut lines, &colors, "Lo mejor", &partido.lo_mejor);
    push_line(&mut lines, &colors, "Qué mejorar", &partido.que_mejorar);
    if !partido.tags.is_empty() {
        push_line(&mut lines, &colors, "Etiquetas", &partido.tags.join(", "));
    }
    push_line(
        &mut lines,
        &colors,
        "Goles (detalle)",
        &partido.goles_detalle,
    );
    push_line(
        &mut lines,
        &colors,
        "Asistencias (detalle)",
        &partido.asistencias_detalle,
    );

    push_heading(&mut lines, &colors, "Finanzas");
    push_line(&mut lines, &colors, "Precio", &partido.precio.display_ars());
    if partido.precio.0 > 0 {
        push_line(
            &mut lines,
            &colors,
            "Transacción",
            "Se registra automáticamente en Financiamiento (categoría Canchas).",
        );
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(
                        " Partido · {} ",
                        crate::domain::dates::display_datetime(partido.fecha_hora)
                    ))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        area,
    );
}

fn push_line(lines: &mut Vec<Line<'static>>, colors: &Palette, label: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    lines.push(Line::from(vec![
        Span::styled(format!("{label:<22}"), Style::default().fg(colors.muted)),
        Span::styled(value.to_string(), Style::default().fg(colors.fg)),
    ]));
}

fn push_heading(lines: &mut Vec<Line<'static>>, colors: &Palette, texto: &str) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        texto.to_string(),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}

/// Asistente por pasos de creación (§12.4).
pub fn draw_wizard(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(form) = &app.form else {
        return;
    };
    let paso = app.partido_wizard_step;
    let (titulo, claves) = crate::app_partidos::WIZARD_PASOS
        .get(paso)
        .map_or(("Paso 4/4 · Confirmar", &[][..]), |(titulo, claves)| {
            (*titulo, *claves)
        });

    // Indicador de pasos con Tabs.
    let [tabs_area, resto] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let pasos = [
        " 1 · Básico ",
        " 2 · Contexto ",
        " 3 · Métricas ",
        " 4 · Confirmar ",
    ];
    let tabs = Tabs::new(pasos.to_vec())
        .select(paso.min(3))
        .style(Style::default().fg(colors.muted))
        .highlight_style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .divider("›")
        .padding(" ", " ");
    frame.render_widget(tabs, tabs_area);
    let area = resto;

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" Asistente · {titulo} "))
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));

    if paso >= 3 {
        let mut lineas: Vec<Line<'static>> = Vec::new();
        for clave in [
            "tipo_partido",
            "fecha_hora",
            "cancha_id",
            "camiseta_id",
            "resultado",
            "goles",
            "asistencias",
            "rival_nombre",
            "minutos",
            "precio",
        ] {
            if let Some(field) = form.fields.iter().find(|field| field.key == clave) {
                lineas.push(Line::from(vec![
                    Span::styled(
                        format!("{:<22}", field.label),
                        Style::default().fg(colors.muted),
                    ),
                    Span::styled(field.display_value(), Style::default().fg(colors.fg)),
                ]));
            }
        }
        lineas.push(Line::from(""));
        lineas.push(Line::from(Span::styled(
            "Enter o Ctrl+S: guardar el partido",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(
            Paragraph::new(lineas)
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().bg(colors.bg)),
            area,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [campos_area, pie_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(inner);

    let indices: Vec<usize> = form
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| claves.contains(&field.key))
        .map(|(indice, _)| indice)
        .collect();
    let posicion_foco = indices
        .iter()
        .position(|indice| *indice == form.focus)
        .unwrap_or(0);
    let visible = campos_area.height as usize;
    let offset = if visible == 0 || posicion_foco < visible {
        0
    } else {
        posicion_foco + 1 - visible
    };

    let lineas: Vec<Line<'static>> = indices
        .iter()
        .map(|indice| {
            let field = &form.fields[*indice];
            let enfocado = *indice == form.focus;
            let prefijo = if enfocado { "▶ " } else { "  " };
            let estilo_etiqueta = if enfocado {
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.muted)
            };
            let estilo_valor = if enfocado {
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            };
            let mut valor = field.display_value();
            if valor.is_empty() {
                valor = if enfocado {
                    "▌".to_string()
                } else {
                    String::new()
                };
            }
            Line::from(vec![
                Span::styled(prefijo, Style::default().fg(colors.accent)),
                Span::styled(format!("{:<22}", field.label), estilo_etiqueta),
                Span::styled(valor, estilo_valor),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lineas)
            .scroll((offset as u16, 0))
            .style(Style::default().bg(colors.bg)),
        campos_area,
    );

    let pie = match (&form.error, form.fields.get(form.focus)) {
        (Some(error), _) => Line::from(Span::styled(
            format!("[!] {error}"),
            Style::default().fg(colors.error),
        )),
        (None, Some(field)) => Line::from(vec![
            Span::styled(
                "Tab: campo   Enter/PgDn: siguiente   PgUp: anterior   Esc: cancelar",
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!("   {}", field.help.clone().unwrap_or_default()),
                Style::default().fg(colors.muted),
            ),
        ]),
        (None, None) => Line::from(""),
    };
    frame.render_widget(
        Paragraph::new(pie)
            .block(Block::default().style(Style::default().bg(colors.bg)))
            .wrap(Wrap { trim: true }),
        pie_area,
    );
}
