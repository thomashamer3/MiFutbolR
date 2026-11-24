//! Punto de entrada: rutas, base de datos, terminal y event loop (§Anexo B).
//!
//! Opciones de línea de comandos:
//! - `--importar-legacy <ruta.db>`: importa una base de MiFutbolC a la BD actual.
//! - `--importar-perfiles [dir]`: importa los perfiles de MiFutbolC como cuentas.
//! - `--version`: muestra la versión.
//! - `--help`: ayuda.

use std::path::Path;

use anyhow::Context;
use mifutbol_rs::app::{self, App};
use mifutbol_rs::config::AppPaths;
use mifutbol_rs::db::Database;
use mifutbol_rs::services::{legacy_import_service, perfiles_import_service};

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
            println!("MiFutbolR {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            println!(
                "MiFutbolR {}\n\nUso:\n  mifutbol-rs                       Abre la TUI\n  mifutbol-rs --importar-legacy <ruta.db>  Importa una BD legacy\n  mifutbol-rs --importar-perfiles [dir]    Importa los perfiles de MiFutbolC (users.db)\n  mifutbol-rs --version             Muestra la versión\n  mifutbol-rs --help                Muestra esta ayuda\n\nBase de datos: {}\nVariable MIFUTBOL_DB para usar otra ruta.",
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
            match legacy_import_service::copiar_imagenes(Path::new(ruta), &paths.images_dir) {
                Ok(copiadas) => println!(
                    "Imágenes copiadas a {}: {copiadas}",
                    paths.images_dir.display()
                ),
                Err(error) => println!("  aviso: no se pudieron copiar las imágenes: {error}"),
            }
            return Ok(());
        }
        Some("--importar-perfiles") => {
            let dir = match args.get(1) {
                Some(dir) => std::path::PathBuf::from(dir),
                None => perfiles_import_service::directorio_c_por_defecto()
                    .context("No se encontró el directorio de MiFutbolC; pasalo como argumento")?,
            };
            let cuentas = Database::open_cuentas(&paths.cuentas_file)
                .with_context(|| format!("No se pudo abrir {}", paths.cuentas_file.display()))?;
            let resumen = perfiles_import_service::importar_perfiles(cuentas.conn(), &paths, &dir)?;
            println!(
                "Perfiles importados desde {}: {}",
                dir.display(),
                resumen.perfiles.len()
            );
            for perfil in &resumen.perfiles {
                let clave = if perfil.con_password {
                    "con contraseña"
                } else {
                    "sin contraseña"
                };
                println!("  {} ({clave}): {} filas", perfil.nombre, perfil.filas);
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
    // Backup de inicio, aviso de actualización y de Modo Retro (como `main.c`).
    app.tareas_inicio = true;
    app.abrir_con_sistema = true;
    app.url_actualizaciones =
        Some(mifutbol_rs::services::actualizacion_service::URL_ULTIMA_RELEASE.to_string());
    // El binario siempre arranca en la portada de inicio de sesión.
    app.mostrar_inicio_sesion();

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let result = app::run(&mut terminal, &mut app);
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    if let Err(error) = &result {
        mifutbol_rs::util::logging::error(&format!("{error:#}"));
    }
    result
}
