//! 25 Sincronización (PocketBase): menú, estado y conflictos.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

use crate::app::App;
use crate::app_sync::SYNC_OPCIONES;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

fn o_guion(texto: &str) -> &str {
    if texto.is_empty() { "-" } else { texto }
}

/// Menú a la izquierda, estado a la derecha. El token nunca se muestra.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let [menu, panel] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
    let items: Vec<ListItem> = SYNC_OPCIONES
        .iter()
        .enumerate()
        .map(|(indice, (titulo, descripcion))| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{} {titulo}", indice + 1),
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("   {descripcion}"),
                    Style::default().fg(colors.muted),
                )),
            ])
        })
        .collect();
    let lista = List::new(items)
        .block(colors.panel(" Sincronización (PocketBase) "))
        .highlight_style(
            Style::default()
                .bg(colors.highlight_bg)
                .fg(colors.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(lista, menu, &mut app.sync_hub);

    let estado = &app.sync_estado;
    let cfg = &estado.config;
    let fila = |etiqueta: &str, valor: String| {
        Line::from(vec![
            Span::styled(format!("{etiqueta:<13}"), Style::default().fg(colors.muted)),
            Span::styled(valor, Style::default().fg(colors.fg)),
        ])
    };
    let mut lineas = vec![
        fila("Local:", o_guion(&cfg.url_local).to_string()),
        fila("Remota:", o_guion(&cfg.url_remota).to_string()),
        fila("Activa:", o_guion(&cfg.servidor_activo).to_string()),
        fila("Usuario:", o_guion(&cfg.email).to_string()),
        fila(
            "Sesión:",
            if cfg.con_sesion() {
                "iniciada"
            } else {
                "sin iniciar"
            }
            .to_string(),
        ),
        fila(
            "Pendientes:",
            format!("{} · Conflictos: {}", estado.pendientes, estado.conflictos),
        ),
        fila(
            "Última sync:",
            if cfg.ultima_sync.is_empty() {
                "nunca".to_string()
            } else {
                cfg.ultima_sync.clone()
            },
        ),
        fila(
            "Auto-sync:",
            if cfg.auto_sync {
                "activada"
            } else {
                "desactivada"
            }
            .to_string(),
        ),
    ];
    if app.sync.ocupado() {
        lineas.push(Line::from(""));
        lineas.push(Line::styled(
            "Sincronizando en segundo plano…",
            Style::default().fg(colors.accent),
        ));
    } else if !app.sync_mensaje.is_empty() {
        lineas.push(Line::from(""));
        lineas.push(Line::styled(
            app.sync_mensaje.clone(),
            Style::default().fg(colors.accent),
        ));
    }
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(" Estado "))
            .wrap(Wrap { trim: true }),
        panel,
    );
}

/// Primer conflicto con las dos versiones.
pub fn draw_conflicto(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let lineas = match &app.sync_estado.conflicto {
        Some((uuid, local, servidor)) => vec![
            Line::from(format!("Partido {uuid}")),
            Line::from(""),
            Line::from(format!("Local:    {local}")),
            Line::from(format!("Servidor: {servidor}")),
            Line::from(""),
            Line::styled(
                "1 = conservar local   2 = conservar servidor   Esc = volver",
                Style::default().fg(colors.accent),
            ),
        ],
        None => vec![Line::from("No hay conflictos pendientes.")],
    };
    frame.render_widget(
        Paragraph::new(lineas)
            .block(colors.panel(format!(
                " Conflicto · {} pendiente(s) ",
                app.sync_estado.conflictos
            )))
            .wrap(Wrap { trim: true }),
        area,
    );
}

/// Formularios de servidor e inicio de sesión.
pub fn draw_form(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(formulario) = &app.form {
        form::draw(frame, area, app, formulario);
    }
}
