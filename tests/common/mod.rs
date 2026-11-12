//! Helpers compartidos por las pruebas de integración de la TUI.
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mifutbol_rs::app::App;
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// Crea una app con base de datos temporal sin marcar la bienvenida.
pub fn test_app_con_bienvenida() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");
    let db = Database::open(&paths.db_file).expect("db");
    let app = App::new(db, paths).expect("app");
    (app, dir)
}

/// Crea una app con base de datos temporal.
pub fn test_app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_root(dir.path());
    paths.ensure_dirs().expect("dirs");
    let db = Database::open(&paths.db_file).expect("db");
    let _ = mifutbol_rs::services::usuario_service::marcar_bienvenida(db.conn());
    let app = App::new(db, paths).expect("app");
    (app, dir)
}

pub fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("handle_key");
}

pub fn press_ctrl(app: &mut App, ch: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL))
        .expect("handle_key");
}

pub fn press_ctrl_code(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::CONTROL))
        .expect("handle_key");
}

pub fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

pub fn render_con(app: &mut App, width: u16, height: u16) -> String {
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

/// Carga una cancha, una camiseta y tres partidos de prueba.
pub fn preparar_partidos(app: &App) {
    app.db
        .conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Sur')", [])
        .unwrap();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    for (fecha, goles, resultado, rendimiento) in [
        ("2026-03-01 20:00:00", 3, 1, 9),
        ("2026-03-08 20:00:00", 1, 1, 7),
        ("2026-03-15 20:00:00", 2, 3, 6),
    ] {
        app.db
            .conn()
            .execute(
                "INSERT INTO partido
                    (cancha_id, camiseta_id, fecha_hora, mes_anio, goles, asistencias,
                     resultado, rendimiento_general, clima, dia, tipo_partido)
                 VALUES (1, 1, ?1, ?2, ?3, 1, ?4, ?5, 1, 2, 1)",
                rusqlite::params![fecha, &fecha[..7], goles, resultado, rendimiento],
            )
            .unwrap();
    }
}
