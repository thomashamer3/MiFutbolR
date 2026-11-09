//! Pruebas de humo de la TUI con `TestBackend` (§22.1).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use mifutbol_rs::app::{App, Screen};
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

#[test]
fn main_menu_lists_all_modules() {
    let (mut app, _dir) = test_app();
    let text = render(&mut app, 100, 30);
    assert!(text.contains("MiFutbolR"), "falta el nombre de la app");
    for label in [
        "Camisetas",
        "Canchas",
        "Botines",
        "Equipos",
        "Partidos",
        "Lesiones",
        "Estadísticas",
        "Financiamiento",
        "Carrera futbolística",
        "Resumen compartible",
        "Ajustes",
        "Torneos",
        "Temporadas",
        "Tiendas",
        "Amigos",
        "Multimedia",
        "Recordatorios",
        "Progresión",
    ] {
        assert!(text.contains(label), "falta el módulo {label}");
    }
}

#[test]
fn enter_opens_camisetas_and_esc_returns() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("No hay camisetas"), "estado vacío");

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn arrow_keys_move_the_selection_with_wrap() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Up);
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Progresion);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Camisetas);
    press(&mut app, KeyCode::Char('3'));
    assert_eq!(app.selected_module(), mifutbol_rs::app::Module::Botines);
}

#[test]
fn quit_requires_confirmation() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('q'));
    assert!(app.modal.is_some());
    let text = render(&mut app, 100, 30);
    assert!(text.contains("¿Cerrar MiFutbolR?"), "el modal no se dibuja");
    assert!(!app.should_quit);

    press(&mut app, KeyCode::Char('s'));
    assert!(app.should_quit);
}

#[test]
fn help_screen_renders_shortcuts() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('?'));
    assert_eq!(app.current_screen(), Screen::Ayuda);
    let text = render(&mut app, 100, 30);
    assert!(text.contains("Atajos globales"));
    assert!(text.contains("Guardar formulario"));
}

#[test]
fn renders_in_narrow_and_short_terminals() {
    let (mut app, _dir) = test_app();
    for (width, height) in [(60, 20), (40, 15), (120, 40)] {
        let text = render(&mut app, width, height);
        assert!(!text.is_empty(), "sin contenido en {width}x{height}");
    }
}

#[test]
fn mouse_wheel_moves_menu_selection() {
    let (mut app, _dir) = test_app();
    let abajo = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(abajo).expect("mouse");
    assert_eq!(app.menu_state.selected(), Some(1));
    let arriba = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        ..abajo
    };
    app.handle_mouse(arriba).expect("mouse");
    assert_eq!(app.menu_state.selected(), Some(0));
}
