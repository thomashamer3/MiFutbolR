//! Render de formularios genéricos (§19.5).
//!
//! Mejoras de TUI: iconos de tipo, campo obligatorio, contador de caracteres,
//! indicador de selección, barra de desplazamiento y pie contextual.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};

use crate::app::{App, Field, FieldKind, Form};
use crate::ui::theme::{Palette, palette};
use crate::ui::widgets::scroll;

/// Dibuja el formulario activo con scroll automático hacia el campo enfocado.
pub fn draw(frame: &mut Frame, area: Rect, app: &App, form: &Form) {
    let colors = palette(app.settings.theme);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(colors.accent))
        .title(format!(" {} ", form.title))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(colors.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Margen interno de una columna para que el contenido no toque el borde.
    let contenido = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };
    let [fields_area, help_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(contenido);

    let visible = fields_area.height as usize;
    let offset = if visible == 0 || form.focus < visible {
        0
    } else {
        form.focus + 1 - visible
    };

    let total = form.fields.len();
    let lineas = form
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| linea_campo(field, index == form.focus, &colors))
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lineas)
            .scroll((offset as u16, 0))
            .style(Style::default().bg(colors.bg)),
        fields_area,
    );

    let barra_area = Rect {
        x: fields_area.right().saturating_sub(1),
        y: fields_area.y,
        width: 1,
        height: fields_area.height,
    };
    scroll::vertical(frame, barra_area, &colors, total, form.focus, visible);

    let ayuda = match (&form.error, form.current()) {
        (Some(error), _) => Line::from(Span::styled(
            format!("[!] {error}"),
            Style::default()
                .fg(colors.error)
                .add_modifier(Modifier::BOLD),
        )),
        (None, Some(field)) => Line::from(vec![
            Span::styled(
                format!("Campo {} de {total}", form.focus + 1),
                Style::default().fg(colors.accent),
            ),
            Span::styled(
                format!("   {}", field.help.clone().unwrap_or_default()),
                Style::default().fg(colors.muted),
            ),
        ]),
        (None, None) => Line::from(""),
    };
    let pistas = Line::from(Span::styled(
        "Tab: siguiente  Shift+Tab: anterior  Ctrl+S: guardar  Esc: cancelar",
        Style::default().fg(colors.muted),
    ));
    frame.render_widget(
        Paragraph::new(vec![ayuda, pistas])
            .block(Block::default().style(Style::default().bg(colors.bg)))
            .wrap(Wrap { trim: true }),
        help_area,
    );
}

fn linea_campo(field: &Field, focused: bool, colors: &Palette) -> Line<'static> {
    let marker = if focused { "▶" } else { " " };
    let icono = kind_icon(&field.kind);
    let etiqueta = format!("{icono} {:<22}", field.label);

    let estilo_etiqueta = if focused {
        Style::default()
            .fg(colors.highlight_fg)
            .bg(colors.highlight_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.muted)
    };
    let estilo_valor = if focused {
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.fg)
    };

    let mut valor = field.display_value();
    if valor.is_empty() {
        valor = if focused {
            "▌".to_string()
        } else {
            String::new()
        };
    }
    if focused {
        match &field.kind {
            FieldKind::Select { .. } => valor.push_str("  ◂ ▸"),
            FieldKind::Bool => valor.push_str("  [Espacio]"),
            _ => {}
        }
    }

    let mut spans = vec![
        Span::styled(
            format!("{marker} "),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(etiqueta, estilo_etiqueta),
        Span::styled(valor, estilo_valor),
    ];
    if field.required {
        spans.push(Span::styled("*", Style::default().fg(colors.warning)));
    }
    let contador = match &field.kind {
        FieldKind::Text { max } | FieldKind::Multiline { max }
            if focused && !field.value.is_empty() =>
        {
            Some(format!("  {}/{max}", field.value.chars().count()))
        }
        _ => None,
    };
    if let Some(contador) = contador {
        spans.push(Span::styled(contador, Style::default().fg(colors.muted)));
    }
    Line::from(spans)
}

fn kind_icon(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text { .. } => "Aa",
        FieldKind::Password { .. } => "••",
        FieldKind::Multiline { .. } => "¶",
        FieldKind::Int { .. } => " #",
        FieldKind::Decimal { .. } => "±",
        FieldKind::Money => "$",
        FieldKind::Date => "D",
        FieldKind::Time => "H",
        FieldKind::DateTime => "DT",
        FieldKind::Bool => "☑",
        FieldKind::Select { .. } => "▾",
        FieldKind::MultiSelect { .. } => "☰",
    }
}

/// Color de acento del formulario (expuesto para tests de contraste).
#[must_use]
pub fn accent(app: &App) -> ratatui::style::Color {
    let Palette { accent, .. } = palette(app.settings.theme);
    accent
}
