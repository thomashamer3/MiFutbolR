//! Formulario y acciones del módulo Ajustes (§18).

use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    APP_NAME, App, ConfirmAction, Field, Form, Modal, Module, Screen, mover_seleccion,
};
use crate::domain::models::{AppSettings, Language, Mode, TextSize, Theme};
use crate::domain::validation::DomainError;
use crate::i18n::t;
use crate::services::actualizacion_service;
use crate::services::backup_service;
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
        let idioma = self.settings.language;

        Form::new(
            "Ajustes",
            vec![
                Field::select(
                    "theme",
                    t(idioma, "settings_theme", "Tema"),
                    self.settings.theme.as_i64().to_string(),
                    Theme::ALL
                        .iter()
                        .map(|theme| (theme.as_i64().to_string(), theme.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "language",
                    t(idioma, "settings_language", "Idioma"),
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
                    t(idioma, "settings_mode", "Modo de menú"),
                    self.settings.mode.as_i64().to_string(),
                    Mode::ALL
                        .iter()
                        .map(|mode| (mode.as_i64().to_string(), mode.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "text_size",
                    t(idioma, "settings_text_size", "Tamaño de texto"),
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
                Field::boolean(
                    "actualizaciones",
                    "Buscar actualizaciones al iniciar",
                    self.settings.buscar_actualizaciones,
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
            buscar_actualizaciones: get("actualizaciones") == "Sí",
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

    /// Exporta el informe PDF (Ctrl+P en Ajustes).
    pub fn exportar_pdf_ajustes(&mut self) {
        let usuario = self.usuario_actual.as_ref().map(|u| u.nombre.clone());
        let dir = self.exportaciones_dir();
        match crate::services::informe_service::exportar_pdf(
            self.db.conn(),
            usuario.as_deref(),
            &dir,
        ) {
            Ok(ruta) => self.status = format!("Informe PDF: {}", ruta.display()),
            Err(error) => self.modal = Some(Modal::Error(error.0)),
        }
    }

    /// Exporta partidos, canchas, camisetas, lesiones y finanzas a ODS (Ctrl+O en Ajustes).
    pub fn exportar_ods_ajustes(&mut self) {
        let dir = self.exportaciones_dir();
        match crate::services::informe_service::exportar_ods(self.db.conn(), &dir) {
            Ok(ruta) => self.status = format!("Planilla ODS: {}", ruta.display()),
            Err(error) => self.modal = Some(Modal::Error(error.0)),
        }
    }

    /// VACUUM + ANALYZE + integridad; antes guarda un backup como hace C
    /// (`backup_base_datos_automatico("antes de VACUUM")`).
    pub(crate) fn mantenimiento_ajustes(&mut self) {
        let previo = self.exportaciones_dir().join(format!(
            "backup_{}_pre_vacuum.db",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        if let Err(error) = self.db.backup(&previo) {
            self.modal = Some(Modal::Error(format!(
                "No se pudo crear el backup previo; no se hizo el mantenimiento: {error}"
            )));
            return;
        }
        match self.db.mantenimiento() {
            Ok(resumen) => {
                self.status = "Mantenimiento completado".to_string();
                self.modal = Some(Modal::Info(format!(
                    "{resumen}\nBackup previo: {}",
                    previo.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Exporta todas las tablas a JSON y los listados principales a Markdown
    /// (Ctrl+X en Ajustes; "Exportar todo" de C).
    pub fn exportar_todo_ajustes(&mut self) {
        let dir = self.exportaciones_dir();
        match crate::services::informe_service::exportar_todo(self.db.conn(), &dir) {
            Ok((json, md)) => {
                self.status = format!("Exportado: {}", json.display());
                self.modal = Some(Modal::Info(format!(
                    "Exportación completa:\n{}\n{}",
                    json.display(),
                    md.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.0)),
        }
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
        self.restaurar_backup_desde(&ruta);
        Ok(())
    }

    /// Restaura `ruta` sobre la conexión viva, con copia de seguridad previa.
    pub(crate) fn restaurar_backup_desde(&mut self, ruta: &Path) {
        // Backup de seguridad antes de restaurar.
        let seguridad = self.exportaciones_dir().join(format!(
            "pre_restore_{}.db",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        let sin_copia = self
            .db
            .backup(&seguridad)
            .err()
            .map(|error| format!("\nNo se pudo hacer la copia previa: {error}"))
            .unwrap_or_default();
        match self.db.restore_from(ruta) {
            Ok(()) => {
                if let Ok(settings) = self.db.settings() {
                    self.settings = settings;
                }
                self.menu_ocultos = crate::db::settings_repo::menus_ocultos(self.db.conn());
                // El backup API no cuenta en `total_changes()`: marcar el Inicio a mano.
                self.inicio_sucio = true;
                self.status = format!("Backup restaurado: {}", ruta.display());
                self.modal = Some(Modal::Info(format!(
                    "Backup restaurado correctamente.{sin_copia}"
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Abre la administración de backups (`menu_backup_restore` de C).
    pub(crate) fn open_backups(&mut self) {
        self.refresh_backups();
        self.push_screen(Screen::BackupsList);
    }

    /// Relee los `backup_*.db` de Exportaciones/.
    pub(crate) fn refresh_backups(&mut self) {
        self.backups = backup_service::listar(&self.exportaciones_dir());
        let total = self.backups.len();
        let actual = self.backups_table.selected().unwrap_or(0);
        self.backups_table
            .select((total > 0).then(|| actual.min(total - 1)));
    }

    /// Teclas de la lista de backups.
    pub(crate) fn handle_backups_key(&mut self, key: KeyEvent) -> Result<bool> {
        let seleccion = self
            .backups_table
            .selected()
            .and_then(|indice| self.backups.get(indice).map(|b| (indice, b.nombre.clone())));
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.backups_table, self.backups.len(), 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.backups_table, self.backups.len(), -1);
            }
            KeyCode::Enter => {
                if let Some((indice, nombre)) = seleccion {
                    self.modal = Some(Modal::confirm(
                        "Restaurar backup",
                        format!("Se reemplazará la base actual con {nombre}. (S = sí, N = no)"),
                        ConfirmAction::RestoreBackupSel(indice),
                    ));
                }
            }
            KeyCode::Char('d') => {
                if let Some((indice, nombre)) = seleccion {
                    self.modal = Some(Modal::confirm(
                        "Eliminar backup",
                        format!("¿Eliminar {nombre}? No se puede deshacer. (S = sí, N = no)"),
                        ConfirmAction::DeleteBackupSel(indice),
                    ));
                }
            }
            KeyCode::Char('b') => {
                self.backup_ajustes()?;
                self.refresh_backups();
            }
            KeyCode::Char('r') => {
                self.refresh_backups();
                self.status = "Backups actualizados".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Al entrar al menú, una vez por sesión: si la base está vacía y hay un
    /// backup con datos, ofrece restaurarlo (`ofrecer_restaurar_backup_inicio`
    /// de MiFutbolC). Solo abre un modal; nunca bloquea.
    pub fn ofrecer_restaurar_backup_inicio(&mut self) {
        if self.restauracion_ofrecida
            || self.modal.is_some()
            || self.current_screen() != Screen::MainMenu
        {
            return;
        }
        self.restauracion_ofrecida = true;
        if crate::db::tiene_datos_usuario(self.db.conn()) {
            return;
        }
        let Some(ruta) = self.backup_mas_reciente() else {
            return;
        };
        let Ok(copia) = rusqlite::Connection::open_with_flags(
            &ruta,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) else {
            return;
        };
        if !crate::db::tiene_datos_usuario(&copia) {
            return;
        }
        let partidos: i64 = copia
            .query_row("SELECT COUNT(*) FROM partido", [], |row| row.get(0))
            .unwrap_or(0);
        self.modal = Some(Modal::confirm(
            "¿Restaurar backup?",
            format!(
                "Se encontró una copia de seguridad con {partidos} partidos en:\n{}\n¿Restaurarla? (S = sí, N = no)",
                ruta.display()
            ),
            ConfirmAction::RestoreBackup,
        ));
    }

    fn backup_mas_reciente(&self) -> Option<PathBuf> {
        backup_service::listar(&self.exportaciones_dir())
            .into_iter()
            .next()
            .map(|backup| backup.ruta)
    }

    fn exportaciones_dir(&self) -> PathBuf {
        self.paths.export_dir.clone()
    }

    /// Texto "Acerca de".
    #[must_use]
    pub fn acerca_de(&self) -> String {
        format!(
            "{APP_NAME} {} · Rust + ratatui + rusqlite",
            actualizacion_service::VERSION_ACTUAL
        )
    }

    /// Lo que `main.c` hace al arrancar, una vez por base al entrar al menú:
    /// backup de inicio, comprobar actualizaciones (una vez por día) y el aviso
    /// de Modo Retro. Solo si el binario lo activó (`tareas_inicio`).
    pub fn tareas_de_inicio(&mut self) {
        if !self.tareas_inicio || self.tareas_inicio_hechas {
            return;
        }
        self.tareas_inicio_hechas = true;
        self.backup_de_inicio();
        if self.settings.buscar_actualizaciones {
            let marca = self
                .paths
                .data_dir
                .join(actualizacion_service::ARCHIVO_MARCA);
            if actualizacion_service::corresponde_verificar(
                &marca,
                chrono::Local::now().timestamp(),
            ) {
                self.buscar_actualizaciones(false);
            }
        }
        self.avisar_modo_retro();
    }

    /// `backup_al_iniciar` de C: una copia por día (los últimos 7) por perfil.
    fn backup_de_inicio(&mut self) {
        if self.db.path().is_none() {
            return;
        }
        let etiqueta = self
            .usuario_actual
            .as_ref()
            .map_or_else(|| "inicio".to_string(), |u| format!("inicio_u{}", u.id));
        if let Err(error) = backup_service::backup_inicio(
            &self.db,
            &self.paths.export_dir,
            &etiqueta,
            chrono::Local::now().naive_local(),
        ) {
            self.status = format!("No se pudo hacer el backup de inicio: {error}");
        }
    }

    /// `carrera_notificar_modo_retro_inicio` de C: si hoy hay recuerdos, ofrece
    /// abrir el Modo Retro (modal, nunca bloquea).
    fn avisar_modo_retro(&mut self) {
        if self.modal.is_some() {
            return;
        }
        let cantidad =
            crate::services::carrera_service::retro(self.db.conn()).map_or(0, |r| r.len());
        if cantidad > 0 {
            self.modal = Some(Modal::confirm(
                "Modo Retro",
                format!(
                    "Hoy en tu historia hay {cantidad} recuerdo(s).\n¿Querés abrir Modo Retro ahora? (S = sí, N = no)"
                ),
                ConfirmAction::AbrirModoRetro,
            ));
        }
    }

    /// Lanza la comprobación en segundo plano (`manual` = Ajustes → Ctrl+U).
    pub(crate) fn buscar_actualizaciones(&mut self, manual: bool) {
        let Some(url) = self.url_actualizaciones.clone() else {
            if manual {
                self.status = "Comprobar actualizaciones no está disponible".to_string();
            }
            return;
        };
        if self.actualizacion.lanzar(url, manual) {
            if manual {
                self.status = "Buscando actualizaciones…".to_string();
            }
        } else if manual {
            self.status = "Ya se están buscando actualizaciones".to_string();
        }
    }

    /// Recoge el resultado (bucle de eventos). Devuelve si cambió algo.
    pub fn tick_actualizacion(&mut self) -> bool {
        let Some((manual, resultado)) = self.actualizacion.recibir() else {
            return false;
        };
        self.aplicar_actualizacion(manual, resultado);
        true
    }

    /// Espera la comprobación (solo tests).
    pub fn esperar_actualizacion(&mut self, limite: std::time::Duration) -> bool {
        let Some((manual, resultado)) = self.actualizacion.esperar(limite) else {
            return false;
        };
        self.aplicar_actualizacion(manual, resultado);
        true
    }

    /// Como C: la automática solo avisa si hay versión nueva; la manual siempre.
    fn aplicar_actualizacion(
        &mut self,
        manual: bool,
        resultado: Result<actualizacion_service::Release, String>,
    ) {
        match resultado {
            Ok(release) => {
                let (nueva, texto) =
                    actualizacion_service::mensaje(actualizacion_service::VERSION_ACTUAL, &release);
                if nueva {
                    self.status = format!("Nueva versión disponible: {}", release.version);
                }
                if (nueva || manual) && self.modal.is_none() {
                    self.modal = Some(Modal::Info(texto));
                } else if manual {
                    self.status = texto;
                }
            }
            Err(error) if manual => self.modal = Some(Modal::Error(error)),
            Err(_) => {}
        }
    }

    /// "Ver configuración actual" (`show_current_settings` de C). Nunca muestra
    /// contraseñas ni el token de sincronización.
    pub(crate) fn configuracion_actual(&self) -> String {
        let s = &self.settings;
        let botin = if s.botin_predeterminado == 0 {
            "Ninguno".to_string()
        } else {
            botin_service::listar(self.db.conn(), &Default::default())
                .unwrap_or_default()
                .into_iter()
                .find(|b| b.id == s.botin_predeterminado)
                .map_or_else(|| s.botin_predeterminado.to_string(), |b| b.nombre)
        };
        let ocultos = if self.menu_ocultos.is_empty() {
            "ninguno".to_string()
        } else {
            self.menu_ocultos.join(", ")
        };
        let visor = if s.image_viewer.is_empty() {
            "predeterminado del sistema"
        } else {
            s.image_viewer.as_str()
        };
        format!(
            "Tema: {}\nIdioma: {}\nModo de menú: {}\nTamaño de texto: {}\nVisor de imágenes: {visor}\nBotín predeterminado: {botin}\nBuscar actualizaciones al iniciar: {}\nMenús ocultos (Personalizado): {ocultos}\nUsuario: {}\nBase de datos: {}\nVersión: {}",
            s.theme.label(),
            s.language.label(),
            s.mode.label(),
            s.text_size.label(),
            if s.buscar_actualizaciones {
                "Sí"
            } else {
                "No"
            },
            self.usuario_actual
                .as_ref()
                .map_or("Invitado", |u| u.nombre.as_str()),
            self.db
                .path()
                .map_or_else(|| "en memoria".to_string(), |p| p.display().to_string()),
            actualizacion_service::VERSION_ACTUAL,
        )
    }

    /// `reset_settings_to_defaults` de C: vuelve los ajustes a sus valores por
    /// defecto (los menús personalizados se conservan, como en C).
    pub(crate) fn restablecer_ajustes(&mut self) {
        let settings = AppSettings::default();
        match self.db.save_settings(&settings) {
            Ok(()) => {
                self.settings = settings;
                if self.current_screen() == Screen::AjustesForm {
                    self.form = Some(self.ajustes_form());
                }
                self.status = "Configuración restablecida a valores por defecto".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Opciones del editor de menús: todas menos Ajustes, que siempre se ve.
    #[must_use]
    pub fn menus_editables() -> Vec<Module> {
        Module::ALL
            .into_iter()
            .filter(|module| *module != Module::Ajustes)
            .collect()
    }

    /// Teclas del editor de menús personalizados (`menu_custom_menus` de C).
    pub(crate) fn handle_menus_personalizados_key(&mut self, key: KeyEvent) -> Result<bool> {
        let menus = Self::menus_editables();
        let actual = self.menus_hub.selected();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.menus_hub
                    .select(mover_seleccion(actual, menus.len(), 1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.menus_hub
                    .select(mover_seleccion(actual, menus.len(), -1));
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let Some(module) = actual.and_then(|i| menus.get(i).copied()) else {
                    return Ok(true);
                };
                let visible = self.menu_ocultos.iter().any(|clave| clave == module.key());
                match crate::db::settings_repo::set_menu_visible(
                    self.db.conn(),
                    module.key(),
                    visible,
                ) {
                    Ok(()) => {
                        self.menu_ocultos = crate::db::settings_repo::menus_ocultos(self.db.conn());
                        self.status = format!(
                            "Menú {} {}",
                            module.label(),
                            if visible {
                                "habilitado"
                            } else {
                                "deshabilitado"
                            }
                        );
                    }
                    Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
