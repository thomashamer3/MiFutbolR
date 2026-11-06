//! Flujo completo del módulo Camisetas por la TUI (§22.2).

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
    let backend = TestBackend::new(110, 32);
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

fn count_rows(app: &App) -> i64 {
    app.db
        .conn()
        .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn create_camiseta_through_the_ui() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter); // abre Camisetas
    assert_eq!(app.current_screen(), Screen::CamisetasList);

    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.current_screen(),
        Screen::CamisetaForm(mifutbol_rs::domain::models::FormMode::Create)
    );

    type_text(&mut app, "Titular 10");
    press_ctrl(&mut app, 's');

    assert_eq!(app.current_screen(), Screen::CamisetasList);
    assert_eq!(count_rows(&app), 1);
    let text = render(&mut app);
    assert!(text.contains("Titular 10"), "la fila no aparece: {text}");
}

#[test]
fn empty_name_is_rejected_and_form_stays_open() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('n'));
    press_ctrl(&mut app, 's');

    assert!(matches!(app.current_screen(), Screen::CamisetaForm(_)));
    assert!(app.modal.is_some());
    assert_eq!(count_rows(&app), 0);

    // Cerrar el modal de error y volver.
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
}

#[test]
fn edit_and_delete_through_the_ui() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Vieja");
    press_ctrl(&mut app, 's');
    assert_eq!(count_rows(&app), 1);

    // Editar: el formulario conserva el nombre; agregamos un sufijo.
    press(&mut app, KeyCode::Char('e'));
    assert!(matches!(
        app.current_screen(),
        Screen::CamisetaForm(mifutbol_rs::domain::models::FormMode::Edit)
    ));
    type_text(&mut app, " Nueva");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CamisetasList);
    let name: String = app
        .db
        .conn()
        .query_row("SELECT nombre FROM camiseta LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(name, "Vieja Nueva");

    // Eliminar con confirmación.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count_rows(&app), 0);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
}

#[test]
fn search_filters_the_list() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    for name in ["Roja", "Azul"] {
        press(&mut app, KeyCode::Char('n'));
        type_text(&mut app, name);
        press_ctrl(&mut app, 's');
    }
    assert_eq!(app.camisetas.len(), 2);

    press(&mut app, KeyCode::Char('/'));
    assert!(app.search.is_some());
    type_text(&mut app, "roj");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.camisetas.len(), 1);
    assert_eq!(app.camisetas[0].nombre, "Roja");

    // Limpiar el filtro volviendo a buscar vacío.
    press(&mut app, KeyCode::Char('/'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.camisetas.len(), 2);
}

#[test]
fn draw_awards_a_jersey_and_marks_it() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Sorteable");
    press_ctrl(&mut app, 's');

    press(&mut app, KeyCode::Char('s'));
    assert!(matches!(app.modal, Some(mifutbol_rs::app::Modal::Info(_))));
    let sorteada: i64 = app
        .db
        .conn()
        .query_row("SELECT sorteada FROM camiseta LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(sorteada, 1);
}
