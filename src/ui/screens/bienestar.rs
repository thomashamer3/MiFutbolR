//! Bienestar (paridad con MiFutbolC).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Wrap};

use crate::app::App;
use crate::domain::bienestar::{Seccion, TipoDato};
use crate::ui::theme::palette;
use crate::ui::widgets::{form, lista};

/// Listado con pestañas por sección.
pub fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    // 13 pestañas no entran en una línea: se reparten en dos.
    let [pestanas, cuerpo] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(3)]).areas(area);
    let mut spans = Vec::new();
    for seccion in Seccion::ALL {
        if !spans.is_empty() {
            spans.push(Span::styled(" │ ", Style::default().fg(colors.muted)));
        }
        spans.push(if seccion == app.bienestar_seccion {
            Span::styled(
                seccion.label(),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(seccion.label(), Style::default().fg(colors.muted))
        });
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true }),
        pestanas,
    );

    let seccion = app.bienestar_seccion;
    if app.bienestar_registros.is_empty() {
        lista::render_vacio(
            frame,
            cuerpo,
            &colors,
            format!(" {} ", seccion.label()),
            "Todavía no hay registros. Presioná n para crear uno.",
        );
        return;
    }
    let columnas = &seccion.columnas()[..seccion.columnas_listado()];
    let filas: Vec<Row> = app
        .bienestar_registros
        .iter()
        .map(|registro| {
            Row::new(
                registro
                    .vista
                    .iter()
                    .take(columnas.len())
                    .map(|valor| Cell::from(valor.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let cabeceras = columnas
        .iter()
        .map(|columna| {
            columna
                .etiqueta
                .split(" (")
                .next()
                .unwrap_or(columna.etiqueta)
        })
        .collect();
    let anchos = columnas
        .iter()
        .map(|columna| match columna.clave {
            _ if matches!(columna.tipo, TipoDato::Referencia(_)) => Constraint::Min(22),
            "fecha" | "ultima_revision" | "fecha_inicio" | "fecha_fin" | "fecha_subida" => {
                Constraint::Length(11)
            }
            "notas" | "descripcion" | "estado_animico" | "nombre" | "rutina_semanal"
            | "nombre_original" | "ruta_archivo" | "imagen_ruta" | "resultado" => {
                Constraint::Min(16)
            }
            _ => Constraint::Min(8),
        })
        .collect();
    lista::render_tabla(
        frame,
        cuerpo,
        &colors,
        format!(" {} · {} ", seccion.label(), app.bienestar_registros.len()),
        cabeceras,
        anchos,
        filas,
        &mut app.bienestar_table,
    );
}

/// Asistente de preparación.
pub fn draw_asistente(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let Some(asistente) = &app.bienestar_asistente else {
        return;
    };
    let etiqueta =
        |texto: &str| Span::styled(format!("{texto:<16}"), Style::default().fg(colors.muted));
    let nivel = ["Baja", "Moderada", "Alta"][usize::from(asistente.nivel.min(2))];
    let riesgo_color = if asistente.riesgo >= 70 {
        colors.error
    } else if asistente.riesgo >= 40 {
        colors.warning
    } else {
        colors.success
    };
    let mut lineas = vec![
        Line::from(vec![
            etiqueta("Preparación"),
            Span::styled(
                format!("{}/100", asistente.preparacion),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            etiqueta("Riesgo de lesión"),
            Span::styled(
                format!("{}/100", asistente.riesgo),
                Style::default().fg(riesgo_color),
            ),
        ]),
        Line::from(vec![etiqueta("Carga sugerida"), Span::raw(nivel)]),
        Line::from(vec![
            etiqueta("Parciales"),
            Span::raw(format!(
                "Rend {} · Hábitos {} · Nutrición {} · Mental {} · Entreno {}",
                asistente.parciales[0],
                asistente.parciales[1],
                asistente.parciales[2],
                asistente.parciales[3],
                asistente.parciales[4]
            )),
        ]),
        Line::default(),
    ];
    if let Some(alerta) = &asistente.alerta_intensidad {
        lineas.push(Line::styled(
            alerta.clone(),
            Style::default().fg(colors.warning),
        ));
        lineas.push(Line::default());
    }
    lineas.extend(
        asistente
            .rutina
            .lines()
            .map(|linea| Line::raw(linea.to_string())),
    );
    if !app.bienestar_recomendaciones.is_empty() {
        lineas.push(Line::default());
        lineas.push(Line::styled(
            "Recomendaciones guardadas",
            Style::default().fg(colors.muted),
        ));
        for recomendacion in &app.bienestar_recomendaciones {
            lineas.push(Line::raw(format!(
                "{}  {}",
                recomendacion.fecha, recomendacion.resumen
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(" Asistente de entrenamientos "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Formulario.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
