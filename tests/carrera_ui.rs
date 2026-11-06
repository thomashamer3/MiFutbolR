//! Flujo del módulo Carrera por la TUI (§16.7).

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
    let backend = TestBackend::new(120, 36);
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

fn open_carrera(app: &mut App) {
    for _ in 0..8 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

#[test]
fn hub_resumen_and_identity_form() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_carrera(&mut app);
    assert_eq!(app.current_screen(), Screen::CarreraMenu);

    let text = render(&mut app);
    assert!(text.contains("Resumen"));
    assert!(text.contains("Hall of Fame"));

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraResumen);
    let text = render(&mut app);
    assert!(text.contains("Partidos"));
    assert!(text.contains("Mejor racha"));
    press(&mut app, KeyCode::Esc);

    // Ficha de identidad.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraIdentidadForm);
    type_text(&mut app, "Jugador");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CarreraMenu);
    let nombre: String = app
        .db
        .conn()
        .query_row(
            "SELECT nombre FROM carrera_identidad WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(nombre, "Jugador");
}

#[test]
fn create_and_delete_hito() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_carrera(&mut app);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraHitos);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::CarreraHitoForm(_)));
    press(&mut app, KeyCode::Tab); // del partido al tipo
    type_text(&mut app, "Primer gol");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::CarreraHitos);
    let hitos: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM carrera_partido_hito", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(hitos, 1);

    let text = render(&mut app);
    assert!(text.contains("Primer gol"));

    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    let hitos: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM carrera_partido_hito", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(hitos, 0);
}

#[test]
fn timeline_and_logros_render() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_carrera(&mut app);
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter); // Timeline
    assert_eq!(app.current_screen(), Screen::CarreraTimeline);
    let text = render(&mut app);
    assert!(text.contains("timeline") || text.contains("Timeline"));
    assert!(text.contains("Partido"));
    press(&mut app, KeyCode::Esc);

    for _ in 0..4 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter); // Logros
    assert_eq!(app.current_screen(), Screen::CarreraLogros);
    let text = render(&mut app);
    assert!(text.contains("Primer partido"));
    assert!(text.contains("Hat-trick"));
}

#[test]
fn perfil_y_modo_retro_render() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    app.menu_state.select(Some(8));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraMenu);

    app.carrera_hub.select(Some(5));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraPerfil);
    let text = render(&mut app);
    assert!(text.contains("Etiqueta"));
    press(&mut app, KeyCode::Esc);

    app.carrera_hub.select(Some(6));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CarreraRetro);
    let text = render(&mut app);
    assert!(text.contains("Modo retro") || text.contains("Sin recuerdos"));
}
