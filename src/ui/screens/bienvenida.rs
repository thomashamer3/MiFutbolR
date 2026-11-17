//! Portada de MiFutbolR: inicio de sesión y registro a pantalla completa.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};
use tui_big_text::{BigText, PixelSize};

use crate::app::App;
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::form;

/// Ancho en celdas de "MiFutbolR" (9 letras de 8 columnas a tamaño completo).
const LETRAS: u16 = 9;

/// Dibuja la portada; si hay un formulario de cuenta abierto, lo muestra en la tarjeta.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = palette(app.settings.theme);
    let (pixel, alto_titulo) = tamano_titulo(area);
    let con_cancha = area.height >= 22;
    let alto_tarjeta = match &app.form {
        Some(formulario) => formulario.fields.len() as u16 + 6,
        None => (app.usuarios.len().max(1) as u16)
            .saturating_add(6)
            .clamp(8, 16),
    };
    let ancho_tarjeta = area.width.saturating_sub(4).min(68);

    let [titulo_area, cancha_area, lema_area, _, tarjeta_area] = Layout::vertical([
        Constraint::Length(alto_titulo),
        Constraint::Length(if con_cancha { 3 } else { 0 }),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(alto_tarjeta),
    ])
    .flex(Flex::Center)
    .areas(area);

    match pixel {
        Some(pixel) => frame.render_widget(
            BigText::builder()
                .pixel_size(pixel)
                .lines(vec![Line::from("MiFutbolR")])
                .style(colors.title_style())
                .centered()
                .build(),
            titulo_area,
        ),
        None => frame.render_widget(
            Paragraph::new("⚽ MiFutbolR")
                .alignment(Alignment::Center)
                .style(colors.title_style()),
            titulo_area,
        ),
    }
    if con_cancha {
        frame.render_widget(cancha(&colors), cancha_area);
    }
    frame.render_widget(
        Paragraph::new("Gestión futbolística personal · cada cuenta con su propia base")
            .alignment(Alignment::Center)
            .style(colors.muted_style()),
        lema_area,
    );

    let tarjeta = tarjeta_area.centered_horizontally(Constraint::Length(ancho_tarjeta));
    if let Some(formulario) = &app.form {
        form::draw(frame, tarjeta, app, formulario);
    } else {
        draw_cuentas(frame, tarjeta, app, &colors);
    }
}

/// Elige el tamaño del título según el espacio disponible.
fn tamano_titulo(area: Rect) -> (Option<PixelSize>, u16) {
    if area.height >= 30 && area.width >= LETRAS * 8 + 4 {
        (Some(PixelSize::Full), 8)
    } else if area.height >= 20 && area.width >= LETRAS * 8 + 4 {
        (Some(PixelSize::HalfHeight), 4)
    } else if area.height >= 16 && area.width >= LETRAS * 4 + 4 {
        (Some(PixelSize::Quadrant), 4)
    } else {
        (None, 1)
    }
}

/// Cancha estilizada como decoración.
fn cancha(colors: &Palette) -> Paragraph<'static> {
    let linea = Style::default().fg(colors.success);
    let pelota = Style::default()
        .fg(colors.accent)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(vec![
        Line::styled("╭─────┬──────────┬──────────┬─────╮", linea),
        Line::from(vec![
            Span::styled("│▌    │          ", linea),
            Span::styled("○", pelota),
            Span::styled("          │    ▐│", linea),
        ]),
        Line::styled("╰─────┴──────────┴──────────┴─────╯", linea),
    ])
    .alignment(Alignment::Center)
}

fn draw_cuentas(frame: &mut Frame, area: Rect, app: &mut App, colors: &Palette) {
    let bloque = colors.panel(Line::styled(" Iniciar sesión ", colors.title_style()));
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let [lista_area, ayuda_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(interior);

    if app.usuarios.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(
                    "Todavía no hay cuentas. Presioná n para crear la primera.",
                    Style::default().fg(colors.fg),
                ),
                Line::styled("o i para entrar como invitado.", colors.muted_style()),
            ])
            .alignment(Alignment::Center),
            lista_area,
        );
    } else {
        let filas = app.usuarios.iter().map(|usuario| {
            Row::new(vec![
                Cell::from(usuario.nombre.clone()),
                Cell::from(if usuario.con_password {
                    "🔒 con contraseña"
                } else {
                    "   sin contraseña"
                }),
                Cell::from(if usuario.ultimo_acceso.is_empty() {
                    "nunca".to_string()
                } else {
                    usuario.ultimo_acceso.clone()
                }),
            ])
            .style(Style::default().fg(colors.fg))
        });
        let tabla = Table::new(
            filas,
            [
                Constraint::Min(12),
                Constraint::Length(18),
                Constraint::Length(19),
            ],
        )
        .header(Row::new(vec!["Cuenta", "Acceso", "Último ingreso"]).style(colors.muted_style()))
        .row_highlight_style(
            Style::default()
                .fg(colors.highlight_fg)
                .bg(colors.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
        frame.render_stateful_widget(tabla, lista_area, &mut app.usuarios_table);
    }

    let tecla = Style::default()
        .fg(colors.accent)
        .add_modifier(Modifier::BOLD);
    let texto = colors.muted_style();
    let mut atajos = Vec::new();
    for (k, t) in [
        ("Enter", "entrar"),
        ("n", "crear"),
        ("l", "ingresar"),
        ("i", "invitado"),
        ("q", "salir"),
    ] {
        if !atajos.is_empty() {
            atajos.push(Span::styled(" · ", texto));
        }
        atajos.push(Span::styled(k, tecla));
        atajos.push(Span::styled(format!(" {t}"), texto));
    }
    frame.render_widget(
        Paragraph::new(vec![Line::from(""), Line::from(atajos)]).alignment(Alignment::Center),
        ayuda_area,
    );
}
