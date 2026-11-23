//! Bucle de eventos: se dibuja una vez por ráfaga y no se redibuja sin cambios.

mod common;

use std::collections::VecDeque;

use common::*;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use mifutbol_rs::app::{EVENTOS_POR_FRAME, procesar_eventos};

fn tecla(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn mouse(kind: MouseEventKind) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
}

/// Fuente de eventos simulada: devuelve `None` cuando se vacía.
fn cola(eventos: Vec<Event>) -> VecDeque<Event> {
    eventos.into()
}

#[test]
fn mouse_moves_alone_do_not_request_a_frame() {
    let (mut app, _dir) = test_app();
    let mut pendientes = cola(vec![mouse(MouseEventKind::Moved); 10]);
    let cambio = procesar_eventos(&mut app, || Ok(pendientes.pop_front())).unwrap();
    assert!(!cambio);
    assert!(
        pendientes.is_empty(),
        "se consumieron todos los movimientos"
    );
}

#[test]
fn burst_is_processed_before_drawing_once() {
    let (mut app, _dir) = test_app();
    let antes = app.menu_state.selected().unwrap_or(0);
    let mut pendientes = cola(vec![
        mouse(MouseEventKind::Moved),
        tecla(KeyCode::Down),
        mouse(MouseEventKind::Moved),
        tecla(KeyCode::Down),
        mouse(MouseEventKind::Up(MouseButton::Left)),
    ]);
    let cambio = procesar_eventos(&mut app, || Ok(pendientes.pop_front())).unwrap();
    assert!(cambio);
    assert!(pendientes.is_empty());
    assert_eq!(app.menu_state.selected(), Some(antes + 2));
}

#[test]
fn endless_stream_is_capped_per_frame() {
    let (mut app, _dir) = test_app();
    let mut leidos = 0;
    let cambio = procesar_eventos(&mut app, || {
        leidos += 1;
        Ok(Some(mouse(MouseEventKind::ScrollDown)))
    })
    .unwrap();
    assert!(cambio);
    assert_eq!(leidos, EVENTOS_POR_FRAME);
}

#[test]
fn quitting_stops_processing() {
    let (mut app, _dir) = test_app_con_bienvenida();
    // En la portada `q` pide confirmar (como en el menú); `s` confirma.
    let mut pendientes = cola(vec![
        tecla(KeyCode::Char('q')),
        tecla(KeyCode::Char('s')),
        tecla(KeyCode::Char('n')),
    ]);
    procesar_eventos(&mut app, || Ok(pendientes.pop_front())).unwrap();
    assert!(app.should_quit);
    assert_eq!(pendientes.len(), 1, "no se procesa nada después de salir");
}
