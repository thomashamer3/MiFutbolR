//! Pantalla de Ajustes (§18.7).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::ui::theme::palette;
use crate::ui::widgets::form;

/// Dibuja el formulario de ajustes con una muestra del tema.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mostrar_preview = app
        .form
        .as_ref()
        .is_some_and(|formulario| formulario.title == "Ajustes");
    let (form_area, preview_area) = if mostrar_preview && area.height > 12 {
        let [formulario, preview] =
            Layout::vertical([Constraint::Min(6), Constraint::Length(3)]).areas(area);
        (formulario, Some(preview))
    } else {
        (area, None)
    };

    if let Some(formulario) = &app.form {
        form::draw(frame, form_area, app, formulario);
    }

    if let Some(preview) = preview_area {
        let chips = vec![
            Span::styled(" Normal ", Style::default().fg(colors.bg).bg(colors.fg)),
            Span::raw(" "),
            Span::styled(
                " Acento ",
                Style::default()
                    .fg(colors.bg)
                    .bg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(" Éxito ", Style::default().fg(colors.bg).bg(colors.success)),
            Span::raw(" "),
            Span::styled(" Aviso ", Style::default().fg(colors.bg).bg(colors.warning)),
            Span::raw(" "),
            Span::styled(" Error ", Style::default().fg(colors.bg).bg(colors.error)),
            Span::raw(" "),
            Span::styled(
                " Selección ",
                Style::default()
                    .fg(colors.highlight_fg)
                    .bg(colors.highlight_bg),
            ),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(chips)).block(
                Block::bordered()
                    .title(" Vista previa del tema ")
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.bg)),
            ),
            preview,
        );
    }
}
