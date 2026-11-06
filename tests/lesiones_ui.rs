//! Flujo del módulo Lesiones por la TUI (§13.6).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

fn preparar(app: &App) {
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("handle_key");
}

fn press_ctrl(app: &mut App, ch: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL))
        .expect("handle_key");
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

fn render(app: &mut App) -> String {
    let backend = TestBackend::new(120, 34);
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

fn open_lesiones(app: &mut App) {
    for _ in 0..5 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

fn count(app: &App) -> i64 {
    app.db
        .conn()
        .query_row("SELECT COUNT(*) FROM lesion", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn create_filter_detail_and_delete_lesion() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_lesiones(&mut app);
    assert_eq!(app.current_screen(), Screen::LesionesList);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::LesionForm(_)));
    type_text(&mut app, "Ana Gomez");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "Esguince");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "Tobillo derecho");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::LesionesList);
    assert_eq!(count(&app), 1);
    assert_eq!(app.lesion_stats.total, 1);
    assert_eq!(app.lesion_stats.activas, 1);

    let text = render(&mut app);
    assert!(text.contains("Ana Gomez"), "la lesión no aparece: {text}");
    assert!(text.contains("Esguince"));

    // Detalle.
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::LesionDetail(_)));
    let text = render(&mut app);
    assert!(text.contains("Tobillo derecho"));
    press(&mut app, KeyCode::Esc);

    // Filtro por estado: activa -> en tratamiento (0 filas) -> ... -> todas.
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(app.lesiones.len(), 1);
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(app.lesiones.len(), 0);
    for _ in 0..4 {
        press(&mut app, KeyCode::Char('f'));
    }
    assert_eq!(app.lesiones.len(), 1);

    // Eliminar.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count(&app), 0);
}

#[test]
fn missing_camiseta_is_reported() {
    let (mut app, _dir) = test_app();
    open_lesiones(&mut app);
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.current_screen(), Screen::LesionesList);
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Error(_))));
}

#[test]
fn search_filters_by_player_or_type() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_lesiones(&mut app);
    for (jugador, tipo) in [("Ana", "Esguince"), ("Beto", "Contractura")] {
        press(&mut app, KeyCode::Char('n'));
        type_text(&mut app, jugador);
        press(&mut app, KeyCode::Tab);
        type_text(&mut app, tipo);
        press(&mut app, KeyCode::Tab);
        type_text(&mut app, "Detalle");
        press_ctrl(&mut app, 's');
    }
    assert_eq!(app.lesiones.len(), 2);

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "beto");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.lesiones.len(), 1);
    assert_eq!(app.lesiones[0].jugador, "Beto");

    press(&mut app, KeyCode::Char('/'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.lesiones.len(), 2);
}
