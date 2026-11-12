//! Pantalla de bienvenida de MiFutbolR.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use tui_big_text::{BigText, PixelSize};

use crate::app::App;
use crate::ui::theme::palette;

/// Dibuja la bienvenida con las opciones de cuenta.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let [marca_area, cuerpo, pie] = Layout::vertical([
        Constraint::Length(if area.height >= 24 { 9 } else { 5 }),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .areas(area);

    let titulo = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .lines(vec![Line::from("MiFutbolR")])
        .style(colors.title_style())
        .build();
    frame.render_widget(titulo, marca_area);
    let lema = Paragraph::new("Gestión futbolística personal · Rust + SQLite")
        .alignment(Alignment::Center)
        .style(Style::default().fg(colors.muted));
    frame.render_widget(
        lema,
        Rect {
            y: marca_area.y + marca_area.height.saturating_sub(1),
            height: 1,
            ..marca_area
        },
    );

    let mut lineas = Vec::new();
    if let Some(usuario) = &app.usuario_actual {
        lineas.push(Line::from(Span::styled(
            format!("Hola, {}", usuario.nombre),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        )));
        lineas.push(Line::from(""));
        lineas.push(Line::from(Span::styled(
            "Tu cuenta está lista. Enter para entrar al menú.",
            Style::default().fg(colors.fg),
        )));
    } else {
        lineas.push(Line::from(Span::styled(
            "Creá una cuenta o entrá sin contraseña.",
            Style::default().fg(colors.fg),
        )));
        lineas.push(Line::from(""));
        for (tecla, texto) in [
            ("n", "Crear una cuenta nueva"),
            ("l", "Iniciar sesión con una cuenta existente"),
            ("Enter", "Continuar sin cuenta"),
        ] {
            lineas.push(Line::from(vec![
                Span::styled(
                    format!("  {tecla:>5}  "),
                    Style::default()
                        .fg(colors.highlight_fg)
                        .bg(colors.border)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(texto, Style::default().fg(colors.fg)),
            ]));
        }
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(
        "Cada cuenta guarda su propia sesión local. Las contraseñas son opcionales.",
        Style::default().fg(colors.muted),
    )));
    frame.render_widget(
        Paragraph::new(lineas)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Bienvenida ")
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        cuerpo,
    );
    frame.render_widget(
        Paragraph::new("n: crear cuenta · l: iniciar sesión · Enter: continuar · q: salir")
            .alignment(Alignment::Center)
            .style(Style::default().fg(colors.muted)),
        pie,
    );
}
