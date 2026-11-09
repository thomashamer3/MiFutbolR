//! Tutorial de inicio (IMP-09).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

const PASOS: [(&str, &str); 4] = [
    (
        "Bienvenido a MiFutbolR",
        "Esta TUI guarda tu historia futbolística en SQLite.\n\nPodés moverte con flechas o j/k, abrir con Enter y volver con Esc.\n\nEl panel de inicio muestra tu próximo partido, la racha, el mes y las alertas.",
    ),
    (
        "Cargá tus recursos",
        "Antes del primer partido necesitás una cancha y una camiseta.\n\n1. Camisetas: n para crear.\n2. Canchas: n para crear (con latitud/longitud para el clima).\n3. Botines: opcional, podés fijar uno predeterminado.\n4. Equipos: plantel con dorsales únicos.",
    ),
    (
        "Registrá un partido",
        "En Partidos, n abre el asistente de 4 pasos.\n\nCompletá fecha, cancha, camiseta, resultado y métricas. Al guardar se crea el movimiento financiero de alquiler y se actualizan las estadísticas.",
    ),
    (
        "Explorá y compartí",
        "c abre el calendario; ! las notificaciones; l los logros; Ctrl+K la paleta de comandos; Ctrl+F la búsqueda global.\n\nx exporta el listado actual a CSV, JSON, HTML y Markdown.\n\nEl Resumen Compartible genera un informe listo para compartir.",
    ),
];

/// Dibuja el tutorial.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let paso = app.tutorial_paso.min(PASOS.len() - 1);
    let (titulo, texto) = PASOS[paso];

    let [cuerpo, pie] = Layout::vertical([Constraint::Min(6), Constraint::Length(2)]).areas(area);
    let bloque = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" Tutorial · paso {}/{} ", paso + 1, PASOS.len()))
        .border_style(Style::default().fg(colors.accent))
        .style(Style::default().bg(colors.bg));
    let lineas = vec![
        Line::from(Span::styled(
            titulo,
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(texto, Style::default().fg(colors.fg))),
    ];
    frame.render_widget(
        Paragraph::new(lineas)
            .block(bloque)
            .wrap(Wrap { trim: false }),
        cuerpo,
    );

    let progreso: String = (0..PASOS.len())
        .map(|indice| if indice == paso { "●" } else { "○" })
        .collect::<Vec<_>>()
        .join(" ");
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {progreso} "), Style::default().fg(colors.accent)),
            Span::styled(
                "  ←/→: paso   Enter: siguiente   Esc: salir",
                Style::default().fg(colors.muted),
            ),
        ])),
        pie,
    );
}
