//! Comparador A/B (IMP-14).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::domain::comparador::{Comparacion, Lado, OpcionComparador};
use crate::ui::theme::palette;

/// Dibuja el comparador.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let [cabecera, cuerpo] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(6)]).areas(area);
    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled(
                format!(" Comparador · {} ", app.comparador_tipo.label()),
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  1 camisetas · 2 canchas · Tab A/B · j/k mover · Enter comparar",
                Style::default().fg(colors.muted),
            ),
        ])]),
        cabecera,
    );

    let (opciones_area, resultado_area) = if area.width >= 100 {
        let [arriba, abajo] =
            Layout::vertical([Constraint::Length(9), Constraint::Min(4)]).areas(cuerpo);
        (arriba, abajo)
    } else {
        let [arriba, abajo] =
            Layout::vertical([Constraint::Length(10), Constraint::Min(4)]).areas(cuerpo);
        (arriba, abajo)
    };

    let [a_area, b_area] = if area.width >= 100 {
        let [a, b] = Layout::horizontal([Constraint::Ratio(1, 2); 2])
            .spacing(1)
            .areas(opciones_area);
        [a, b]
    } else {
        let [a, b] = Layout::vertical([Constraint::Ratio(1, 2); 2]).areas(opciones_area);
        [a, b]
    };

    draw_opciones(
        frame,
        a_area,
        "A",
        &app.comparador_opciones,
        app.comparador_a,
        app.comparador_activo == 0,
        &colors,
    );
    draw_opciones(
        frame,
        b_area,
        "B",
        &app.comparador_opciones,
        app.comparador_b,
        app.comparador_activo == 1,
        &colors,
    );

    match &app.comparador_resultado {
        Some(comparacion) => draw_resultado(frame, resultado_area, comparacion, &colors),
        None => frame.render_widget(
            Paragraph::new("Elegí A y B y presioná Enter para comparar.")
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Resultado ")
                        .border_style(Style::default().fg(colors.border)),
                )
                .style(Style::default().fg(colors.muted))
                .wrap(Wrap { trim: true }),
            resultado_area,
        ),
    }
}

fn draw_opciones(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    opciones: &[OpcionComparador],
    seleccionada: usize,
    activa: bool,
    colors: &crate::ui::theme::Palette,
) {
    let borde = if activa { colors.accent } else { colors.border };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {etiqueta} "))
        .border_style(Style::default().fg(borde));
    if opciones.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin opciones cargadas.")
                .block(block)
                .style(Style::default().fg(colors.muted)),
            area,
        );
        return;
    }
    let interior = block.inner(area);
    frame.render_widget(block, area);
    let visible = interior.height.max(1) as usize;
    let inicio = seleccionada.saturating_sub(visible.saturating_sub(1));
    let lineas: Vec<Line> = opciones
        .iter()
        .enumerate()
        .skip(inicio)
        .take(visible)
        .map(|(indice, opcion)| {
            let estilo = if indice == seleccionada {
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(if activa {
                        colors.accent
                    } else {
                        colors.highlight_bg
                    })
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            };
            Line::from(Span::styled(format!(" {} ", opcion.nombre), estilo))
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), interior);
}

fn draw_resultado(
    frame: &mut Frame,
    area: Rect,
    comparacion: &Comparacion,
    colors: &crate::ui::theme::Palette,
) {
    let filas: Vec<Row> = comparacion
        .filas
        .iter()
        .map(|fila| {
            let mejor = match fila.mejor {
                Lado::A => "< A",
                Lado::B => "B >",
                Lado::Empate => "=",
            };
            Row::new(vec![
                Cell::from(fila.metrica),
                Cell::from(fila.a.clone()),
                Cell::from(fila.b.clone()),
                Cell::from(mejor),
            ])
            .style(if matches!(fila.mejor, Lado::Empate) {
                Style::default().fg(colors.muted)
            } else {
                Style::default().fg(colors.fg)
            })
        })
        .collect();
    let tabla = Table::new(
        filas,
        [
            Constraint::Min(26),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(6),
        ],
    )
    .header(
        Row::new(vec![
            "Métrica",
            comparacion.titulo_a.as_str(),
            comparacion.titulo_b.as_str(),
            "Mejor",
        ])
        .style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Resultado ")
            .border_style(Style::default().fg(colors.border)),
    );
    frame.render_widget(tabla, area);
}
