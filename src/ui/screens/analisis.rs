//! Análisis básico, evolución temporal y química (port de `analisis.c`).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::domain::analisis::{Promedios, Tendencia};
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::form;

fn titulo(colors: &Palette, texto: &str) -> Line<'static> {
    Line::from(Span::styled(
        texto.to_string(),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    ))
}

fn dato(colors: &Palette, etiqueta: &str, valor: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{etiqueta:<26}"), Style::default().fg(colors.muted)),
        Span::styled(valor, Style::default().fg(colors.fg)),
    ])
}

fn grupo(lineas: &mut Vec<Line<'static>>, colors: &Palette, nombre: &str, p: &Promedios) {
    lineas.push(dato(
        colors,
        nombre,
        if p.partidos == 0 {
            "sin partidos".to_string()
        } else {
            format!(
                "G {:.2} · A {:.2} · Rend {:.2} · {} partidos",
                p.goles, p.asistencias, p.rendimiento, p.partidos
            )
        },
    ));
}

fn sin_datos(frame: &mut Frame, area: Rect, colors: &Palette, titulo: &str) {
    frame.render_widget(
        Paragraph::new(
            "No hay suficientes datos para el análisis. Registrá algunos partidos para verlo.",
        )
        .block(colors.panel(format!(" {titulo} ")))
        .style(Style::default().fg(colors.muted))
        .wrap(Wrap { trim: true }),
        area,
    );
}

const fn resultado(codigo: i64) -> &'static str {
    match codigo {
        1 => "Victoria",
        2 => "Empate",
        3 => "Derrota",
        _ => "—",
    }
}

/// Análisis básico (`mostrar_analisis_basico`).
pub fn draw_basico(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(b) = &app.analisis_basico else {
        sin_datos(frame, area, &colors, "Análisis básico");
        return;
    };
    let mut lineas = vec![titulo(&colors, "Últimos 5 partidos")];
    for p in &b.ultimos {
        lineas.push(Line::from(Span::styled(
            format!(
                "  {} · G {} A {} · Rend {} · {}",
                p.fecha_hora.get(..16).unwrap_or(&p.fecha_hora),
                p.goles,
                p.asistencias,
                p.rendimiento,
                resultado(p.resultado)
            ),
            Style::default().fg(colors.fg),
        )));
    }
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Últimos 5 vs promedio general"));
    let (u, g) = (&b.ultimos5, &b.general);
    for (nombre, ultimos, general) in [
        ("Goles", u.goles, g.goles),
        ("Asistencias", u.asistencias, g.asistencias),
        ("Rendimiento", u.rendimiento, g.rendimiento),
        ("Cansancio", u.cansancio, g.cansancio),
        ("Estado de ánimo", u.animo, g.animo),
    ] {
        lineas.push(dato(
            &colors,
            nombre,
            format!("{ultimos:.1} vs {general:.1}"),
        ));
    }
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Rachas"));
    lineas.push(dato(
        &colors,
        "Mejor racha de victorias",
        format!("{} partidos", b.mejor_racha_victorias),
    ));
    lineas.push(dato(
        &colors,
        "Peor racha de derrotas",
        format!("{} partidos", b.peor_racha_derrotas),
    ));
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Análisis motivacional"));
    lineas.push(Line::from(Span::styled(
        b.mensaje,
        Style::default().fg(colors.fg),
    )));
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(format!(" Análisis básico · {} partidos ", g.partidos)))
            .wrap(Wrap { trim: false })
            .scroll((app.analisis_scroll, 0)),
        area,
    );
}

/// Evolución temporal (las 8 opciones de `mostrar_evolucion_temporal`).
pub fn draw_evolucion(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(e) = &app.analisis_evolucion else {
        sin_datos(frame, area, &colors, "Evolución temporal");
        return;
    };
    let metrica = app.analisis_metrica;
    let alto_grafico = if area.height >= 20 { 10 } else { 0 };
    let [grafico_area, texto_area] =
        Layout::vertical([Constraint::Length(alto_grafico), Constraint::Min(3)]).areas(area);

    if alto_grafico > 0 {
        // Últimos 12 meses en orden cronológico; valores x10 porque BarChart usa enteros.
        let barras: Vec<Bar> = e
            .meses
            .iter()
            .take(12)
            .rev()
            .map(|mes| {
                let valor = metrica.valor(&mes.promedios);
                Bar::default()
                    .label(mes.corta())
                    .value((valor * 10.0).round().max(0.0) as u64)
                    .text_value(format!("{valor:.1}"))
            })
            .collect();
        frame.render_widget(
            BarChart::new(barras)
                .bar_width(5)
                .bar_gap(1)
                .bar_style(Style::default().fg(colors.accent))
                .value_style(Style::default().fg(colors.fg))
                .label_style(Style::default().fg(colors.muted))
                .block(colors.panel(format!(
                    " {} promedio por mes · m: cambiar métrica ",
                    metrica.label()
                ))),
            grafico_area,
        );
    }

    let mut lineas = vec![titulo(
        &colors,
        &format!("Evolución mensual de {}", metrica.label().to_lowercase()),
    )];
    for mes in &e.meses {
        lineas.push(dato(
            &colors,
            &mes.etiqueta(),
            format!(
                "{:.2} ({} partidos)",
                metrica.valor(&mes.promedios),
                mes.promedios.partidos
            ),
        ));
    }
    lineas.push(Line::from(""));
    for (nombre, indice) in [
        ("Mejor mes histórico", e.mejor_mes),
        ("Peor mes histórico", e.peor_mes),
    ] {
        let mes = &e.meses[indice];
        lineas.push(dato(
            &colors,
            nombre,
            format!(
                "{} · rendimiento {:.2} · {} partidos",
                mes.etiqueta(),
                mes.promedios.rendimiento,
                mes.promedios.partidos
            ),
        ));
    }
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Inicio vs fin de año"));
    grupo(&mut lineas, &colors, "Inicio (Ene–Jun)", &e.inicio_anio);
    grupo(&mut lineas, &colors, "Fin (Jul–Dic)", &e.fin_anio);
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Meses fríos vs cálidos"));
    grupo(&mut lineas, &colors, "Fríos (Jun–Sep)", &e.frios);
    grupo(&mut lineas, &colors, "Cálidos (Dic–Abr)", &e.calidos);
    lineas.push(Line::from(""));
    lineas.push(titulo(&colors, "Progreso total del jugador"));
    let progreso = &e.progreso;
    lineas.push(dato(
        &colors,
        "Período",
        format!(
            "{} – {}",
            progreso.desde.get(..10).unwrap_or(&progreso.desde),
            progreso.hasta.get(..10).unwrap_or(&progreso.hasta)
        ),
    ));
    grupo(&mut lineas, &colors, "Promedios", &progreso.general);
    match progreso.tendencia {
        Some((primeros, ultimos)) => lineas.push(dato(
            &colors,
            "Tendencia (rendimiento)",
            format!(
                "primeros 5 {primeros:.2} · últimos 5 {ultimos:.2} · {} ({:+.2})",
                Tendencia::de(ultimos - primeros).label(),
                ultimos - primeros
            ),
        )),
        None => lineas.push(dato(
            &colors,
            "Tendencia (rendimiento)",
            "hacen falta 10 partidos".to_string(),
        )),
    }
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(" Evolución temporal "))
            .wrap(Wrap { trim: false })
            .scroll((app.analisis_scroll, 0)),
        texto_area,
    );
}

/// Química: mejor combinación automática y duplas cargadas a mano.
pub fn draw_quimica(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [mejor_area, tabla_area] =
        Layout::vertical([Constraint::Length(4), Constraint::Min(4)]).areas(area);
    let mejor = match &app.quimica_mejor {
        Some(m) => vec![
            Line::from(Span::styled(
                format!("{} + {}", m.jugador_a, m.jugador_b),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(
                    "V {} · D {} · E {} · winrate {:.1}%",
                    m.victorias,
                    m.derrotas,
                    m.empates,
                    m.winrate()
                ),
                Style::default().fg(colors.fg),
            )),
        ],
        None => vec![Line::from(Span::styled(
            "Sin datos: asigná equipos con jugadores a tus partidos.",
            Style::default().fg(colors.muted),
        ))],
    };
    frame.render_widget(
        Paragraph::new(mejor).block(colors.panel(" Mejor combinación (equipos de partidos) ")),
        mejor_area,
    );

    if app.quimica_duplas.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin duplas cargadas. Presioná n para registrar una.")
                .block(colors.panel(" Duplas "))
                .style(Style::default().fg(colors.muted)),
            tabla_area,
        );
        return;
    }
    let filas: Vec<Row> = app
        .quimica_duplas
        .iter()
        .map(|d| {
            Row::new(vec![
                Cell::from(format!("{} + {}", d.jugador1, d.jugador2)),
                Cell::from(d.partidos.to_string()),
                Cell::from(d.victorias.to_string()),
                Cell::from(format!("{:.1}%", d.winrate())),
                Cell::from(format!("{:.2}", d.aporte())),
                Cell::from(d.comentario.clone()),
            ])
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Min(24),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(
        Row::new(vec![
            "Dupla",
            "Partidos",
            "Victorias",
            "Winrate",
            "Aporte",
            "Comentario",
        ])
        .style(
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(colors.panel(format!(" Duplas · {} ", app.quimica_duplas.len())))
    .style(Style::default().fg(colors.fg))
    .row_highlight_style(
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");
    frame.render_stateful_widget(tabla, tabla_area, &mut app.quimica_table);
}

/// Formulario de dupla.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
