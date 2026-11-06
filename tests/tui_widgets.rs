//! Verificación de los widgets de TUI mejorados (vista previa, Gauge,
//! BarChart, Tabs, scrollbar y muestra de tema).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mifutbol_rs::app::App;
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
    assert!(text.contains("Módulo 1 de 11"), "{text}");
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
