//! Pruebas de humo de la Ola 0: dashboard, calendario, logros, avisos y overlays.

use crossterm::event::KeyCode;
use mifutbol_rs::app::{Overlay, Screen};

mod common;
use common::*;

#[test]
fn main_menu_shows_dashboard_cards() {
    let (mut app, _dir) = test_app();
    let text = render_con(&mut app, 120, 40);
    for titulo in ["Próximo partido", "Racha", "Este mes", "Alertas"] {
        assert!(text.contains(titulo), "falta la tarjeta {titulo}");
    }
}

#[test]
fn calendar_opens_navigates_and_closes() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(app.current_screen(), Screen::Calendario);
    let text = render_con(&mut app, 120, 36);
    assert!(
        text.contains("Calendario"),
        "falta el título del calendario"
    );

    let dia = app.calendario.dia;
    press(&mut app, KeyCode::Right);
    assert!(app.calendario.dia >= dia);
    let mes = app.calendario.month;
    press(&mut app, KeyCode::Char('n'));
    assert_ne!(app.calendario.month, mes);

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn logros_open_with_catalog_and_tabs() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(app.current_screen(), Screen::Logros);
    assert!(app.logros.len() >= 50, "catálogo demasiado corto");
    let text = render_con(&mut app, 110, 32);
    assert!(text.contains("Goleador"), "falta un logro");
    press(&mut app, KeyCode::Right);
    assert_eq!(app.logros_tab, mifutbol_rs::app::LogrosTab::Completados);
    let text = render_con(&mut app, 110, 32);
    assert!(text.contains("Completados"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn notifications_open() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('!'));
    assert_eq!(app.current_screen(), Screen::Notificaciones);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Avisos") || text.contains("Notificaciones"));
}

#[test]
fn global_search_opens_and_closes() {
    let (mut app, _dir) = test_app();
    press_ctrl_code(&mut app, KeyCode::Char('f'));
    assert!(matches!(app.overlay, Some(Overlay::Busqueda(_))));
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Búsqueda global"));
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Esc);
    assert!(app.overlay.is_none());
}

#[test]
fn palette_filters_and_executes_calendar() {
    let (mut app, _dir) = test_app();
    press_ctrl_code(&mut app, KeyCode::Char('k'));
    assert!(matches!(app.overlay, Some(Overlay::Paleta(_))));
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Paleta de comandos"));

    for caracter in "calendario".chars() {
        press(&mut app, KeyCode::Char(caracter));
    }
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Calendario"));
    press(&mut app, KeyCode::Enter);
    assert!(app.overlay.is_none());
    assert_eq!(app.current_screen(), Screen::Calendario);
}

#[test]
fn tutorial_opens_and_completes() {
    let (mut app, _dir) = test_app();
    assert!(!app.tutorial_completado);
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(app.current_screen(), Screen::Tutorial);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Bienvenido a MiFutbolR"));
    for _ in 0..4 {
        press(&mut app, KeyCode::Enter);
    }
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(app.tutorial_completado);
}

#[test]
fn export_list_writes_four_formats() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('1'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::CamisetasList);
    press(&mut app, KeyCode::Char('x'));
    assert!(app.status.contains("Exportados"), "status: {}", app.status);
    let archivos = std::fs::read_dir(&app.paths.export_dir)
        .expect("export dir")
        .count();
    assert!(archivos >= 4, "archivos exportados: {archivos}");
}

#[test]
fn records_open_and_close() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('R'));
    assert_eq!(app.current_screen(), Screen::Records);
    let text = render_con(&mut app, 110, 34);
    assert!(text.contains("Récords"));
    assert!(text.contains("Récord de goles"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn ctrl_z_undoes_last_creation() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Temporal')", [])
        .unwrap();
    press_ctrl_code(&mut app, KeyCode::Char('z'));
    assert!(app.modal.is_some(), "debe abrir la confirmación de undo");
    press(&mut app, KeyCode::Enter);
    let total: i64 = app
        .db
        .conn()
        .query_row("SELECT COUNT(*) FROM camiseta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 0);
    assert!(app.status.contains("Deshecho"), "status: {}", app.status);
}

#[test]
fn torneos_open_detail_and_generate_fixture() {
    let (mut app, _dir) = test_app();
    for nombre in ["A", "B", "C"] {
        app.db
            .conn()
            .execute("INSERT INTO equipo (nombre) VALUES (?1)", [nombre])
            .unwrap();
    }
    press(&mut app, KeyCode::Char('T'));
    assert_eq!(app.current_screen(), Screen::TorneosList);
    assert!(app.torneos.is_empty());
    press(&mut app, KeyCode::Esc);

    app.db
        .conn()
        .execute(
            "INSERT INTO torneo (nombre, tipo_torneo) VALUES ('Copa', 0)",
            [],
        )
        .unwrap();
    let id = app.db.conn().last_insert_rowid();
    app.db
        .conn()
        .execute(
            "INSERT INTO equipo_torneo (torneo_id, equipo_id)
             SELECT ?1, id FROM equipo",
            [id],
        )
        .unwrap();
    app.db
        .conn()
        .execute("UPDATE torneo SET cantidad_equipos = 3 WHERE id = ?1", [id])
        .unwrap();

    press(&mut app, KeyCode::Char('T'));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::TorneoDetail(_)));
    press(&mut app, KeyCode::Char('g'));
    press(&mut app, KeyCode::Enter);
    let ficha = app.torneo_detail.as_ref().expect("ficha");
    assert_eq!(ficha.partidos.len(), 3);
    let text = render_con(&mut app, 130, 38);
    assert!(text.contains("Fixture"));
}

#[test]
fn ultimos_open() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('U'));
    assert_eq!(app.current_screen(), Screen::Ultimos);
    let text = render_con(&mut app, 110, 32);
    assert!(text.contains("Última victoria"));
    assert!(text.contains("Último gol"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn comparador_opens_and_compares() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Azul')", [])
        .unwrap();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Roja')", [])
        .unwrap();
    press(&mut app, KeyCode::Char('C'));
    assert_eq!(app.current_screen(), Screen::Comparador);
    assert_eq!(app.comparador_opciones.len(), 2);
    press(&mut app, KeyCode::Enter);
    assert!(app.comparador_resultado.is_some());
    let text = render_con(&mut app, 110, 34);
    assert!(text.contains("Goles"));
    assert!(text.contains("Azul"));
    assert!(text.contains("Roja"));
}

#[test]
fn temporadas_open_and_form() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('E'));
    assert_eq!(app.current_screen(), Screen::TemporadasList);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::TemporadaForm(_)));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn tiendas_y_amigos_abren() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('D'));
    assert_eq!(app.current_screen(), Screen::TiendasList);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::TiendaForm(_)));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);

    press(&mut app, KeyCode::Char('A'));
    assert_eq!(app.current_screen(), Screen::AmigosList);
    let text = render_con(&mut app, 100, 30);
    assert!(text.contains("Yo"));
}

#[test]
fn multimedia_open_and_form() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('M'));
    assert_eq!(app.current_screen(), Screen::MediaList);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::MediaForm(_)));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn recordatorios_y_agenda_abren() {
    let (mut app, _dir) = test_app();
    press(&mut app, KeyCode::Char('Y'));
    assert_eq!(app.current_screen(), Screen::RecordatoriosList);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::RecordatorioForm(_)));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(app.current_screen(), Screen::Agenda);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
}

#[test]
fn progresion_open_with_form() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute("INSERT INTO equipo (nombre) VALUES ('Equipo')", [])
        .unwrap();
    app.db
        .conn()
        .execute(
            "INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES (1, 'Ana', 10, 3)",
            [],
        )
        .unwrap();
    press(&mut app, KeyCode::Char('P'));
    assert_eq!(app.current_screen(), Screen::ProgresionList);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::ProgresionForm(_)));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.current_screen(), Screen::PlanesList);
    press(&mut app, KeyCode::Esc);
}
