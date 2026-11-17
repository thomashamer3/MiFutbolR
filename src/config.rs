//! Rutas de datos, exportación e importación (§20.9).

use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

/// Rutas de la aplicación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Directorio de datos local.
    pub data_dir: PathBuf,
    /// Archivo SQLite (sesión de invitado).
    pub db_file: PathBuf,
    /// Registro de cuentas locales.
    pub cuentas_file: PathBuf,
    /// Exportaciones.
    pub export_dir: PathBuf,
    /// Importaciones.
    pub import_dir: PathBuf,
    /// Imágenes.
    pub images_dir: PathBuf,
    /// Log de la aplicación.
    pub log_file: PathBuf,
}

impl AppPaths {
    /// Resuelve las rutas del sistema operativo.
    ///
    /// `MIFUTBOL_DB` permite apuntar a otra base de datos (tests, backup).
    pub fn discover() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "mifutbol", "mifutbol-rs")
            .context("No se pudo resolver el directorio de datos")?;
        let data_dir = dirs.data_local_dir().to_path_buf();
        let db_file = std::env::var_os("MIFUTBOL_DB")
            .map_or_else(|| data_dir.join("mifutbol.db"), PathBuf::from);
        Ok(Self {
            export_dir: data_dir.join("Exportaciones"),
            import_dir: data_dir.join("Importaciones"),
            images_dir: data_dir.join("Imagenes"),
            log_file: data_dir.join("mifutbol.log"),
            cuentas_file: data_dir.join("cuentas.db"),
            data_dir,
            db_file,
        })
    }

    /// Rutas apoyadas en un directorio temporal (tests manuales).
    pub fn with_root(root: &std::path::Path) -> Self {
        Self {
            data_dir: root.to_path_buf(),
            db_file: root.join("mifutbol.db"),
            export_dir: root.join("Exportaciones"),
            import_dir: root.join("Importaciones"),
            images_dir: root.join("Imagenes"),
            log_file: root.join("mifutbol.log"),
            cuentas_file: root.join("cuentas.db"),
        }
    }

    /// Base de datos propia de una cuenta.
    #[must_use]
    pub fn db_usuario(&self, id: i64) -> PathBuf {
        self.data_dir
            .join("usuarios")
            .join(id.to_string())
            .join("mifutbol.db")
    }

    /// Crea los directorios necesarios.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for dir in [
            &self.data_dir,
            &self.export_dir,
            &self.import_dir,
            &self.images_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_root_is_self_consistent() {
        let root = std::path::Path::new("C:/tmp/mifutbol-test");
        let paths = AppPaths::with_root(root);
        assert_eq!(paths.db_file, root.join("mifutbol.db"));
        assert_eq!(paths.export_dir, root.join("Exportaciones"));
        assert_eq!(paths.cuentas_file, root.join("cuentas.db"));
        assert_eq!(
            paths.db_usuario(7),
            root.join("usuarios").join("7").join("mifutbol.db")
        );
    }
}
