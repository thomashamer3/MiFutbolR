//! Catálogo de logros (IMP-04).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Tabs, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Dibuja el catálogo.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [tabs_area, lista_area, pie_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .areas(area);

    let titulos: Vec<Line> = crate::app::LogrosTab::ALL
        .iter()
        .map(|tab| Line::from(tab.label()))
        .collect();
    let seleccionado = crate::app::LogrosTab::ALL
        .iter()
        .position(|tab| *tab == app.logros_tab)
        .unwrap_or(0);
    let tabs = Tabs::new(titulos)
        .select(seleccionado)
        .style(Style::default().fg(colors.muted))
        .highlight_style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│")
        .padding(" ", " ");
    frame.render_widget(tabs, tabs_area);

    let visibles: Vec<(String, String, String, String)> = app
        .logros_filtrados()
        .iter()
        .map(|logro| {
            let ancho = 10usize;
            let llenos = (logro.progreso() * ancho as f64).round() as usize;
            let barra = format!(
                "{}{}",
                "█".repeat(llenos.min(ancho)),
                "░".repeat(ancho.saturating_sub(llenos))
            );
            (
                if logro.completado {
                    "OK".to_string()
                } else {
                    "  ".to_string()
                },
                logro.def.nombre.clone(),
                format!("{barra} {}", logro.texto_progreso()),
                logro.def.categoria.clone(),
            )
        })
        .collect();

    if visibles.is_empty() {
        let bloque = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Logros ")
            .border_style(Style::default().fg(colors.border));
        frame.render_widget(
            Paragraph::new("No hay logros en esta pestaña. Jugá un partido para empezar.")
                .block(bloque)
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            lista_area,
        );
    } else {
        let filas: Vec<Row> = visibles
            .into_iter()
            .map(|(estado, nombre, progreso, categoria)| {
                Row::new(vec![
                    Cell::from(estado),
                    Cell::from(nombre),
                    Cell::from(progreso),
                    Cell::from(categoria),
                ])
            })
            .collect();
        let anchos = [
            Constraint::Length(3),
            Constraint::Min(24),
            Constraint::Length(22),
            Constraint::Length(14),
        ];
        let tabla = Table::new(filas, anchos)
            .header(
                Row::new(vec!["✓", "Logro", "Progreso", "Categoría"]).style(
                    Style::default()
                        .fg(colors.muted)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Logros ")
                    .border_style(Style::default().fg(colors.border)),
            )
            .row_highlight_style(
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.highlight_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(tabla, lista_area, &mut app.logros_table);
    }

    let completados = crate::services::logros_service::completados(&app.logros);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                " {} / {} logros · {} · h/l: pestaña  j/k: mover  r: refrescar ",
                completados,
                app.logros.len(),
                app.logros_tab.label()
            ),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        ))),
        pie_area,
    );
}

/// Área interna de una tabla para tests de layout.
#[must_use]
pub fn area_util(area: Rect) -> Rect {
    Block::bordered().inner(area)
}
