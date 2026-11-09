//! Pantallas del módulo Financiamiento (§15.6).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, Block, BorderType, Cell, HighlightSpacing, LineGauge, Paragraph, Row, Table,
    Wrap,
};

use crate::app::App;
use crate::domain::models::AlertaPresupuesto;
use crate::ui::theme::palette;
use crate::ui::widgets::{detail, form};

/// Listado con resumen, presupuesto (Gauge), gastos por categoría y vista previa.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [summary_area, table_area] = Layout::vertical([Constraint::Length(7), Constraint::Min(5)])
        .spacing(1)
        .areas(area);

    let bloque = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Resumen financiero ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    let interior = bloque.inner(summary_area);
    frame.render_widget(bloque, summary_area);

    let resumen = app.finanzas_resumen;
    let totales = Line::from(vec![
        Span::styled(" Ingresos ", Style::default().fg(colors.success)),
        Span::styled(
            resumen.ingresos.display_ars(),
            Style::default().fg(colors.fg),
        ),
        Span::styled("   Gastos ", Style::default().fg(colors.error)),
        Span::styled(resumen.gastos.display_ars(), Style::default().fg(colors.fg)),
        Span::styled("   Balance ", Style::default().fg(colors.accent)),
        Span::styled(
            resumen.balance.display_ars(),
            Style::default()
                .fg(if resumen.balance.0 >= 0 {
                    colors.success
                } else {
                    colors.error
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let ancho = interior.width >= 100;
    let (izquierda, derecha) = if ancho {
        let [izq, der] = Layout::horizontal([Constraint::Percentage(58), Constraint::Min(0)])
            .spacing(1)
            .areas(interior);
        (izq, Some(der))
    } else {
        (interior, None)
    };

    let [fila_totales, fila_presupuesto, area_gauge] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(izquierda);

    frame.render_widget(Paragraph::new(totales), fila_totales);

    let mut texto_presupuesto = Line::from(Span::styled(
        " Sin presupuesto mensual configurado (p: configurar)",
        Style::default().fg(colors.muted),
    ));
    let mut color_gauge = colors.success;
    let mut ratio = 0.0;
    if let Some(vista) = &app.presupuesto {
        let limite = vista.presupuesto.limite_gasto.0.max(1);
        let porcentaje = (vista.gasto_real.0 as f64 / limite as f64).clamp(0.0, 1.0);
        ratio = porcentaje;
        color_gauge = match vista.alerta {
            AlertaPresupuesto::Ok => colors.success,
            AlertaPresupuesto::LimiteSuperado => colors.warning,
            AlertaPresupuesto::PresupuestoSuperado => colors.error,
        };
        texto_presupuesto = Line::from(vec![
            Span::styled(
                format!(" Presupuesto {}: ", vista.presupuesto.mes_anio),
                Style::default().fg(colors.muted),
            ),
            Span::styled(
                format!(
                    "{} de {} ",
                    vista.gasto_real.display_ars(),
                    vista.presupuesto.limite_gasto.display_ars()
                ),
                Style::default().fg(colors.fg),
            ),
            Span::styled(
                vista.alerta.label(),
                Style::default()
                    .fg(color_gauge)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }
    frame.render_widget(Paragraph::new(texto_presupuesto), fila_presupuesto);

    if app.presupuesto.is_some() {
        let gauge = LineGauge::default()
            .ratio(ratio)
            .label(format!("{:.0}% consumido", ratio * 100.0))
            .filled_style(Style::default().fg(color_gauge))
            .unfilled_style(Style::default().fg(colors.border))
            .style(Style::default().fg(colors.fg));
        frame.render_widget(gauge, area_gauge);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Configure un presupuesto mensual para ver el consumo.",
                Style::default().fg(colors.muted),
            ))),
            area_gauge,
        );
    }

    if let Some(derecha) = derecha {
        let maximo = app
            .finanzas_categorias
            .iter()
            .take(5)
            .map(|categoria| categoria.total.0 / 100)
            .max()
            .unwrap_or(0)
            .max(1) as u64;
        let barras: Vec<Bar> = app
            .finanzas_categorias
            .iter()
            .take(5)
            .map(|categoria| {
                Bar::default()
                    .label(categoria.categoria.label().to_string())
                    .value((categoria.total.0 / 100).max(0) as u64)
                    .style(Style::default().fg(colors.accent))
            })
            .collect();
        if !barras.is_empty() {
            let chart = BarChart::new(barras)
                .bar_width(7)
                .bar_gap(1)
                .max(maximo)
                .value_style(Style::default().fg(colors.fg))
                .label_style(Style::default().fg(colors.muted))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Gastos por categoría ")
                        .border_style(Style::default().fg(colors.border)),
                );
            frame.render_widget(chart, derecha);
        }
    }

    let (tabla_area, preview_area) = match detail::split(table_area) {
        Some((tabla, panel)) => (tabla, Some(panel)),
        None => (table_area, None),
    };

    let bloque_tabla = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Movimientos ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));

    if app.transacciones.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay transacciones.\n\nn: agregar      /: buscar      Esc: volver")
                .block(bloque_tabla)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(colors.fg)),
            tabla_area,
        );
        return;
    }

    let header = Row::new(vec![
        "Fecha",
        "Tipo",
        "Categoría",
        "Descripción",
        "Monto",
        "Ítem",
    ])
    .style(
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.transacciones.iter().map(|transaccion| {
        let color = match transaccion.tipo {
            crate::domain::models::TipoTransaccion::Ingreso => colors.success,
            crate::domain::models::TipoTransaccion::Gasto => colors.error,
        };
        Row::new(vec![
            Cell::from(crate::domain::dates::display_date(transaccion.fecha)),
            Cell::from(transaccion.tipo.label()),
            Cell::from(transaccion.categoria.label()),
            Cell::from(transaccion.descripcion.clone()),
            Cell::from(transaccion.monto.display_ars()),
            Cell::from(transaccion.item_especifico.clone()),
        ])
        .style(Style::default().fg(color))
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(11),
            Constraint::Length(9),
            Constraint::Length(15),
            Constraint::Min(20),
            Constraint::Length(13),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(bloque_tabla)
    .column_spacing(1)
    .highlight_spacing(HighlightSpacing::Always)
    .row_highlight_style(
        Style::default()
            .bg(colors.highlight_bg)
            .fg(colors.highlight_fg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, tabla_area, &mut app.transacciones_table);
    app.table_hits = crate::ui::widgets::hit::table_hits(
        tabla_area,
        app.transacciones_table.offset(),
        app.transacciones.len(),
    );
    app.table_hit_target = Some(crate::app::TableHitTarget::Financiamiento);

    if let Some(panel) = preview_area {
        let seleccionada = app
            .transacciones_table
            .selected()
            .and_then(|indice| app.transacciones.get(indice));
        let mut lineas = Vec::new();
        if let Some(transaccion) = seleccionada {
            lineas.push(Line::from(Span::styled(
                transaccion.descripcion.clone(),
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
                "Fecha",
                crate::domain::dates::display_date(transaccion.fecha),
            );
            push("Tipo", transaccion.tipo.label().to_string());
            push("Categoría", transaccion.categoria.label().to_string());
            push("Monto", transaccion.monto.display_ars());
            push("Ítem", transaccion.item_especifico.clone());
            if let Some(partido_id) = transaccion.partido_id {
                push("Partido", format!("#{partido_id}"));
            }
            lineas.push(Line::from(""));
            lineas.push(Line::from(Span::styled(
                "e: editar   d: eliminar   p: presupuesto",
                Style::default().fg(colors.muted),
            )));
        }
        detail::panel(frame, panel, &colors, "Vista previa", lineas);
    }
}

/// Formulario de transacción o presupuesto.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(form) = &app.form {
        form::draw(frame, area, app, form);
    }
}
