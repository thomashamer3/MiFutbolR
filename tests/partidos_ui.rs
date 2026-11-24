//! Flujo del módulo Partidos por la TUI (§12.9).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn preparar(app: &App) -> (i64, i64) {
    app.db
        .conn()
        .execute("INSERT INTO cancha (nombre) VALUES ('Cancha Sur')", [])
        .unwrap();
    let cancha_id = app.db.conn().last_insert_rowid();
    app.db
        .conn()
        .execute("INSERT INTO camiseta (nombre) VALUES ('Titular')", [])
        .unwrap();
    let camiseta_id = app.db.conn().last_insert_rowid();
    (cancha_id, camiseta_id)
}

fn open_partidos(app: &mut App) {
    for _ in 0..5 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

fn count(app: &App, sql: &str) -> i64 {
    app.db.conn().query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn create_favorite_and_delete_partido() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_partidos(&mut app);
    assert_eq!(app.current_screen(), Screen::PartidosList);

    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.current_screen(), Screen::PartidoWizard);
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::PartidosList);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM partido"), 1);

    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Cancha Sur"), "la cancha no aparece: {text}");
    assert!(text.contains("Victoria"));

    // Favorito.
    press(&mut app, KeyCode::Char('F'));
    assert_eq!(
        count(&app, "SELECT COUNT(*) FROM partido_meta WHERE favorito = 1"),
        1
    );

    // Detalle.
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::PartidoDetail(_)));
    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Marcador"));
    assert!(text.contains("Contexto"));
    assert!(text.contains("Estadísticas") || text.contains("Métricas"));
    press(&mut app, KeyCode::Esc);

    // Eliminar.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count(&app, "SELECT COUNT(*) FROM partido"), 0);
}

#[test]
fn missing_prerequisites_are_reported() {
    let (mut app, _dir) = test_app();
    open_partidos(&mut app);
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.current_screen(), Screen::PartidosList);
    assert!(app.modal.is_some());
    let text_matches = matches!(app.modal, Some(mifutbol_rs::app::Modal::Error(_)));
    assert!(text_matches);
}

#[test]
fn pagination_splits_results() {
    let (mut app, _dir) = test_app();
    let (cancha_id, camiseta_id) = preparar(&app);
    for day in 1..=25 {
        app.db
            .conn()
            .execute(
                "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![cancha_id, camiseta_id, format!("2026-03-{day:02} 20:00:00")],
            )
            .unwrap();
    }
    open_partidos(&mut app);
    assert_eq!(app.partido_total, 25);
    assert_eq!(app.partidos.len(), 20);
    assert_eq!(app.partido_page, 1);

    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.partido_page, 2);
    assert_eq!(app.partidos.len(), 5);

    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.partido_page, 1);

    // Filtro por tipo: Amistoso no tiene filas (todas quedaron con resultado 0, tipo 1 por defecto).
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(
        app.partido_filter.tipo_partido,
        Some(mifutbol_rs::domain::models::TipoPartido::Amistoso)
    );
    assert_eq!(app.partido_total, 25);
}

#[test]
fn wizard_creates_match_in_four_steps() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    open_partidos(&mut app);

    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.current_screen(), Screen::PartidoWizard);
    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Paso 1/4"), "{text}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.partido_wizard_step, 1);
    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Paso 2/4"), "{text}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.partido_wizard_step, 2);
    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Paso 3/4"), "{text}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.partido_wizard_step, 3);
    let text = render_con(&mut app, 130, 34);
    assert!(text.contains("Confirmar"), "{text}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), Screen::PartidosList);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM partido"), 1);
}

#[test]
fn home_summary_refreshes_only_when_menu_is_visible() {
    let (mut app, _dir) = test_app();
    preparar(&app);
    assert!(app.inicio.ultimo.is_none());
    open_partidos(&mut app);
    press(&mut app, KeyCode::Char('n'));
    press_ctrl(&mut app, 's');
    assert_eq!(count(&app, "SELECT COUNT(*) FROM partido"), 1);
    // Guardado fuera del menú: el Inicio queda pendiente, sin recalcular.
    assert!(app.inicio_sucio);
    assert!(app.inicio.ultimo.is_none());

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(!app.inicio_sucio);
    assert!(
        app.inicio.ultimo.is_some(),
        "el Inicio muestra el partido nuevo"
    );
}

#[test]
fn go_back_recomputes_home_only_after_writes() {
    let (mut app, _dir) = test_app();
    let (cancha_id, camiseta_id) = preparar(&app);
    let insertar = "INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, mes_anio)
                    VALUES (?1, ?2, '2026-03-01 20:00:00', '2026-03')";

    // Escritura desde otra conexión: no cuenta en `total_changes()` de la app,
    // así que volver sin cambios propios no recalcula el Inicio.
    let otra = rusqlite::Connection::open(&app.paths.db_file).unwrap();
    open_partidos(&mut app);
    press(&mut app, KeyCode::Esc); // sincroniza el contador tras `preparar`
    open_partidos(&mut app);
    otra.execute(insertar, rusqlite::params![cancha_id, camiseta_id])
        .unwrap();
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::MainMenu);
    assert!(
        app.inicio.ultimo.is_none(),
        "volver sin cambios no recalcula"
    );

    // Una escritura propia (cualquier tabla, sin marcar `inicio_sucio`) sí recalcula.
    open_partidos(&mut app);
    app.db
        .conn()
        .execute("UPDATE cancha SET localidad = 'Quilmes'", [])
        .unwrap();
    assert!(!app.inicio_sucio);
    press(&mut app, KeyCode::Esc);
    assert!(
        app.inicio.ultimo.is_some(),
        "tras guardar, el Inicio se recalcula"
    );
}

#[test]
fn fecha_y_hora_se_ven_enteras_con_ancho_medio() {
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    abrir_opcion(&mut app, mifutbol_rs::app::Module::Partidos);
    // 120 columnas: antes el panel de vista previa dejaba la tabla en ~81 y cortaba la hora.
    let text = render_con(&mut app, 120, 32);
    assert!(text.contains("01/03/2026 20:00"), "hora cortada: {text}");
}
