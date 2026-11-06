//! Renderizado de la TUI. Solo representa el estado de `App`; nunca consulta
//! la base de datos ni ejecuta reglas de negocio (§Performance.md §3).

pub mod screens;
pub mod theme;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::app::{APP_NAME, App, Modal, Screen};
use crate::util::text::truncate_width;
use theme::{Palette, palette};

/// Dibuja el frame completo.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let colors = palette(app.settings.theme);
    let area = frame.area();

    // Fondo uniforme del tema.
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.bg).fg(colors.fg)),
        area,
    );

    let [header, separador, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_header(frame, header, app, &colors);
    draw_rule(frame, separador, &colors);

    match app.current_screen() {
        Screen::MainMenu => screens::main_menu::draw(frame, body, app),
        Screen::Module(module) => screens::module_placeholder(frame, body, app, module),
        Screen::CamisetasList => screens::camisetas::draw_list(frame, body, app),
        Screen::CamisetaDetail(id) => screens::camisetas::draw_detail(frame, body, app, id),
        Screen::CamisetaForm(_) => screens::camisetas::draw_form(frame, body, app),
        Screen::CanchasList => screens::canchas::draw_list(frame, body, app),
        Screen::CanchaDetail(id) => screens::canchas::draw_detail(frame, body, app, id),
        Screen::CanchaForm(_) => screens::canchas::draw_form(frame, body, app),
        Screen::BotinesList => screens::botines::draw_list(frame, body, app),
        Screen::BotinDetail(id) => screens::botines::draw_detail(frame, body, app, id),
        Screen::BotinForm(_) => screens::botines::draw_form(frame, body, app),
        Screen::EquiposList => screens::equipos::draw_list(frame, body, app),
        Screen::EquipoDetail(id) => screens::equipos::draw_detail(frame, body, app, id),
        Screen::EquipoForm(_) | Screen::JugadorForm(_) => {
            screens::equipos::draw_form(frame, body, app)
        }
        Screen::PartidosList => screens::partidos::draw_list(frame, body, app),
        Screen::PartidoDetail(id) => screens::partidos::draw_detail(frame, body, app, id),
        Screen::PartidoForm(_) => screens::partidos::draw_form(frame, body, app),
        Screen::PartidoWizard => screens::partidos::draw_wizard(frame, body, app),
        Screen::LesionesList => screens::lesiones::draw_list(frame, body, app),
        Screen::LesionDetail(id) => screens::lesiones::draw_detail(frame, body, app, id),
        Screen::LesionForm(_) => screens::lesiones::draw_form(frame, body, app),
        Screen::EstadisticasHub => screens::estadisticas::draw_hub(frame, body, app),
        Screen::EstadisticasDetalle(kind) => {
            screens::estadisticas::draw_detail(frame, body, app, kind);
        }
        Screen::FinanciamientoList => screens::financiamiento::draw_list(frame, body, app),
        Screen::TransaccionForm(_) | Screen::PresupuestoForm => {
            screens::financiamiento::draw_form(frame, body, app);
        }
        Screen::CarreraMenu => screens::carrera::draw_menu(frame, body, app),
        Screen::CarreraResumen => screens::carrera::draw_resumen(frame, body, app),
        Screen::CarreraHitos => screens::carrera::draw_hitos(frame, body, app),
        Screen::CarreraTimeline => screens::carrera::draw_timeline(frame, body, app),
        Screen::CarreraHallOfFame => screens::carrera::draw_hall(frame, body, app),
        Screen::CarreraPerfil => screens::carrera::draw_perfil(frame, body, app),
        Screen::CarreraRetro => screens::carrera::draw_retro(frame, body, app),
        Screen::CarreraLogros => screens::carrera::draw_logros(frame, body, app),
        Screen::CarreraIdentidadForm | Screen::CarreraHitoForm(_) => {
            screens::carrera::draw_form(frame, body, app);
        }
        Screen::ResumenPreview => screens::resumen::draw(frame, body, app),
        Screen::AjustesForm | Screen::LegacyImportForm => {
            screens::ajustes::draw(frame, body, app);
        }
        Screen::Ayuda => screens::ayuda::draw(frame, body, app),
    }

    draw_footer(frame, footer, app, &colors);

    if let Some(modal) = app.modal.clone() {
        draw_modal(frame, area, &modal, &colors);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, colors: &Palette) {
    let db_label = match app.db.path() {
        Some(path) => path.display().to_string(),
        None => "en memoria".to_string(),
    };
    let [izquierda, derecha] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(30)]).areas(area);

    let titulo = truncate_width(&app.screen_title(), area.width as usize / 2);
    let lineas_izquierda = vec![
        Line::from(vec![
            Span::styled(
                format!(" {APP_NAME} "),
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " › ",
                Style::default()
                    .fg(colors.muted)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                titulo,
                Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!(" BD: {db_label}"),
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(Paragraph::new(lineas_izquierda), izquierda);

    let modo = format!(
        " {} · {} ",
        app.settings.theme.label(),
        app.settings.mode.label()
    );
    let lineas_derecha = vec![
        Line::from(Span::styled(
            truncate_width(&modo, 30),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.border)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ? Ayuda · q Salir",
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lineas_derecha).alignment(Alignment::Right),
        derecha,
    );
}

fn draw_rule(frame: &mut Frame, area: Rect, colors: &Palette) {
    let ancho = area.width as usize;
    if ancho == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(ancho),
            Style::default().fg(colors.border),
        ))),
        area,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App, colors: &Palette) {
    let (texto_estado, color_estado) = match &app.search {
        Some(search) => (format!(" Buscar: {}▌ ", search.buffer), colors.warning),
        None => (format!(" {} ", app.status), colors.highlight_bg),
    };
    let ancho_estado = texto_estado.chars().count() + 2;
    let disponible = (area.width as usize).saturating_sub(ancho_estado);
    let pistas = truncate_width(app.footer_hints(), disponible);
    let line = Line::from(vec![
        Span::styled(
            texto_estado.clone(),
            Style::default()
                .fg(colors.highlight_fg)
                .bg(color_estado)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {pistas}"), Style::default().fg(colors.muted)),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(colors.bg)),
        area,
    );
}

fn draw_modal(frame: &mut Frame, area: Rect, modal: &Modal, colors: &Palette) {
    let (titulo, mensaje, borde, icono) = match modal {
        Modal::Confirm { title, message, .. } => {
            (title.as_str(), message.as_str(), colors.warning, "?")
        }
        Modal::Error(message) => ("Error", message.as_str(), colors.error, "!"),
        Modal::Info(message) => ("Información", message.as_str(), colors.accent, "i"),
    };

    let largo = mensaje
        .lines()
        .map(|linea| linea.chars().count())
        .max()
        .unwrap_or(24);
    let ancho = (largo as u16 + 8).clamp(36, area.width.saturating_sub(4));
    let alto = (mensaje.lines().count() as u16 + 6).clamp(7, area.height.saturating_sub(2).max(7));
    let popup = Rect {
        x: area.x + area.width.saturating_sub(ancho) / 2,
        y: area.y + area.height.saturating_sub(alto) / 2,
        width: ancho,
        height: alto,
    };
    frame.render_widget(Clear, popup);

    let contenido = vec![
        Line::from(Span::styled(
            mensaje.to_string(),
            Style::default().fg(colors.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter / S: aceptar     Esc / N: cancelar",
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(contenido)
            .block(
                Block::bordered()
                    .title(format!(" [{icono}] {titulo} "))
                    .border_style(Style::default().fg(borde))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        popup,
    );
}
