//! Entrenador IA (paridad con MiFutbolC).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::App;
use crate::domain::entrenador::Severidad;
use crate::ui::theme::palette;

const NIVELES: [&str; 4] = ["desactivado", "bajo", "medio", "alto"];

/// Pantalla del entrenador.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(a) = &app.entrenador else {
        return;
    };
    let e = &a.estado;
    let titulo = |texto: &str| {
        Line::styled(
            texto.to_string(),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let dato = |etiqueta: &str, valor: String| {
        Line::from(vec![
            Span::styled(format!("{etiqueta:<20}"), Style::default().fg(colors.muted)),
            Span::raw(valor),
        ])
    };

    let mut estado = vec![titulo("Estado del jugador")];
    if e.partidos == 0 {
        estado.push(Line::raw(
            "Sin partidos cargados: el análisis usa valores neutros.",
        ));
    } else {
        estado.extend([
            dato("Partidos analizados", e.partidos.to_string()),
            dato("Rendimiento", format!("{:.1}/10", e.rendimiento)),
            dato("Cansancio", format!("{:.1}/10", e.cansancio)),
            dato("Ánimo", format!("{:.1}/10", e.animo)),
            dato("Días de descanso", e.dias_descanso.to_string()),
            dato("Partidos en 7 días", e.consecutivos.to_string()),
            dato("Racha de derrotas", e.derrotas.to_string()),
            dato("Riesgo de lesión", format!("{:.2}", e.riesgo)),
        ]);
    }
    estado.push(Line::default());
    estado.push(titulo("Consejos"));
    for consejo in &a.consejos {
        let color = match consejo.severidad {
            Severidad::Info => colors.success,
            Severidad::Advertencia => colors.warning,
            Severidad::Critico => colors.error,
        };
        estado.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", consejo.severidad.label()),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("{}: ", consejo.area),
                Style::default().fg(colors.muted),
            ),
            Span::raw(consejo.texto),
        ]));
    }
    estado.push(Line::default());
    estado.push(dato(
        "Historial",
        format!("{} seguidos · {} ignorados", a.historial.0, a.historial.1),
    ));
    estado.push(dato(
        "Intervención",
        format!(
            "{} ({})",
            NIVELES[usize::try_from(a.nivel).unwrap_or(0).min(3)],
            if a.alerta_previa {
                "se alertaría antes del próximo partido"
            } else {
                "sin alerta previa"
            }
        ),
    ));

    let (victoria, empate, derrota) = a.prediccion;
    let mut tactica = vec![
        titulo("Predicción del próximo partido"),
        dato("Victoria", format!("{victoria:.0}%")),
        dato("Empate", format!("{empate:.0}%")),
        dato("Derrota", format!("{derrota:.0}%")),
        Line::default(),
        titulo("Táctica"),
        dato("Formación", a.formacion.to_string()),
    ];
    tactica.extend(a.tacticas.iter().map(|t| Line::raw(format!("• {t}"))));
    tactica.push(Line::default());
    tactica.push(titulo("Descanso"));
    tactica.push(dato(
        "Sugerido",
        format!("{} días ({})", a.descanso.0, a.descanso.1),
    ));
    if let Some(nota) = a.nota_descanso {
        tactica.push(Line::styled(nota, Style::default().fg(colors.error)));
    }
    tactica.push(Line::default());
    tactica.push(titulo("Puntos débiles"));
    if a.puntos_debiles.is_empty() {
        tactica.push(Line::raw("No se detectaron puntos débiles."));
    }
    tactica.extend(a.puntos_debiles.iter().map(|p| Line::raw(format!("• {p}"))));

    let [izquierda, derecha] = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area)
    } else {
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area)
    };
    frame.render_widget(
        Paragraph::new(estado)
            .block(colors.panel(" Entrenador IA "))
            .wrap(Wrap { trim: true }),
        izquierda,
    );
    frame.render_widget(
        Paragraph::new(tactica)
            .block(colors.panel(" Plan "))
            .wrap(Wrap { trim: true }),
        derecha,
    );
}
