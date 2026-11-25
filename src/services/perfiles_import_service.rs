//! Importación de los perfiles de MiFutbolC: `users.db` (tabla `local_users`)
//! más un `mifutbol_<usuario>.db` por perfil en el mismo directorio.
//!
//! Cada perfil se convierte en una cuenta de `cuentas.db` con su propia base en
//! `mifutbolr_<usuario>.db`. Los archivos de MiFutbolC se abren en solo lectura.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::config::AppPaths;
use crate::db::Database;
use crate::services::legacy_import_service::{self, ImportError};
use crate::services::usuario_service;

/// Perfil importado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfilImportado {
    /// Nombre de la cuenta creada.
    pub nombre: String,
    /// ¿Tenía contraseña?
    pub con_password: bool,
    /// Filas copiadas de su base (0 si no tenía).
    pub filas: usize,
}

/// Resultado de la importación de perfiles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumenPerfiles {
    /// Cuentas creadas.
    pub perfiles: Vec<PerfilImportado>,
    /// Perfiles omitidos o importados con problemas.
    pub advertencias: Vec<String>,
}

/// Directorio de datos de MiFutbolC (`%LOCALAPPDATA%\MiFutbolC\data` o `./data`).
#[must_use]
pub fn directorio_c_por_defecto() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join("MiFutbolC").join("data"))
    } else {
        Some(PathBuf::from("./data"))
    }
}

/// Importa todos los perfiles de `data_dir_c`. Un perfil con problemas no
/// detiene a los demás: queda en `advertencias`.
pub fn importar_perfiles(
    cuentas: &Connection,
    paths: &AppPaths,
    data_dir_c: &Path,
) -> Result<ResumenPerfiles, ImportError> {
    let users = data_dir_c.join("users.db");
    let mut resumen = ResumenPerfiles::default();
    if !users.is_file() {
        let compartida = data_dir_c.join("mifutbol.db");
        if compartida.is_file() {
            resumen.advertencias.push(format!(
                "No hay users.db en {}. Para la base sin perfiles usá: mifutbol-rs --importar-legacy \"{}\"",
                data_dir_c.display(),
                compartida.display()
            ));
            return Ok(resumen);
        }
        return Err(ImportError::NoExiste(users.display().to_string()));
    }

    let origen = Connection::open_with_flags(&users, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let usuarios: Vec<(String, String, String)> = origen
        .prepare(
            "SELECT username, COALESCE(password_salt, ''), COALESCE(password_hash, '')
               FROM local_users ORDER BY id",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(origen);

    for (nombre, salt, hash) in usuarios {
        let con_password = !hash.is_empty();
        let almacenado = if con_password {
            usuario_service::hash_mifutbolc(&salt, &hash)
        } else {
            String::new()
        };
        let cuenta =
            match usuario_service::importar_cuenta(cuentas, &nombre, &almacenado, con_password) {
                Ok(cuenta) => cuenta,
                Err(error) => {
                    resumen
                        .advertencias
                        .push(format!("{nombre}: omitido ({error})"));
                    continue;
                }
            };

        let datos = data_dir_c.join(format!("mifutbol_{nombre}.db"));
        let mut filas = 0;
        if datos.is_file() {
            match importar_datos(&paths.db_usuario(&cuenta.nombre), &datos) {
                Ok(importado) => {
                    filas = importado.total();
                    resumen.advertencias.extend(
                        importado
                            .advertencias
                            .into_iter()
                            .map(|aviso| format!("{nombre}: {aviso}")),
                    );
                }
                Err(error) => resumen
                    .advertencias
                    .push(format!("{nombre}: cuenta creada sin datos ({error})")),
            }
        }
        resumen.perfiles.push(PerfilImportado {
            nombre: cuenta.nombre,
            con_password,
            filas,
        });
    }
    Ok(resumen)
}

fn importar_datos(
    destino: &Path,
    origen: &Path,
) -> Result<legacy_import_service::ImportResumen, ImportError> {
    let mut db = Database::open(destino).map_err(|error| ImportError::Io(error.to_string()))?;
    legacy_import_service::importar(db.conn_mut(), origen)
}
