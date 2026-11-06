//! Genera capturas de la TUI real en JSON para el README.
//!
//! Uso: `cargo run --example capturar_tui` (requiere `datos/mifutbol_convertida.db`).
//! Después `python scripts/render_capturas.py` convierte los JSON a PNG.

use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mifutbol_rs::app::App;
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::domain::models::Theme;
use mifutbol_rs::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use serde::Serialize;

const ANCHO: u16 = 120;
const ALTO: u16 = 36;

#[derive(Serialize)]
struct Celda {
    s: String,
    fg: [u8; 3],
    bg: [u8; 3],
    bold: bool,
    dim: bool,
}

#[derive(Serialize)]
struct Captura {
    width: u16,
    height: u16,
    celdas: Vec<Celda>,
}

fn color(c: Color) -> [u8; 3] {
    match c {
        Color::Reset => [12, 12, 12],
        Color::Black => [12, 12, 12],
        Color::Red => [197, 15, 31],
        Color::Green => [19, 161, 14],
        Color::Yellow => [193, 156, 0],
        Color::Blue => [0, 55, 218],
        Color::Magenta => [136, 23, 152],
        Color::Cyan => [58, 150, 221],
        Color::Gray => [204, 204, 204],
        Color::DarkGray => [118, 118, 118],
        Color::LightRed => [231, 72, 86],
        Color::LightGreen => [22, 198, 12],
        Color::LightYellow => [249, 241, 165],
        Color::LightBlue => [59, 120, 255],
        Color::LightMagenta => [180, 0, 158],
        Color::LightCyan => [97, 214, 214],
        Color::White => [242, 242, 242],
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(i) => match i {
            0 => [12, 12, 12],
            1 => [197, 15, 31],
            2 => [19, 161, 14],
            3 => [193, 156, 0],
            4 => [0, 55, 218],
            5 => [136, 23, 152],
            6 => [58, 150, 221],
            7 => [204, 204, 204],
            _ => [160, 160, 160],
        },
    }
}

fn nueva_app() -> anyhow::Result<App> {
    let db = Database::open(&PathBuf::from("target/capturas.db"))?;
    let paths = AppPaths::with_root(std::path::Path::new("target/capturas_paths"));
    let mut app = App::new(db, paths)?;
    app.settings.theme = Theme::Dark;
    app.status = "Listo".to_string();
    Ok(app)
}

fn pulsar(app: &mut App, code: KeyCode) -> anyhow::Result<()> {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))?;
    Ok(())
}

fn abrir_modulo(app: &mut App, indice: usize) -> anyhow::Result<()> {
    app.menu_state.select(Some(indice));
    pulsar(app, KeyCode::Enter)
}

fn guardar(app: &mut App, nombre: &str) -> anyhow::Result<()> {
    let backend = TestBackend::new(ANCHO, ALTO);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::draw(frame, app))?;
    let buffer = terminal.backend().buffer();
    let mut celdas = Vec::with_capacity((ANCHO as usize) * (ALTO as usize));
    for y in 0..ALTO {
        for x in 0..ANCHO {
            let celda = &buffer[(x, y)];
            celdas.push(Celda {
                s: celda.symbol().to_string(),
                fg: color(celda.fg),
                bg: color(celda.bg),
                bold: celda.modifier.contains(Modifier::BOLD),
                dim: celda.modifier.contains(Modifier::DIM),
            });
        }
    }
    let captura = Captura {
        width: ANCHO,
        height: ALTO,
        celdas,
    };
    fs::create_dir_all("target/capturas")?;
    fs::write(
        format!("target/capturas/{nombre}.json"),
        serde_json::to_vec(&captura)?,
    )?;
    println!("captura {nombre}");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    fs::create_dir_all("target/capturas")?;
    for sufijo in ["-wal", "-shm"] {
        let _ = fs::remove_file(format!("target/capturas.db{sufijo}"));
    }
    fs::copy("datos/mifutbol_convertida.db", "target/capturas.db")
        .context("copiá datos/mifutbol_convertida.db")?;

    let mut app = nueva_app()?;
    guardar(&mut app, "menu")?;

    let mut app = nueva_app()?;
    abrir_modulo(&mut app, 4)?;
    pulsar(&mut app, KeyCode::Down)?;
    guardar(&mut app, "partidos")?;

    let mut app = nueva_app()?;
    abrir_modulo(&mut app, 6)?;
    app.estadisticas_hub.select(Some(0));
    pulsar(&mut app, KeyCode::Enter)?;
    guardar(&mut app, "estadisticas")?;

    let mut app = nueva_app()?;
    abrir_modulo(&mut app, 7)?;
    guardar(&mut app, "financiamiento")?;

    Ok(())
}
