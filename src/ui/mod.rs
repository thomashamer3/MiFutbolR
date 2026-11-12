//! Renderizado de la TUI. Solo representa el estado de `App`; nunca consulta
//! la base de datos ni ejecuta reglas de negocio (§Performance.md §3).

pub mod animation;
pub mod images;
pub mod screens;
pub mod theme;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Tabs, Wrap};

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

    app.sidebar_hits.clear();
    app.menu_hits.clear();
    let seleccion = table_selection(app);
    if app.animations.enabled && seleccion != app.last_selection {
        if let Some((objetivo, indice)) = seleccion
            && app.table_hit_target == Some(objetivo)
            && let Some((area, _)) = app.table_hits.iter().find(|(_, fila)| *fila == indice)
        {
            app.animations.selection(&colors, *area);
        }
        app.last_selection = seleccion;
    }
    app.table_hits.clear();
    app.table_hit_target = None;
    let en_modulo = app.active_module.is_some()
        && !matches!(app.current_screen(), Screen::MainMenu | Screen::Ayuda);
    let con_sidebar = en_modulo && area.width >= 140;
    let con_tabs = en_modulo && !con_sidebar && area.width >= 70;
    let alto_tabs = u16::from(con_tabs);
    let [header, separador, tabs_area, cuerpo, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(alto_tabs),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_header(frame, header, app, &colors);
    draw_rule(frame, separador, &colors);
    if con_tabs {
        draw_module_tabs(frame, tabs_area, app, &colors);
    }
    let body = if con_sidebar {
        let [rail, contenido] = Layout::horizontal([Constraint::Length(18), Constraint::Min(0)])
            .spacing(1)
            .areas(cuerpo);
        draw_sidebar(frame, rail, app, &colors);
        contenido
    } else {
        cuerpo
    };

    if app.animations.enabled {
        let pantalla = app.current_screen();
        if app.last_screen != Some(pantalla) {
            app.last_screen = Some(pantalla);
            app.animations.fade_in(&colors);
        }
        let modal_abierto = app.modal.is_some();
        if modal_abierto && !app.last_modal_open {
            app.animations.modal_in(&colors);
        }
        app.last_modal_open = modal_abierto;
    }

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
        Screen::Calendario => screens::calendario::draw(frame, body, app),
        Screen::Logros => screens::logros::draw(frame, body, app),
        Screen::Notificaciones => screens::notificaciones::draw(frame, body, app),
        Screen::Tutorial => screens::tutorial::draw(frame, body, app),
        Screen::Records => screens::records::draw(frame, body, app),
        Screen::TorneosList => screens::torneos::draw_list(frame, body, app),
        Screen::TorneoDetail(_) => screens::torneos::draw_detail(frame, body, app),
        Screen::TorneoForm(_) | Screen::TorneoResultadoForm(_) => {
            screens::torneos::draw_form(frame, body, app)
        }
        Screen::Ultimos => screens::ultimos::draw(frame, body, app),
        Screen::Comparador => screens::comparador::draw(frame, body, app),
        Screen::TemporadasList => screens::temporadas::draw_list(frame, body, app),
        Screen::TemporadaDetail(_) => screens::temporadas::draw_detail(frame, body, app),
        Screen::TemporadaForm(_) => screens::temporadas::draw_form(frame, body, app),
        Screen::TiendasList => screens::tiendas::draw_list(frame, body, app),
        Screen::TiendaForm(_) => screens::tiendas::draw_form(frame, body, app),
        Screen::AmigosList => screens::amigos::draw_list(frame, body, app),
        Screen::AmigoForm(_) => screens::amigos::draw_form(frame, body, app),
        Screen::MediaList => screens::media::draw_list(frame, body, app),
        Screen::MediaForm(_) => screens::media::draw_form(frame, body, app),
        Screen::RecordatoriosList => screens::recordatorios::draw_list(frame, body, app),
        Screen::Agenda => screens::recordatorios::draw_agenda(frame, body, app),
        Screen::RecordatorioForm(_) => screens::recordatorios::draw_form(frame, body, app),
        Screen::ProgresionList => screens::progresion::draw_list(frame, body, app),
        Screen::ProgresionForm(_) => screens::progresion::draw_form(frame, body, app),
        Screen::PlanesList => screens::progresion::draw_planes(frame, body, app),
        Screen::PlanForm(_) => screens::progresion::draw_form(frame, body, app),
        Screen::Bienvenida => screens::bienvenida::draw(frame, body, app),
        Screen::UsuarioList => screens::usuario::draw_list(frame, body, app),
        Screen::UsuarioForm(_) | Screen::UsuarioLogin => {
            screens::usuario::draw_form(frame, body, app)
        }
        Screen::AjustesForm | Screen::LegacyImportForm => {
            screens::ajustes::draw(frame, body, app);
        }
        Screen::Ayuda => screens::ayuda::draw(frame, body, app),
        Screen::ImagenViewer => screens::imagen::draw(frame, body, app),
    }

    draw_footer(frame, footer, app, &colors);

    screens::busqueda::draw(frame, area, app, &colors);

    if let Some(modal) = app.modal.clone() {
        draw_modal(frame, area, &modal, &colors);
    }

    app.animations.process(frame.buffer_mut(), area);
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
            if app.no_leidas > 0 {
                format!(" ! {} avisos · ? Ayuda · q Salir", app.no_leidas)
            } else {
                "? Ayuda · q Salir".to_string()
            },
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lineas_derecha).alignment(Alignment::Right),
        derecha,
    );
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &mut App, colors: &Palette) {
    use ratatui::text::Line;
    let Some(activo) = app.active_module else {
        return;
    };
    let bloque = colors.panel(" Módulos ");
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let mut lineas = Vec::new();
    let mut hits = Vec::new();
    for (indice, module) in crate::app::Module::ALL.iter().enumerate() {
        let seleccionado = *module == activo;
        let texto = format!(" {:>2} {:<11}", indice + 1, module.short_label());
        let estilo = if seleccionado {
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg)
        };
        lineas.push(Line::from(Span::styled(texto, estilo)));
        hits.push((
            Rect {
                x: interior.x,
                y: interior.y + indice as u16,
                width: interior.width,
                height: 1,
            },
            *module,
        ));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        "Clic: cambiar",
        Style::default().fg(colors.accent),
    )));
    lineas.push(Line::from(Span::styled(
        "Esc: volver",
        Style::default().fg(colors.muted),
    )));
    frame.render_widget(Paragraph::new(lineas), interior);
    app.sidebar_hits = hits;
}

fn table_selection(app: &App) -> Option<(crate::app::TableHitTarget, usize)> {
    use crate::app::{Screen, TableHitTarget};
    match app.current_screen() {
        Screen::CamisetasList => app
            .camisetas_table
            .selected()
            .map(|indice| (TableHitTarget::Camisetas, indice)),
        Screen::CanchasList => app
            .canchas_table
            .selected()
            .map(|indice| (TableHitTarget::Canchas, indice)),
        Screen::BotinesList => app
            .botines_table
            .selected()
            .map(|indice| (TableHitTarget::Botines, indice)),
        Screen::EquiposList => app
            .equipos_table
            .selected()
            .map(|indice| (TableHitTarget::Equipos, indice)),
        Screen::PartidosList => app
            .partidos_table
            .selected()
            .map(|indice| (TableHitTarget::Partidos, indice)),
        Screen::LesionesList => app
            .lesiones_table
            .selected()
            .map(|indice| (TableHitTarget::Lesiones, indice)),
        Screen::FinanciamientoList => app
            .transacciones_table
            .selected()
            .map(|indice| (TableHitTarget::Financiamiento, indice)),
        _ => None,
    }
}

fn draw_module_tabs(frame: &mut Frame, area: Rect, app: &App, colors: &Palette) {
    let Some(activo) = app.active_module else {
        return;
    };
    let titulos: Vec<ratatui::text::Line> = crate::app::Module::ALL
        .iter()
        .map(|module| ratatui::text::Line::from(module.short_label()))
        .collect();
    let seleccionado = crate::app::Module::ALL
        .iter()
        .position(|module| *module == activo)
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
    frame.render_widget(tabs, area);
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
                    .border_type(BorderType::Rounded)
                    .title(format!(" [{icono}] {titulo} "))
                    .border_style(Style::default().fg(borde))
                    .style(Style::default().bg(colors.bg)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(colors.bg)),
        popup,
    );
}
