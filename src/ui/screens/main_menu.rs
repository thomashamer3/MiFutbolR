//! Menú principal con los 11 módulos (§19.2).
//!
//! Encabezado con tipografía grande (tui-big-text), lista de módulos y tarjeta
//! de vista previa; en pantallas angostas se apila y se usa texto normal.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};
use tui_big_text::{BigText, PixelSize};

use crate::app::{App, Module};
use crate::ui::theme::palette;

/// Dibuja el menú y la ficha del módulo seleccionado.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let ancho = area.width >= 100;
    let alto_titulo = if area.height < 34 {
        5
    } else if area.width >= 100 {
        9
    } else if area.width >= 60 {
        5
    } else {
        2
    };
    let mostrar_hoy = area.height >= 34 && area.width >= 70;
    let alto_hoy = if mostrar_hoy { 8 } else { 0 };
    let [titulo_area, hoy_area, cuerpo] = Layout::vertical([
        Constraint::Length(alto_titulo),
        Constraint::Length(alto_hoy),
        Constraint::Min(8),
    ])
    .areas(area);

    draw_titulo(frame, titulo_area, &colors);
    if mostrar_hoy {
        draw_hoy(frame, hoy_area, app, &colors);
    }

    let (list_area, detail_area) = if ancho {
        let [izquierda, derecha] =
            Layout::horizontal([Constraint::Percentage(44), Constraint::Min(40)])
                .spacing(1)
                .areas(cuerpo);
        (izquierda, derecha)
    } else {
        let [arriba, abajo] =
            Layout::vertical([Constraint::Min(8), Constraint::Length(7)]).areas(cuerpo);
        (arriba, abajo)
    };

    let items: Vec<ListItem> = Module::ALL
        .iter()
        .enumerate()
        .map(|(index, module)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {:>2} ", index + 1),
                    Style::default()
                        .fg(colors.highlight_fg)
                        .bg(colors.border)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(module.label(), Style::default().fg(colors.fg)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(colors.panel(" Módulos · 1-9 acceso rápido "))
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, list_area, &mut app.menu_state);
    let mut hits = Vec::new();
    let filas_visibles = list_area.height.saturating_sub(2) as usize;
    for indice in 0..Module::ALL.len().min(filas_visibles) {
        hits.push((
            Rect {
                x: list_area.x + 1,
                y: list_area.y + 1 + indice as u16,
                width: list_area.width.saturating_sub(2),
                height: 1,
            },
            indice,
        ));
    }
    app.menu_hits = hits;

    let selected = app.selected_module();
    let indice = Module::ALL
        .iter()
        .position(|module| *module == selected)
        .map_or(1, |posicion| posicion + 1);

    let mut lineas = vec![
        Line::from(Span::styled(selected.label(), colors.title_style())),
        Line::from(""),
        Line::from(Span::styled(
            selected.description(),
            Style::default().fg(colors.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Módulo ", colors.muted_style()),
            Span::styled(
                format!("{indice} de {}", Module::ALL.len()),
                Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   ·   tema ", colors.muted_style()),
            Span::styled(
                app.settings.theme.label(),
                Style::default().fg(colors.accent),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Atajos",
            Style::default()
                .fg(colors.muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ↑↓/jk  mover      Enter  abrir      ?  ayuda",
            Style::default().fg(colors.fg),
        )),
        Line::from(Span::styled(
            "  1-9   ir directo    q  salir",
            Style::default().fg(colors.fg),
        )),
        Line::from(Span::styled(
            if app.tutorial_completado {
                "  c calendario · Ctrl+K paleta · / buscar"
            } else {
                "  t: tutorial de inicio"
            },
            Style::default().fg(colors.accent),
        )),
    ];
    if area.width < 60 {
        lineas.retain(|linea| linea.width() <= area.width as usize);
    }

    let bloque = colors.panel(" Vista previa ");
    let interior = bloque.inner(detail_area);
    frame.render_widget(bloque, detail_area);
    let con_logo = interior.height >= 18 && interior.width >= 30;
    let alto_logo = if con_logo { 9 } else { 0 };
    let [texto_area, logo_area] =
        Layout::vertical([Constraint::Min(6), Constraint::Length(alto_logo)]).areas(interior);
    frame.render_widget(Paragraph::new(lineas).wrap(Wrap { trim: true }), texto_area);
    if con_logo {
        crate::ui::images::dibujar_logo(frame, logo_area, &mut app.images, &colors);
    }
}

fn draw_hoy(frame: &mut Frame, area: Rect, app: &App, colors: &crate::ui::theme::Palette) {
    let tarjetas = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
    ])
    .spacing(1)
    .split(area);

    let proximo = app.inicio.proximo.as_ref().map_or_else(
        || "Sin partido próximo".to_string(),
        |partido| {
            format!(
                "{} · {}\n{} · {}",
                partido.fecha_hora.format("%d/%m %H:%M"),
                if partido.rival.trim().is_empty() {
                    "Rival a confirmar"
                } else {
                    &partido.rival
                },
                partido.resultado.label(),
                if partido.goles + partido.asistencias == 0 {
                    "sin aportes".to_string()
                } else {
                    format!("{} G · {} A", partido.goles, partido.asistencias)
                }
            )
        },
    );
    let racha = app.inicio.racha.tipo.label();
    let racha_texto = if app.inicio.racha.cantidad > 0 {
        format!("{} · {} partidos", racha, app.inicio.racha.cantidad)
    } else {
        "Sin racha registrada".to_string()
    };
    let mes = &app.inicio.mes;
    let mes_texto = format!(
        "{} PJ · {} G · {} A\n{}V {}E {}D · {:.1} rend.",
        mes.partidos,
        mes.goles,
        mes.asistencias,
        mes.victorias,
        mes.empates,
        mes.derrotas,
        mes.rendimiento_promedio
    );
    let alertas = match (app.inicio.lesiones_activas, &app.inicio.presupuesto_alerta) {
        (0, None) => "Sin alertas".to_string(),
        (lesiones, None) => format!("{lesiones} lesión(es) activa(s)"),
        (0, Some(alerta)) => alerta.clone(),
        (lesiones, Some(alerta)) => format!("{lesiones} lesión(es)\n{alerta}"),
    };

    let tarjetas_texto = [
        (" Próximo partido ", proximo),
        (" Racha ", racha_texto),
        (" Este mes ", mes_texto),
        (" Alertas ", alertas),
    ];
    for (indice, (titulo, texto)) in tarjetas_texto.into_iter().enumerate() {
        let bloque = colors
            .panel(titulo)
            .border_style(Style::default().fg(colors.border));
        frame.render_widget(
            Paragraph::new(texto)
                .block(bloque)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(colors.fg)),
            tarjetas[indice],
        );
    }
}

fn draw_titulo(frame: &mut Frame, area: Rect, colors: &crate::ui::theme::Palette) {
    if area.width >= 64 {
        let [marca, lema] =
            Layout::horizontal([Constraint::Length(82), Constraint::Min(0)]).areas(area);
        let titulo = BigText::builder()
            .pixel_size(PixelSize::HalfHeight)
            .lines(vec![Line::from("MiFutbolRS")])
            .style(colors.title_style())
            .build();
        frame.render_widget(titulo, marca);
        let texto_lema = Paragraph::new(vec![
            Line::from(Span::styled(
                "Gestión futbolística",
                Style::default().fg(colors.fg),
            )),
            Line::from(Span::styled("en terminal", Style::default().fg(colors.fg))),
            Line::from(Span::styled("Rust · SQLite · v0.1", colors.muted_style())),
        ])
        .alignment(Alignment::Right);
        frame.render_widget(texto_lema, lema);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("MiFutbolRS", colors.title_style()))),
            area,
        );
    }
}
