//! Navegación del módulo Estadísticas por la TUI (§14.6).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mifutbol_rs::app::{App, Screen};
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::domain::models::EstadisticaKind;
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
        .execute(
            "INSERT INTO cancha (nombre, localidad) VALUES ('Sur', 'Quilmes')",
            [],
        )
        .unwrap();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    for (fecha, goles, resultado, rendimiento) in [
        ("2026-03-10 20:00:00", 2, 1, 7),
        ("2026-03-20 20:00:00", 1, 3, 5),
        ("2026-04-05 20:00:00", 3, 1, 9),
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

fn render(app: &mut App) -> String {
    let backend = TestBackend::new(130, 40);
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

fn open_estadisticas(app: &mut App) {
    for _ in 0..6 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

#[test]
fn hub_lists_categories_and_opens_details() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_estadisticas(&mut app);
    assert_eq!(app.current_screen(), Screen::EstadisticasHub);

    let text = render(&mut app);
    assert!(text.contains("Generales"));
    assert!(text.contains("Rendimiento"));
    assert!(text.contains("Goles y asistencias"));

    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.current_screen(),
        Screen::EstadisticasDetalle(EstadisticaKind::Generales)
    );
    let text = render(&mut app);
    assert!(text.contains("Total de partidos"));
    assert!(text.contains("Récords"));

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::EstadisticasHub);

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.current_screen(),
        Screen::EstadisticasDetalle(EstadisticaKind::PorMes)
    );
    let text = render(&mut app);
    assert!(text.contains("2026-03"));
    assert!(text.contains("2026-04"));

    // Desplazamiento y vuelta.
    press(&mut app, KeyCode::PageDown);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn empty_state_without_matches() {
    let (mut app, _dir) = test_app();
    open_estadisticas(&mut app);
    press(&mut app, KeyCode::Enter);
    let text = render(&mut app);
    assert!(text.contains("Sin datos suficientes"));
}

#[test]
fn rendering_other_categories_does_not_panic() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_estadisticas(&mut app);
    for kind in [
        EstadisticaKind::PorAnio,
        EstadisticaKind::Rendimiento,
        EstadisticaKind::Canchas,
        EstadisticaKind::Goles,
    ] {
        app.estadisticas_hub.select(Some(
            EstadisticaKind::ALL
                .iter()
                .position(|k| *k == kind)
                .unwrap(),
        ));
        press(&mut app, KeyCode::Enter);
        let text = render(&mut app);
        assert!(!text.is_empty());
        assert!(text.contains(kind.label()));
        press(&mut app, KeyCode::Esc);
    }
}
