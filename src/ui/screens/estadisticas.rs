//! Hub y detalle del módulo Estadísticas (§14.6).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, Block, BorderType, LineGauge, List, ListItem, Paragraph, Sparkline, Wrap,
};

use crate::app::App;
use crate::domain::models::{
    EstadisticaKind, Estadisticas, GolesStats, PeriodoStats, PromedioGrupo,
};
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::{detail, scroll};

/// Hub con las categorías.
pub fn draw_hub(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let items: Vec<ListItem> = EstadisticaKind::ALL
        .iter()
        .map(|kind| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    kind.label(),
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("   {}", kind.description()),
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
                .border_type(BorderType::Rounded)
                .title(" Estadísticas ")
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

    frame.render_stateful_widget(list, lista_area, &mut app.estadisticas_hub);

    if let Some(panel) = resumen_area {
        let mut lineas = Vec::new();
        if let Some(estadisticas) = app.estadisticas() {
            let g = &estadisticas.generales;
            let mut push = |etiqueta: &str, valor: String| {
                lineas.push(Line::from(vec![
                    Span::styled(format!("{etiqueta:<18}"), Style::default().fg(colors.muted)),
                    Span::styled(valor, Style::default().fg(colors.fg)),
                ]));
            };
            push("Partidos", g.total.to_string());
            push(
                "V / E / D",
                format!("{} / {} / {}", g.victorias, g.empates, g.derrotas),
            );
            push("Goles", g.goles.to_string());
            push("Asistencias", g.asistencias.to_string());
            push(
                "Rendimiento",
                format!("{:.2}", estadisticas.rendimiento.promedio),
            );
            push("% victorias", format!("{:.1}%", g.porcentaje_victorias));
            if let Some(mejor) = &g.mejor_partido {
                lineas.push(Line::from(""));
                lineas.push(Line::from(Span::styled(
                    "Mejor partido",
                    Style::default()
                        .fg(colors.muted)
                        .add_modifier(Modifier::BOLD),
                )));
                lineas.push(Line::from(Span::styled(
                    mejor.clone(),
                    Style::default().fg(colors.fg),
                )));
            }
        } else {
            lineas.push(Line::from(Span::styled(
                "Sin datos todavía. Registrá partidos para ver estadísticas.",
                Style::default().fg(colors.muted),
            )));
        }
        lineas.push(Line::from(""));
        lineas.push(Line::from(Span::styled(
            "Enter: abrir categoría   ↑↓: mover",
            Style::default().fg(colors.muted),
        )));
        detail::panel(frame, panel, &colors, "Resumen rápido", lineas);
    }
}

/// Detalle de una categoría.
pub fn draw_detail(frame: &mut Frame, area: Rect, app: &App, kind: EstadisticaKind) {
    let colors = palette(app.settings.theme);
    let Some(estadisticas) = app.estadisticas() else {
        frame.render_widget(
            Paragraph::new("Sin datos suficientes; registra partidos para ver estadísticas.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Estadísticas "),
                )
                .style(Style::default().fg(colors.fg)),
            area,
        );
        return;
    };

    if estadisticas.generales.total == 0 {
        frame.render_widget(
            Paragraph::new("Sin datos suficientes; registra partidos para ver estadísticas.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(format!(" {} ", kind.label()))
                        .border_style(Style::default().fg(colors.border)),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }

    let lines = build_lines(estadisticas, kind, &colors);
    let total_lineas = lines.len();
    let alto = area.height;

    let con_gauge = kind == EstadisticaKind::Generales && alto >= 12;
    let con_sparkline = kind == EstadisticaKind::Rendimiento
        && !estadisticas.rendimiento.tendencia.is_empty()
        && alto >= 18;
    let barras = barras_para(kind, estadisticas);
    let con_grafico = barras.is_some() && alto >= 26;

    let alto_gauge = if con_gauge { 1 } else { 0 };
    let alto_spark = if con_sparkline { 4 } else { 0 };
    let alto_grafico = if con_grafico { 10 } else { 0 };

    let [area_gauge, resto] =
        Layout::vertical([Constraint::Length(alto_gauge), Constraint::Min(3)]).areas(area);
    let [texto_area, spark_area, grafico_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(alto_spark),
        Constraint::Length(alto_grafico),
    ])
    .areas(resto);

    if con_gauge {
        let g = &estadisticas.generales;
        let ratio = (g.porcentaje_victorias / 100.0).clamp(0.0, 1.0);
        let gauge = LineGauge::default()
            .ratio(ratio)
            .label(format!(
                "Victorias {:.1}% · {} partidos",
                g.porcentaje_victorias, g.total
            ))
            .filled_style(Style::default().fg(colors.success))
            .unfilled_style(Style::default().fg(colors.border))
            .style(Style::default().fg(colors.fg));
        frame.render_widget(gauge, area_gauge);
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(format!(" {} ", kind.label()))
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .scroll((app.estadisticas_scroll, 0))
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
        &colors,
        total_lineas,
        app.estadisticas_scroll as usize,
        texto_area.height.saturating_sub(2) as usize,
    );

    if con_sparkline {
        let datos: Vec<u64> = estadisticas
            .rendimiento
            .tendencia
            .iter()
            .map(|valor| (*valor).round().clamp(0.0, 10.0) as u64)
            .collect();
        let sparkline = Sparkline::default()
            .data(&datos)
            .max(10)
            .style(Style::default().fg(colors.accent))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Tendencia de rendimiento ")
                    .border_style(Style::default().fg(colors.border)),
            );
        frame.render_widget(sparkline, spark_area);
    }

    if con_grafico && let Some((titulo, barras)) = barras {
        dibujar_barras(frame, grafico_area, &colors, &titulo, barras);
    }
}

fn barras_para(
    kind: EstadisticaKind,
    estadisticas: &Estadisticas,
) -> Option<(String, Vec<Bar<'static>>)> {
    let (titulo, grupos): (&str, Vec<(&str, i64)>) = match kind {
        EstadisticaKind::Rendimiento => (
            "Rendimiento por clima",
            estadisticas
                .rendimiento
                .por_clima
                .iter()
                .take(6)
                .map(|grupo| (grupo.clave.as_str(), grupo.promedio.round() as i64))
                .collect(),
        ),
        EstadisticaKind::Goles => (
            "Goles por franja (x10)",
            estadisticas
                .goles
                .por_franja
                .iter()
                .take(7)
                .map(|grupo| (grupo.clave.as_str(), (grupo.promedio * 10.0).round() as i64))
                .collect(),
        ),
        EstadisticaKind::Canchas => (
            "Partidos por cancha",
            estadisticas
                .canchas
                .ranking
                .iter()
                .take(6)
                .map(|cancha| (cancha.nombre.as_str(), cancha.partidos))
                .collect(),
        ),
        _ => return None,
    };
    if grupos.is_empty() {
        return None;
    }
    let barras = grupos
        .into_iter()
        .map(|(clave, valor)| {
            Bar::default()
                .label(clave.to_string())
                .value(valor.max(0) as u64)
        })
        .collect();
    Some((titulo.to_string(), barras))
}

fn dibujar_barras(
    frame: &mut Frame,
    area: Rect,
    colors: &Palette,
    titulo: &str,
    barras: Vec<Bar<'static>>,
) {
    let chart = BarChart::new(barras)
        .bar_width(4)
        .bar_gap(1)
        .bar_style(Style::default().fg(colors.accent))
        .value_style(Style::default().fg(colors.fg))
        .label_style(Style::default().fg(colors.muted))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(format!(" {titulo} "))
                .border_style(Style::default().fg(colors.border)),
        );
    frame.render_widget(chart, area);
}

fn build_lines(
    estadisticas: &Estadisticas,
    kind: EstadisticaKind,
    colors: &Palette,
) -> Vec<Line<'static>> {
    match kind {
        EstadisticaKind::Generales => generales(estadisticas, colors),
        EstadisticaKind::PorMes => series(&estadisticas.por_mes, colors),
        EstadisticaKind::PorAnio => series(&estadisticas.por_anio, colors),
        EstadisticaKind::Rendimiento => rendimiento(estadisticas, colors),
        EstadisticaKind::Canchas => canchas(estadisticas, colors),
        EstadisticaKind::Goles => goles(&estadisticas.goles, colors),
        EstadisticaKind::Meta => meta(estadisticas, colors),
    }
}

fn generales(estadisticas: &Estadisticas, colors: &Palette) -> Vec<Line<'static>> {
    let g = &estadisticas.generales;
    let mut lines = Vec::new();
    linea(
        &mut lines,
        colors,
        "Total de partidos",
        &g.total.to_string(),
    );
    linea(
        &mut lines,
        colors,
        "Goles / asistencias",
        &format!("{} / {}", g.goles, g.asistencias),
    );
    linea(
        &mut lines,
        colors,
        "Promedios",
        &format!(
            "G {:.2} · A {:.2} · Rend {:.2}",
            g.promedio_goles, g.promedio_asistencias, g.promedio_rendimiento
        ),
    );
    linea(
        &mut lines,
        colors,
        "V / E / D",
        &format!(
            "{} / {} / {}  ({:.1}% victorias)",
            g.victorias, g.empates, g.derrotas, g.porcentaje_victorias
        ),
    );
    let racha = g.racha_actual.as_ref().map_or_else(
        || "—".to_string(),
        |(tipo, largo)| format!("{tipo} ({largo})"),
    );
    linea(&mut lines, colors, "Racha actual", &racha);
    linea(
        &mut lines,
        colors,
        "Mejor racha",
        &format!("{} victorias", g.mejor_racha),
    );
    linea(
        &mut lines,
        colors,
        "Consistencia (desvío)",
        &format!("{:.2}", g.consistencia),
    );
    if let Some(mejor) = &g.mejor_partido {
        linea(&mut lines, colors, "Mejor partido", mejor);
    }
    if let Some(peor) = &g.peor_partido {
        linea(&mut lines, colors, "Peor partido", peor);
    }
    encabezado(&mut lines, colors, "Récords");
    for (nombre, valor) in &estadisticas.records {
        linea(&mut lines, colors, nombre, valor);
    }
    lines
}

fn series(series: &[PeriodoStats], colors: &Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if series.is_empty() {
        lines.push(Line::from("Sin datos."));
        return lines;
    }
    lines.push(Line::from(Span::styled(
        format!(
            "{:<10} {:>4}  {:>4}  {:>4}  {:>8}",
            "Período", "P", "G", "A", "Rend."
        ),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    for periodo in series {
        lines.push(Line::from(Span::styled(
            format!(
                "{:<10} {:>4}  {:>4}  {:>4}  {:>8.2}",
                periodo.clave,
                periodo.partidos,
                periodo.goles,
                periodo.asistencias,
                periodo.rendimiento
            ),
            Style::default().fg(colors.fg),
        )));
    }
    lines
}

fn rendimiento(estadisticas: &Estadisticas, colors: &Palette) -> Vec<Line<'static>> {
    let r = &estadisticas.rendimiento;
    let mut lines = Vec::new();
    linea(
        &mut lines,
        colors,
        "Promedio general",
        &format!("{:.2}", r.promedio),
    );
    seccion_grupos(&mut lines, colors, "Por clima", &r.por_clima);
    seccion_grupos(
        &mut lines,
        colors,
        "Por día de la semana",
        &r.por_dia_semana,
    );
    seccion_grupos(&mut lines, colors, "Por franja horaria", &r.por_franja);
    seccion_grupos(&mut lines, colors, "Por cansancio", &r.por_cansancio);
    seccion_grupos(&mut lines, colors, "Por estado de ánimo", &r.por_animo);
    seccion_grupos(&mut lines, colors, "Por intensidad", &r.por_intensidad);
    seccion_grupos(&mut lines, colors, "Por dolor físico", &r.por_dolor);
    seccion_grupos(&mut lines, colors, "Por arbitraje", &r.por_arbitraje);
    seccion_grupos(&mut lines, colors, "Por temperatura", &r.por_temperatura);
    lines
}

fn canchas(estadisticas: &Estadisticas, colors: &Palette) -> Vec<Line<'static>> {
    let c = &estadisticas.canchas;
    let mut lines = Vec::new();
    linea(
        &mut lines,
        colors,
        "Resumen",
        &format!(
            "{} canchas ({} activas) · {} partidos",
            c.total, c.activas, c.partidos
        ),
    );
    encabezado(&mut lines, colors, "Más usadas");
    for cancha in c.ranking.iter().take(5) {
        lines.push(linea_cancha(cancha, colors));
    }
    encabezado(&mut lines, colors, "Menos usadas");
    for cancha in c.ranking.iter().rev().take(5) {
        lines.push(linea_cancha(cancha, colors));
    }
    encabezado(&mut lines, colors, "Rendimiento y aportes por cancha");
    lines.push(Line::from(Span::styled(
        format!(
            "{:<24} {:>4} {:>4} {:>4} {:>8}",
            "Cancha", "P", "G", "A", "Rend."
        ),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    for cancha in &c.ranking {
        lines.push(Line::from(Span::styled(
            format!(
                "{:<24} {:>4} {:>4} {:>4} {:>8.2}",
                cancha.nombre,
                cancha.partidos,
                cancha.goles,
                cancha.asistencias,
                cancha.rendimiento
            ),
            Style::default().fg(colors.fg),
        )));
    }
    seccion_grupos(
        &mut lines,
        colors,
        "Techadas vs descubiertas",
        &c.por_techada,
    );
    seccion_grupos(&mut lines, colors, "Por tipo de cancha", &c.por_tipo);
    seccion_grupos(&mut lines, colors, "Por superficie", &c.por_superficie);
    seccion_grupos(&mut lines, colors, "Por franja horaria", &c.por_franja);
    seccion_grupos(&mut lines, colors, "Por localidad", &c.por_localidad);
    lines
}

fn goles(g: &GolesStats, colors: &Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    linea(
        &mut lines,
        colors,
        "Promedio goles / asistencias",
        &format!("{:.2} / {:.2}", g.promedio_goles, g.promedio_asistencias),
    );
    linea(
        &mut lines,
        colors,
        "Eficiencia / efectividad",
        &format!("{:.2} / {:.2}", g.eficiencia, g.efectividad),
    );
    linea(
        &mut lines,
        colors,
        "Goles con cansancio alto / bajo",
        &format!(
            "{:.2} / {:.2}",
            g.goles_cansancio_alto, g.goles_cansancio_bajo
        ),
    );
    linea(
        &mut lines,
        colors,
        "Partidos atípicos (|z| > 2)",
        &g.atipicos.to_string(),
    );
    linea(
        &mut lines,
        colors,
        "Exigentes bien rendidos",
        &g.exigentes_bien.to_string(),
    );
    linea(
        &mut lines,
        colors,
        "Fáciles mal rendidos",
        &g.faciles_mal.to_string(),
    );
    seccion_grupos(&mut lines, colors, "Goles por clima", &g.por_clima);
    seccion_grupos(&mut lines, colors, "Goles por franja", &g.por_franja);
    lines
}

fn seccion_grupos(
    lines: &mut Vec<Line<'static>>,
    colors: &Palette,
    titulo: &str,
    grupos: &[PromedioGrupo],
) {
    if grupos.is_empty() {
        return;
    }
    encabezado(lines, colors, titulo);
    for grupo in grupos {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<22}", grupo.clave),
                Style::default().fg(colors.fg),
            ),
            Span::styled(
                format!(
                    "{:>4} partidos · prom. {:.2}",
                    grupo.partidos, grupo.promedio
                ),
                Style::default().fg(colors.muted),
            ),
        ]));
    }
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

fn linea(lines: &mut Vec<Line<'static>>, colors: &Palette, etiqueta: &str, valor: &str) {
    lines.push(Line::from(vec![
        Span::styled(format!("{etiqueta:<26}"), Style::default().fg(colors.muted)),
        Span::styled(valor.to_string(), Style::default().fg(colors.fg)),
    ]));
}

fn linea_cancha(cancha: &crate::domain::models::CanchaStats, colors: &Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(
            "{:<24} {:>4} {:>4} {:>4} {:>8.2}",
            cancha.nombre, cancha.partidos, cancha.goles, cancha.asistencias, cancha.rendimiento
        ),
        Style::default().fg(colors.fg),
    ))
}

fn meta(estadisticas: &Estadisticas, colors: &Palette) -> Vec<Line<'static>> {
    let m = &estadisticas.meta;
    let mut lines = Vec::new();
    linea(
        &mut lines,
        colors,
        "Consistencia",
        &format!(
            "{} partidos · prom {:.2} · desvío {:.2} · CV {:.1}% · rango {}–{}",
            m.partidos, m.promedio, m.desvio, m.cv, m.minimo, m.maximo
        ),
    );
    linea(
        &mut lines,
        colors,
        "Partidos atípicos (|z| > 2)",
        &m.outliers.to_string(),
    );
    linea(
        &mut lines,
        colors,
        "Impacto del cansancio",
        &format!(
            "alto {:.2} vs bajo {:.2} (Δ {:.2})",
            m.cansancio_alto,
            m.cansancio_bajo,
            m.cansancio_alto - m.cansancio_bajo
        ),
    );
    linea(
        &mut lines,
        colors,
        "Impacto del estado de ánimo",
        &format!(
            "alto {:.2} vs bajo {:.2} (Δ {:.2})",
            m.animo_alto,
            m.animo_bajo,
            m.animo_alto - m.animo_bajo
        ),
    );
    linea(
        &mut lines,
        colors,
        "Eficiencia",
        &format!(
            "{:.2} goles/partido · {:.2} asistencias/partido",
            m.goles_por_partido, m.asistencias_por_partido
        ),
    );
    if let Some((cancha, promedio)) = &m.mejor_contexto {
        linea(
            &mut lines,
            colors,
            "Mejor contexto",
            &format!("{cancha} · rend. {promedio:.2}"),
        );
    }
    if let Some((cancha, promedio)) = &m.peor_contexto {
        linea(
            &mut lines,
            colors,
            "Peor contexto",
            &format!("{cancha} · rend. {promedio:.2}"),
        );
    }
    if !m.tendencia.is_empty() {
        let valores: Vec<String> = m
            .tendencia
            .iter()
            .map(|valor| format!("{valor:.0}"))
            .collect();
        linea(
            &mut lines,
            colors,
            "Últimos rendimientos",
            &valores.join(" · "),
        );
    }
    seccion_grupos(
        &mut lines,
        colors,
        "Por intensidad",
        &estadisticas.rendimiento.por_intensidad,
    );
    seccion_grupos(
        &mut lines,
        colors,
        "Por dolor físico",
        &estadisticas.rendimiento.por_dolor,
    );
    seccion_grupos(
        &mut lines,
        colors,
        "Por arbitraje",
        &estadisticas.rendimiento.por_arbitraje,
    );
    seccion_grupos(
        &mut lines,
        colors,
        "Por temperatura",
        &estadisticas.rendimiento.por_temperatura,
    );
    lines
}
