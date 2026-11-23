//! Ayuda contextual con los atajos globales (§19.4).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::App;
use crate::ui::theme::palette;

/// Atajos documentados.
const ATAJOS: &[(&str, &str)] = &[
    ("↑/↓ o k/j", "Mover la selección"),
    ("Enter", "Abrir / editar / confirmar"),
    ("Esc", "Volver / cancelar"),
    ("Ctrl+S", "Guardar formulario"),
    ("Tab / Shift+Tab", "Campo siguiente / anterior"),
    ("n / e / d", "Nuevo / editar / borrar"),
    ("/", "Buscar en el listado"),
    ("f / o", "Filtros / ordenar"),
    ("r", "Refrescar"),
    ("x", "Exportar listado (CSV/JSON/HTML/MD)"),
    ("Ctrl+K / Ctrl+F", "Paleta / búsqueda global"),
    ("g / !", "Inicio / avisos"),
    ("Ctrl+Z", "Deshacer"),
    ("Espacio", "Marcar / desmarcar"),
    ("?", "Esta ayuda"),
    (
        "q / Esc",
        "Volver; en el menú y la portada, salir (pide confirmar)",
    ),
];

/// Atajos que solo valen en el menú principal.
const ATAJOS_MENU: &[(&str, &str)] = &[
    (
        "1 … 28",
        "Abrir la opción (\"15\" abre la 15; Enter abre lo tecleado)",
    ),
    ("0", "Salir"),
    ("c / l / t", "Calendario / logros / tutorial"),
    ("n", "Nuevo partido"),
    ("R / T / U", "Récords / torneos / últimos"),
    ("C / E", "Comparador A/B / temporadas"),
    ("D / A / M", "Tiendas / amigos / multimedia"),
    ("Y / P / L", "Recordatorios / progresión / logros"),
];

/// Atajos propios de algunos módulos.
const ATAJOS_MODULOS: &[(&str, &str)] = &[
    (
        "a (Camisetas)",
        "Activar / reactivar la camiseta (borrar con historial la desactiva)",
    ),
    (
        "Ctrl+G (Ajustes)",
        "Backups: Enter restaura, d elimina, b crea uno nuevo",
    ),
    (
        "Ctrl+T (Ajustes)",
        "Mantenimiento: backup, VACUUM, ANALYZE e integridad",
    ),
    (
        "Ctrl+D (Ajustes)",
        "Exportar / Importar: todo, XLSX, ODS, PDF, base, móvil, legacy e importar JSON/TXT/CSV/HTML",
    ),
    (
        "Ctrl+X (Ajustes)",
        "Exportar todo: tablas a JSON y listados a Markdown",
    ),
    (
        "Ctrl+W / Ctrl+Y (Ajustes)",
        "MiFutbolMobile: exportar mifutbolmobile_import.json / importar partidos.json",
    ),
    (
        "Ctrl+A (Ajustes)",
        "Importar partidos.json del móvil al iniciar (activar / desactivar)",
    ),
    (
        "1-8 (Sincronización)",
        "PocketBase: servidor, sesión, sincronizar, subir todo, estado, conflictos, auto-sync, salir",
    ),
    (
        "m (Evolución)",
        "Análisis → Evolución temporal: goles / asistencias / rendimiento",
    ),
];

/// Dibuja la pantalla de ayuda.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let colors = palette(app.settings.theme);
    let mut lines = vec![Line::from(Span::styled(
        "Atajos globales",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    ))];
    let fila = |(key, action): &(&str, &'static str)| {
        Line::from(vec![
            Span::styled(format!("  {key:<16}"), Style::default().fg(colors.warning)),
            Span::styled(*action, Style::default().fg(colors.fg)),
        ])
    };
    lines.extend(ATAJOS.iter().map(fila));
    lines.push(Line::from(Span::styled(
        "Menú principal",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.extend(ATAJOS_MENU.iter().map(fila));
    lines.push(Line::from(Span::styled(
        "Módulos",
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.extend(ATAJOS_MODULOS.iter().map(fila));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Esc: volver al menú",
        Style::default().fg(colors.muted),
    )));

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Ayuda ")
        .border_style(Style::default().fg(colors.border))
        .style(Style::default().bg(colors.bg));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
