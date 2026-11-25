//! Rutas de datos, exportación e importación (§20.9).

use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::{ProjectDirs, UserDirs};

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
        // Windows: %LOCALAPPDATA%\MiFutbolR\data
        let dirs = ProjectDirs::from_path(PathBuf::from("MiFutbolR"))
            .context("No se pudo resolver el directorio de datos")?;
        let data_dir = dirs.data_local_dir().to_path_buf();
        let db_file = std::env::var_os("MIFUTBOL_DB")
            .map_or_else(|| data_dir.join("mifutbol.db"), PathBuf::from);
        // Carpetas que el usuario maneja a mano: Documentos\MiFutbolR\...
        let documentos = UserDirs::new()
            .and_then(|user| user.document_dir().map(|dir| dir.join("MiFutbolR")))
            .unwrap_or_else(|| data_dir.clone());
        Ok(Self {
            export_dir: documentos.join("Exportaciones"),
            import_dir: documentos.join("Importaciones"),
            images_dir: documentos.join("Imagenes"),
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

    /// Base de los adjuntos `Imagenes/...` guardados en la BD (padre de `images_dir`).
    #[must_use]
    pub fn adjuntos_dir(&self) -> &std::path::Path {
        self.images_dir.parent().unwrap_or(&self.data_dir)
    }

    /// Base de datos propia de una cuenta: `mifutbolr_<Usuario>.db`.
    #[must_use]
    pub fn db_usuario(&self, nombre: &str) -> PathBuf {
        // Los nombres no se pueden renombrar, así que el archivo queda fijo.
        let seguro: String = nombre
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect();
        self.data_dir.join(format!("mifutbolr_{seguro}.db"))
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
        assert_eq!(paths.adjuntos_dir(), root);
        assert_eq!(paths.db_usuario("Hamer"), root.join("mifutbolr_Hamer.db"));
        assert_eq!(paths.db_usuario("a/b:c"), root.join("mifutbolr_a_b_c.db"));
    }
}
