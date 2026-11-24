//! B9: funciones de MiFutbolC que faltaban (simulación, lesiones, estadísticas,
//! amigos, notificaciones, equipamiento, camisetas, torneos y temporadas).

mod common;

use common::*;
use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Eleccion, Module, Screen};

fn count(app: &App, sql: &str) -> i64 {
    app.db.conn().query_row(sql, [], |row| row.get(0)).unwrap()
}

/// Dos equipos con un jugador cada uno.
fn dos_equipos(app: &App) {
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO equipo (id, nombre, tipo, tipo_futbol, num_jugadores) VALUES
                (1, 'Rojos', 0, 0, 5), (2, 'Azules', 0, 0, 5);
             INSERT INTO jugador (equipo_id, nombre, numero, posicion) VALUES
                (1, 'Ana', 9, 3), (2, 'Beto', 10, 3);",
        )
        .unwrap();
}

#[test]
fn simular_con_equipos_guardados_muestra_el_resultado_sin_guardar() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Partidos);
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.current_screen(), Screen::PartidosList, "faltan equipos");
    assert!(app.status.contains("al menos 2 equipos"), "{}", app.status);

    dos_equipos(&app);
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(
        app.current_screen(),
        Screen::Eleccion(Eleccion::SimularEquipos)
    );
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Reporte);
    let texto = render_con(&mut app, 120, 40);
    assert!(texto.contains("Rojos vs Azules"), "{texto}");
    assert!(texto.contains("Alineaciones"), "{texto}");
    assert_eq!(count(&app, "SELECT COUNT(*) FROM partido"), 0);

    press(&mut app, KeyCode::Char('x'));
    assert!(app.status.contains("reporte_"), "{}", app.status);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.current_screen(), Screen::PartidosList);
}

#[test]
fn lesiones_diferencias_y_actualizar_estado() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO camiseta (nombre) VALUES ('Titular');
             INSERT INTO lesion (jugador, tipo, descripcion, fecha, camiseta_id, estado) VALUES
                ('Yo', 'Esguince', 'Tobillo', '2026-03-01 10:00:00', 1, 'ACTIVA'),
                ('Yo', 'Desgarro', 'Isquio', '2026-03-11 10:00:00', 1, 'ACTIVA');",
        )
        .unwrap();
    abrir_opcion(&mut app, Module::Lesiones);
    assert_eq!(app.current_screen(), Screen::LesionesList);
    let id = app.selected_lesion_id().unwrap();
    press(&mut app, KeyCode::Char('u'));
    let estado: String = app
        .db
        .conn()
        .query_row("SELECT estado FROM lesion WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(estado, "EN TRATAMIENTO");

    press(&mut app, KeyCode::Char('D'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    let texto = render_con(&mut app, 120, 30);
    assert!(texto.contains("10 días después"), "{texto}");
}

fn abrir_estadistica(app: &mut App, kind: mifutbol_rs::domain::models::EstadisticaKind) {
    use mifutbol_rs::domain::models::EstadisticaKind;
    abrir_opcion(app, Module::Estadisticas);
    let indice = EstadisticaKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap();
    app.estadisticas_hub.select(Some(indice));
    press(app, KeyCode::Enter);
}

#[test]
fn estadisticas_posiciones_contexto_filtros_y_reportes() {
    use mifutbol_rs::domain::models::EstadisticaKind;
    let (mut app, _dir) = test_app();
    preparar_partidos(&app);
    for kind in [EstadisticaKind::Posiciones, EstadisticaKind::Contexto] {
        abrir_estadistica(&mut app, kind);
        assert_eq!(app.current_screen(), Screen::Reporte, "{kind:?}");
        let texto = render_con(&mut app, 120, 40);
        assert!(
            texto.contains(kind.label()) || texto.contains("Contexto"),
            "{texto}"
        );
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
    }

    abrir_estadistica(&mut app, EstadisticaKind::Filtros);
    assert_eq!(
        app.current_screen(),
        Screen::Eleccion(Eleccion::FiltrosPartidos)
    );
    type_text(&mut app, "08/03/2026");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Reporte);
    assert!(
        app.reporte
            .lineas
            .iter()
            .any(|l| l.starts_with("Total 2 partidos")),
        "{:?}",
        app.reporte.lineas
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);

    abrir_estadistica(&mut app, EstadisticaKind::Reportes);
    assert_eq!(
        app.current_screen(),
        Screen::Eleccion(Eleccion::GenerarReporte)
    );
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Reporte);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM reporte_generado"), 1);
    assert!(app.status.contains("reporte_general_"), "{}", app.status);
}

#[test]
fn amigos_comparar_conmigo() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute(
            "INSERT INTO amigo (nombre, posicion, goles, asistencias, partidos)
             VALUES ('Zoe', 'Arquera', 0, 0, 0)",
            [],
        )
        .unwrap();
    abrir_opcion(&mut app, Module::Amigos);
    assert_eq!(app.current_screen(), Screen::AmigosList);
    let indice = app.amigos_ranking.iter().position(|f| !f.local).unwrap();
    app.amigos_table.select(Some(indice));
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    let texto = render_con(&mut app, 110, 30);
    assert!(texto.contains("Comparar conmigo"), "{texto}");
    assert!(texto.contains("Arquera"), "{texto}");
}

#[test]
fn notificaciones_crear_a_mano_y_cumpleanos_del_jugador() {
    let (mut app, _dir) = test_app();
    dos_equipos(&app);
    let hoy = mifutbol_rs::domain::dates::today();
    app.db
        .conn()
        .execute(
            "UPDATE jugador SET fecha_nacimiento = ?1 WHERE nombre = 'Ana'",
            [hoy.strftime("2000-%m-%d").to_string()],
        )
        .unwrap();
    press(&mut app, KeyCode::Char('!'));
    assert_eq!(app.current_screen(), Screen::Notificaciones);
    assert!(
        app.notificaciones
            .iter()
            .any(|n| n.mensaje == "Ana cumple años ¡hoy!")
    );

    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.current_screen(),
        Screen::Eleccion(Eleccion::NuevaNotificacion)
    );
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "Pagar cancha");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "Antes del viernes");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::Notificaciones);
    assert!(
        app.notificaciones
            .iter()
            .any(|n| n.titulo == "Pagar cancha")
    );
    let texto = render_con(&mut app, 120, 30);
    assert!(texto.contains("Pagar cancha"), "{texto}");
}

#[test]
fn historial_de_equipamiento_desde_colecciones() {
    let (mut app, _dir) = test_app();
    abrir_opcion(&mut app, Module::Colecciones);
    press(&mut app, KeyCode::Char('H'));
    assert_eq!(app.current_screen(), Screen::EquipamientoList);
    render_con(&mut app, 120, 30);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::EquipamientoForm(_)));
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "Adidas");
    press_ctrl(&mut app, 's');
    assert_eq!(
        app.current_screen(),
        Screen::EquipamientoList,
        "{:?}",
        app.modal
    );
    assert_eq!(
        count(&app, "SELECT COUNT(*) FROM equipamiento_historial"),
        1
    );
    let texto = render_con(&mut app, 120, 30);
    assert!(texto.contains("Adidas"), "{texto}");
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    assert!(app.reporte.texto().contains("Total items:      1"));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(
        count(&app, "SELECT COUNT(*) FROM equipamiento_historial"),
        0
    );
}

#[test]
fn camisetas_grafico_de_uso() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO cancha (id, nombre) VALUES (1, 'C');
             INSERT INTO camiseta (id, nombre) VALUES (1, 'Titular'), (2, 'Suplente');
             INSERT INTO partido (cancha_id, camiseta_id, fecha_hora, goles) VALUES
                (1, 1, '2026-01-01 10:00:00', 2), (1, 1, '2026-01-02 10:00:00', 1);",
        )
        .unwrap();
    abrir_opcion(&mut app, Module::Camisetas);
    press(&mut app, KeyCode::Char('u'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    assert_eq!(app.reporte.barras[0], ("Titular".to_string(), 2));
    let texto = render_con(&mut app, 120, 40);
    assert!(texto.contains("Gráfico"), "{texto}");
    assert!(texto.contains("Titular: 2 partidos · 3 goles"), "{texto}");
}

#[test]
fn torneo_finalizar_y_reporte_con_historial() {
    let (mut app, _dir) = test_app();
    dos_equipos(&app);
    abrir_opcion(&mut app, Module::Torneos);
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "Copa");
    press_ctrl(&mut app, 's');
    assert!(matches!(app.current_screen(), Screen::TorneoDetail(_)));
    press(&mut app, KeyCode::Char('g'));
    press(&mut app, KeyCode::Char('s'));
    app.db
        .conn()
        .execute_batch(
            "UPDATE partido_torneo SET goles_equipo1 = 2, goles_equipo2 = 0, estado = 'Jugado';",
        )
        .unwrap();
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('f'));
    press(&mut app, KeyCode::Char('s'));
    assert!(app.status.contains("Torneo finalizado"), "{}", app.status);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM equipo_historial"), 2);
    let texto = render_con(&mut app, 130, 30);
    assert!(texto.contains("Finalizado"), "{texto}");

    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    let reporte = app.reporte.texto();
    assert!(reporte.contains("Tabla de posiciones"), "{reporte}");
    assert!(reporte.contains("· Copa: posición 1"), "{reporte}");
    press(&mut app, KeyCode::Char('x'));
    assert!(app.status.contains("reporte_"), "{}", app.status);
}

#[test]
fn temporada_asociar_torneos_y_resumen() {
    let (mut app, _dir) = test_app();
    app.db
        .conn()
        .execute_batch(
            "INSERT INTO temporada (nombre, anio, fecha_inicio, fecha_fin, descripcion)
                VALUES ('2026', 2026, '2026-01-01', '2026-12-31', '');
             INSERT INTO torneo (nombre) VALUES ('Liga');",
        )
        .unwrap();
    abrir_opcion(&mut app, Module::Temporadas);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.current_screen(), Screen::TemporadaDetail(_)));
    press(&mut app, KeyCode::Char('t'));
    assert!(app.status.contains("1 torneo(s)"), "{}", app.status);
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.current_screen(), Screen::Reporte);
    let texto = app.reporte.texto();
    assert!(texto.contains("Resumen de temporada: 2026"), "{texto}");
    assert!(texto.contains("Liga"), "{texto}");
}
