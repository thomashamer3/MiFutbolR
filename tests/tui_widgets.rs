//! Verificación de los widgets de TUI mejorados (vista previa, Gauge,
//! BarChart, Tabs, scrollbar y muestra de tema).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use mifutbol_rs::app::{App, Module, Screen};
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn test_app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");
    let db = Database::open(&paths.db_file).expect("db");
    let app = App::new(db, paths).expect("app");
    (app, dir)
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("handle_key");
}

fn render(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| ui::draw(frame, app)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn abrir_modulo(app: &mut App, indice: usize) {
    app.menu_state.select(Some(indice));
    press(app, KeyCode::Enter);
}

#[test]
fn wide_main_menu_shows_preview_and_shortcuts() {
    let (mut app, _dir) = test_app();
    let text = render(&mut app, 120, 34);
    assert!(text.contains("Vista previa"), "{text}");
    assert!(text.contains("Módulo 1 de 18"), "{text}");
    assert!(text.contains("Atajos"), "{text}");
    assert!(text.contains("Camisetas"), "{text}");
}

#[test]
fn camisetas_list_shows_side_preview() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute(
            "INSERT INTO camiseta (nombre, marca) VALUES ('Titular', 'Adidas')",
            [],
        )
        .unwrap();
    abrir_modulo(&mut app, 0);
    press(&mut app, KeyCode::Down);
    let text = render(&mut app, 120, 34);
    assert!(text.contains("Vista previa"), "{text}");
    assert!(text.contains("Titular"), "{text}");
    assert!(text.contains("Partidos"), "{text}");
}

#[test]
fn financiamiento_uses_gauge_and_barchart() {
    let (mut app, _dir) = test_app();
    let mes = chrono::Local::now().format("%Y-%m").to_string();
    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();
    app.db
        .conn()
        .execute(
            "INSERT INTO presupuesto_mensual
                (mes_anio, presupuesto_total_centavos, limite_gasto_centavos,
                 fecha_creacion, fecha_modificacion)
             VALUES (?1, 100000, 100000, ?2, ?2)",
            rusqlite::params![mes, hoy],
        )
        .unwrap();
    app.db
        .conn()
        .execute(
            "INSERT INTO financiamiento
                (fecha, tipo, categoria, descripcion, monto_centavos)
             VALUES (?1, 1, 5, 'Alquiler', 50000)",
            rusqlite::params![hoy],
        )
        .unwrap();
    abrir_modulo(&mut app, 7);
    let text = render(&mut app, 120, 34);
    assert!(text.contains("% consumido"), "{text}");
    assert!(text.contains("Gastos por categoría"), "{text}");
    assert!(text.contains("Presupuesto"), "{text}");
}

#[test]
fn wizard_shows_step_tabs() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
        .unwrap();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    abrir_modulo(&mut app, 4);
    press(&mut app, KeyCode::Char('n'));
    let text = render(&mut app, 120, 34);
    assert!(text.contains("Paso 1/4"), "{text}");
    assert!(text.contains("2 · Contexto"), "{text}");
    assert!(text.contains("4 · Confirmar"), "{text}");
}

#[test]
fn ajustes_shows_theme_preview() {
    let (mut app, _dir) = test_app();
    abrir_modulo(&mut app, 10);
    let text = render(&mut app, 120, 34);
    assert!(text.contains("Vista previa del tema"), "{text}");
    assert!(text.contains("Éxito"), "{text}");
    assert!(text.contains("Selección"), "{text}");
}

#[test]
fn module_tabs_show_context() {
    let (mut app, _dir) = test_app();
    abrir_modulo(&mut app, 4);
    let text = render(&mut app, 120, 34);
    assert!(text.contains("Finanzas"), "{text}");
    assert!(text.contains("Ajustes"), "{text}");
    assert!(text.contains("Camis"), "{text}");
}

#[test]
fn image_viewer_opens_and_closes() {
    let (mut app, _dir) = test_app();
    let ruta = std::fs::canonicalize("images/icon-512x512.png").expect("imagen");
    app.db
        .conn()
        .execute(
            "INSERT INTO camiseta (nombre, imagen_ruta) VALUES ('Con foto', ?1)",
            [ruta.to_string_lossy()],
        )
        .unwrap();
    abrir_modulo(&mut app, 0);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    assert_eq!(app.current_screen(), Screen::ImagenViewer);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Esc para volver"), "{text}");
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.current_screen(), Screen::CamisetaDetail(_)));
}

#[test]
fn sidebar_click_switches_module() {
    let (mut app, _dir) = test_app();
    abrir_modulo(&mut app, 4);
    let _ = render(&mut app, 160, 42);
    let objetivo = app
        .sidebar_hits
        .iter()
        .find(|(_, module)| *module == Module::Camisetas)
        .map(|(area, _)| *area)
        .expect("hit de Camisetas");
    let clic = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: objetivo.x + 1,
        row: objetivo.y,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(clic).expect("clic");
    assert_eq!(app.active_module, Some(Module::Camisetas));
    assert_eq!(app.current_screen(), Screen::CamisetasList);
}

#[test]
fn main_menu_click_opens_module() {
    let (mut app, _dir) = test_app();
    let _ = render(&mut app, 120, 34);
    let objetivo = app
        .menu_hits
        .iter()
        .find(|(_, indice)| *indice == 3)
        .map(|(area, _)| *area)
        .expect("hit de Equipos");
    let clic = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: objetivo.x + 1,
        row: objetivo.y,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(clic).expect("clic");
    assert_eq!(app.active_module, Some(Module::Equipos));
    assert_eq!(app.current_screen(), Screen::EquiposList);
}

#[test]
fn table_click_selects_row() {
    let (mut app, _dir) = test_app();
    for nombre in ["Uno", "Dos", "Tres"] {
        app.db
            .conn()
            .execute("INSERT INTO camiseta (nombre) VALUES (?1)", [nombre])
            .unwrap();
    }
    abrir_modulo(&mut app, 0);
    let _ = render(&mut app, 120, 34);
    let objetivo = app
        .table_hits
        .iter()
        .find(|(_, indice)| *indice == 2)
        .map(|(area, _)| *area)
        .expect("hit de fila 2");
    let clic = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: objetivo.x + 1,
        row: objetivo.y,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(clic).expect("clic");
    assert_eq!(app.camisetas_table.selected(), Some(2));
}

#[test]
fn canchas_table_click_selects_row() {
    let (mut app, _dir) = test_app();
    for nombre in ["Cancha Uno", "Cancha Dos"] {
        app.db
            .conn()
            .execute("INSERT INTO cancha (nombre) VALUES (?1)", [nombre])
            .unwrap();
    }
    abrir_modulo(&mut app, 1);
    let _ = render(&mut app, 120, 34);
    let objetivo = app
        .table_hits
        .iter()
        .find(|(_, indice)| *indice == 1)
        .map(|(area, _)| *area)
        .expect("hit de cancha 1");
    let clic = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: objetivo.x + 1,
        row: objetivo.y,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(clic).expect("clic");
    assert_eq!(app.canchas_table.selected(), Some(1));
}

#[test]
fn moving_selection_starts_row_animation() {
    let (mut app, _dir) = test_app();
    for nombre in ["Uno", "Dos"] {
        app.db
            .conn()
            .execute("INSERT INTO camiseta (nombre) VALUES (?1)", [nombre])
            .unwrap();
    }
    abrir_modulo(&mut app, 0);
    let _ = render(&mut app, 120, 34);
    app.animations.clear();
    press(&mut app, KeyCode::Down);
    let _ = render(&mut app, 120, 34);
    assert!(app.animations.running());
}
