//! Flujo del módulo Financiamiento por la TUI (§15.6).

use crossterm::event::KeyCode;
use mifutbol_rs::app::{App, Screen};

mod common;
use common::*;

fn open_financiamiento(app: &mut App) {
    for _ in 0..7 {
        press(app, KeyCode::Down);
    }
    press(app, KeyCode::Enter);
}

fn count(app: &App, sql: &str) -> i64 {
    app.db.conn().query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn create_transaction_and_delete_it() {
    let (mut app, _dir) = test_app();
    open_financiamiento(&mut app);
    assert_eq!(app.current_screen(), Screen::FinanciamientoList);

    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.current_screen(), Screen::TransaccionForm(_)));
    type_text(&mut app, "Alquiler cancha");
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::FinanciamientoList);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM financiamiento"), 1);

    let text = render_con(&mut app, 130, 36);
    assert!(text.contains("Alquiler cancha"));
    assert!(text.contains("Ingresos"));
    assert!(text.contains("Balance"));

    // Filtros por tipo: Ingreso -> Gasto -> Todos.
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(
        app.finanzas_filter.tipo,
        Some(mifutbol_rs::domain::models::TipoTransaccion::Ingreso)
    );
    assert_eq!(app.transacciones.len(), 0);
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(app.transacciones.len(), 1);
    press(&mut app, KeyCode::Char('f'));
    assert_eq!(app.finanzas_filter.tipo, None);

    // Borrar con confirmación.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.modal.is_some());
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(count(&app, "SELECT COUNT(*) FROM financiamiento"), 0);
}

#[test]
fn budget_form_persists_and_shows_status() {
    let (mut app, _dir) = test_app();
    open_financiamiento(&mut app);
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.current_screen(), Screen::PresupuestoForm);
    press_ctrl(&mut app, 's');
    assert_eq!(app.current_screen(), Screen::FinanciamientoList);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM presupuesto_mensual"), 1);

    let text = render_con(&mut app, 130, 36);
    assert!(text.contains("Presupuesto"), "falta el presupuesto: {text}");
    assert!(text.contains("Dentro del presupuesto"));
}

#[test]
fn empty_state_is_rendered() {
    let (mut app, _dir) = test_app();
    open_financiamiento(&mut app);
    let text = render_con(&mut app, 130, 36);
    assert!(text.contains("No hay transacciones"));
    assert!(text.contains("Sin presupuesto mensual"));
}
