//! Punto de entrada: rutas, base de datos, terminal y event loop (§Anexo B).
//!
//! Opciones de línea de comandos:
//! - `--importar-legacy <ruta.db>`: importa una base de MiFutbolC a la BD actual.
//! - `--version`: muestra la versión.
//! - `--help`: ayuda.

use std::path::Path;

use anyhow::Context;
use mifutbol_rs::app::{self, App};
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::services::legacy_import_service;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let paths = AppPaths::discover()?;
    paths.ensure_dirs().with_context(|| {
        format!(
            "No se pudieron crear los directorios de {}",
            paths.data_dir.display()
        )
    })?;
    let _ = mifutbol_rs::util::logging::init(&paths.log_file);

    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("MiFutbolRS {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            println!(
                "MiFutbolRS {}\n\nUso:\n  mifutbol-rs                       Abre la TUI\n  mifutbol-rs --importar-legacy <ruta.db>  Importa una BD legacy\n  mifutbol-rs --version             Muestra la versión\n  mifutbol-rs --help                Muestra esta ayuda\n\nBase de datos: {}\nVariable MIFUTBOL_DB para usar otra ruta.",
                env!("CARGO_PKG_VERSION"),
                paths.db_file.display()
            );
            return Ok(());
        }
        Some("--importar-legacy") => {
            let ruta = args.get(1).context(
                "Falta la ruta de la base legacy. Uso: mifutbol-rs --importar-legacy <ruta.db>",
            )?;
            let mut db = Database::open(&paths.db_file)
                .with_context(|| format!("No se pudo abrir {}", paths.db_file.display()))?;
            let resumen = legacy_import_service::importar(db.conn_mut(), Path::new(ruta))?;
            println!(
                "Importación completada en {}: {} filas.",
                paths.db_file.display(),
                resumen.total()
            );
            for (tabla, filas) in &resumen.tablas {
                println!("  {tabla}: {filas}");
            }
            for aviso in &resumen.advertencias {
                println!("  aviso: {aviso}");
            }
            return Ok(());
        }
        Some(otra) => {
            anyhow::bail!("Opción desconocida: {otra}. Usá --help para ver las opciones.");
        }
        None => {}
    }

    let db = Database::open(&paths.db_file)
        .with_context(|| format!("No se pudo abrir {}", paths.db_file.display()))?;

    let mut app = App::new(db, paths)?;

    let mut terminal = ratatui::init();
    let result = app::run(&mut terminal, &mut app);
    ratatui::restore();

    if let Err(error) = &result {
        mifutbol_rs::util::logging::error(&format!("{error:#}"));
    }
    result
}
