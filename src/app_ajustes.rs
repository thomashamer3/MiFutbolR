//! Formulario y acciones del módulo Ajustes (§18).

use std::path::PathBuf;

use anyhow::Result;

use crate::app::{APP_NAME, App, ConfirmAction, Field, Form, Modal, Screen};
use crate::domain::models::{AppSettings, Language, Mode, TextSize, Theme};
use crate::domain::validation::DomainError;
use crate::services::botin_service;
use crate::services::legacy_import_service;

impl App {
    /// Abre el formulario de ajustes.
    pub fn open_ajustes_form(&mut self) -> Result<()> {
        self.form = Some(self.ajustes_form());
        self.push_screen(Screen::AjustesForm);
        self.status = format!(
            "BD: {}",
            self.db.path().map_or_else(
                || "en memoria".to_string(),
                |path| path.display().to_string(),
            )
        );
        Ok(())
    }

    fn ajustes_form(&self) -> Form {
        let botines = botin_service::listar(self.db.conn(), &Default::default())
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row.id.to_string(), row.nombre))
            .collect::<Vec<_>>();
        let mut opciones_botin = vec![("0".to_string(), "Ninguno".to_string())];
        opciones_botin.extend(botines);
        let botin_actual = self.settings.botin_predeterminado.to_string();

        Form::new(
            "Ajustes",
            vec![
                Field::select(
                    "theme",
                    "Tema",
                    self.settings.theme.as_i64().to_string(),
                    Theme::ALL
                        .iter()
                        .map(|theme| (theme.as_i64().to_string(), theme.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "language",
                    "Idioma",
                    self.settings.language.as_i64().to_string(),
                    Language::ALL
                        .iter()
                        .map(|language| {
                            (language.as_i64().to_string(), language.label().to_string())
                        })
                        .collect(),
                ),
                Field::select(
                    "mode",
                    "Modo de menú",
                    self.settings.mode.as_i64().to_string(),
                    Mode::ALL
                        .iter()
                        .map(|mode| (mode.as_i64().to_string(), mode.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "text_size",
                    "Tamaño de texto",
                    self.settings.text_size.as_i64().to_string(),
                    TextSize::ALL
                        .iter()
                        .map(|size| (size.as_i64().to_string(), size.label().to_string()))
                        .collect(),
                ),
                Field::text(
                    "image_viewer",
                    "Visor de imágenes",
                    self.settings.image_viewer.clone(),
                    255,
                ),
                Field::select(
                    "botin",
                    "Botín predeterminado",
                    botin_actual,
                    opciones_botin,
                ),
            ],
        )
    }

    /// Guarda los ajustes y aplica el tema en vivo.
    pub(crate) fn save_ajustes_form(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let settings = match self.ajustes_desde_form() {
            Ok(settings) => settings,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        match self.db.save_settings(&settings) {
            Ok(()) => {
                self.settings = settings;
                self.form = None;
                self.stack.pop();
                self.status = "Ajustes guardados".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn ajustes_desde_form(&self) -> Result<AppSettings, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let codigo = |key: &str| -> Result<i64, DomainError> {
            get(key)
                .parse()
                .map_err(|_| DomainError::Message(format!("Valor inválido en {key}")))
        };
        let botin = codigo("botin")?;
        Ok(AppSettings {
            theme: Theme::try_from(codigo("theme")?)?,
            language: Language::try_from(codigo("language")?)?,
            mode: Mode::try_from(codigo("mode")?)?,
            text_size: TextSize::try_from(codigo("text_size")?)?,
            image_viewer: get("image_viewer"),
            botin_predeterminado: botin,
        })
    }

    /// Abre el formulario de importación legacy.
    pub fn open_legacy_import_form(&mut self) -> Result<()> {
        self.form = Some(Form::new(
            "Importar base legacy",
            vec![Field::text("ruta", "Archivo .db", String::new(), 260).required()],
        ));
        self.push_screen(Screen::LegacyImportForm);
        self.status = "Importación legacy: indique la ruta del archivo .db".to_string();
        Ok(())
    }

    /// Importa la base legacy indicada en el formulario.
    pub(crate) fn save_legacy_import(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let ruta = form.value("ruta").unwrap_or("").trim().to_string();
        let resultado =
            legacy_import_service::importar(self.db.conn_mut(), std::path::Path::new(&ruta));
        match resultado {
            Ok(resumen) => {
                self.form = None;
                self.stack.pop();
                let detalle = resumen
                    .tablas
                    .iter()
                    .map(|(tabla, filas)| format!("{tabla}: {filas}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.status = format!("Importadas {} filas", resumen.total());
                self.modal = Some(Modal::Info(format!(
                    "Importación completada: {} filas.\n\n{}",
                    resumen.total(),
                    detalle
                )));
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }

    /// Crea un backup de la base en `Exportaciones/`.
    pub fn backup_ajustes(&mut self) -> Result<()> {
        let destino = self.exportaciones_dir().join(format!(
            "backup_{}.db",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        match self.db.backup(&destino) {
            Ok(()) => {
                let ruta = destino.display().to_string();
                self.status = format!("Backup creado: {ruta}");
                self.modal = Some(Modal::Info(format!("Backup creado en:\n{ruta}")));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Exporta una copia de la base.
    pub fn exportar_db_ajustes(&mut self) -> Result<()> {
        let destino = self.exportaciones_dir().join(format!(
            "mifutbol_export_{}.db",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        match self.db.backup(&destino) {
            Ok(()) => {
                let ruta = destino.display().to_string();
                self.status = format!("BD exportada: {ruta}");
                self.modal = Some(Modal::Info(format!("Base exportada en:\n{ruta}")));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Comprueba la integridad de la base.
    pub fn integridad_ajustes(&mut self) -> Result<()> {
        match self.db.integrity() {
            Ok(resumen) => {
                self.status = "Integridad verificada".to_string();
                self.modal = Some(Modal::Info(resumen));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Pide confirmación para restaurar el backup más reciente.
    pub fn confirmar_restauracion(&mut self) {
        let Some(ruta) = self.backup_mas_reciente() else {
            self.status = "No hay backups en Exportaciones/".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Restaurar backup",
            format!(
                "Se reemplazará la base actual con {}. (S = sí, N = no)",
                ruta.file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
            ),
            ConfirmAction::RestoreBackup,
        ));
    }

    /// Restaura el backup más reciente sobre la conexión viva.
    pub fn restaurar_backup(&mut self) -> Result<()> {
        let Some(ruta) = self.backup_mas_reciente() else {
            self.status = "No hay backups en Exportaciones/".to_string();
            return Ok(());
        };
        // Backup de seguridad antes de restaurar.
        let seguridad = self.exportaciones_dir().join(format!(
            "pre_restore_{}.db",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        let _ = self.db.backup(&seguridad);
        match self.db.restore_from(&ruta) {
            Ok(()) => {
                self.settings = self.db.settings()?;
                self.status = format!("Backup restaurado: {}", ruta.display());
                self.modal = Some(Modal::Info("Backup restaurado correctamente.".to_string()));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn backup_mas_reciente(&self) -> Option<PathBuf> {
        let dir = self.exportaciones_dir();
        let mut archivos: Vec<PathBuf> = std::fs::read_dir(&dir)
            .ok()?
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|ruta| {
                ruta.extension().is_some_and(|ext| ext == "db")
                    && ruta
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("backup_"))
            })
            .collect();
        archivos.sort();
        archivos.pop()
    }

    fn exportaciones_dir(&self) -> PathBuf {
        self.paths.export_dir.clone()
    }

    /// Texto "Acerca de".
    #[must_use]
    pub fn acerca_de(&self) -> String {
        format!("{APP_NAME} 0.1.0 · Rust + ratatui + rusqlite")
    }
}
