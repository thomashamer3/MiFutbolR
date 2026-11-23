//! 25 Sincronización (PocketBase): menú de `menu_sincronizacion` de C, la
//! sincronización en segundo plano y lo que C hace al iniciar (partidos del
//! móvil y auto-sync).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Field, Form, Modal, Screen, mover_seleccion};
use crate::services::{mobile_service, sync_service};

/// Opciones del menú, en el orden de C.
pub const SYNC_OPCIONES: [(&str, &str); 8] = [
    (
        "Configurar servidor",
        "URL local (Wi-Fi) y URL remota (HTTPS).",
    ),
    (
        "Iniciar sesión",
        "Correo y contraseña de PocketBase; se guarda solo el token.",
    ),
    (
        "Sincronizar ahora",
        "Sube la cola y baja los cambios, en segundo plano.",
    ),
    (
        "Subir todos los partidos",
        "Reenvía todo el historial (el uuid evita duplicar).",
    ),
    (
        "Estado y pendientes",
        "Vuelve a leer pendientes, conflictos y última sync.",
    ),
    (
        "Resolver conflictos",
        "Elegí entre la copia local y la del servidor.",
    ),
    (
        "Activar/desactivar auto-sync al iniciar",
        "Sincroniza al entrar al menú.",
    ),
    ("Cerrar sesión", "Borra el token y el cursor de descarga."),
];

impl App {
    /// Abre el menú de sincronización.
    pub(crate) fn open_sincronizacion(&mut self) {
        self.refrescar_sync_estado();
        if !matches!(self.current_screen(), Screen::Sincronizacion) {
            self.push_screen(Screen::Sincronizacion);
        }
    }

    fn refrescar_sync_estado(&mut self) {
        match sync_service::estado(self.db.conn()) {
            Ok(estado) => self.sync_estado = estado,
            Err(error) => self.status = format!("Sincronización: {error}"),
        }
    }

    /// Teclas del menú: flechas + Enter o el número de la opción.
    pub(crate) fn handle_sincronizacion_key(&mut self, key: KeyEvent) -> Result<bool> {
        let actual = self.sync_hub.selected();
        let opcion = match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.sync_hub
                    .select(mover_seleccion(actual, SYNC_OPCIONES.len(), 1));
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.sync_hub
                    .select(mover_seleccion(actual, SYNC_OPCIONES.len(), -1));
                return Ok(true);
            }
            KeyCode::Enter => actual.unwrap_or(0),
            KeyCode::Char(c @ '1'..='8') => c as usize - '1' as usize,
            _ => return Ok(false),
        };
        self.sync_hub.select(Some(opcion));
        self.ejecutar_opcion_sync(opcion);
        Ok(true)
    }

    fn ejecutar_opcion_sync(&mut self, opcion: usize) {
        let cfg = self.sync_estado.config.clone();
        match opcion {
            0 => {
                self.form = Some(Form::new(
                    "Servidor PocketBase",
                    vec![
                        Field::text("url_local", "URL local (Wi-Fi)", cfg.url_local, 500),
                        Field::text("url_remota", "URL remota (HTTPS)", cfg.url_remota, 500),
                    ],
                ));
                self.push_screen(Screen::SyncConfigForm);
            }
            1 => {
                self.form = Some(Form::new(
                    "Iniciar sesión en PocketBase",
                    vec![
                        Field::text("email", "Correo", cfg.email, 255).required(),
                        Field::password("password", "Contraseña", "", 255).required(),
                    ],
                ));
                self.push_screen(Screen::SyncLoginForm);
            }
            2 | 3 => self.lanzar_sync(opcion == 3),
            4 => {
                self.refrescar_sync_estado();
                self.status = "Estado de sincronización actualizado".to_string();
            }
            5 => {
                if self.sync_estado.conflicto.is_some() {
                    self.push_screen(Screen::SyncConflicto);
                } else {
                    self.status = "No hay conflictos pendientes".to_string();
                }
            }
            6 => match sync_service::alternar_auto(self.db.conn()) {
                Ok(activo) => {
                    self.status = format!(
                        "Sincronización automática al iniciar: {}",
                        if activo { "activada" } else { "desactivada" }
                    );
                    self.refrescar_sync_estado();
                }
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            },
            _ => match sync_service::cerrar_sesion(self.db.conn()) {
                Ok(()) => {
                    self.status = "Sesión de sincronización cerrada".to_string();
                    self.refrescar_sync_estado();
                }
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            },
        }
    }

    /// Guarda las URLs.
    pub(crate) fn save_sync_config_form(&mut self) -> Result<()> {
        let Some(form) = self.form.take() else {
            return Ok(());
        };
        let local = form.value("url_local").unwrap_or("").to_string();
        let remota = form.value("url_remota").unwrap_or("").to_string();
        match sync_service::configurar(self.db.conn(), &local, &remota) {
            Ok(()) => {
                self.go_back();
                self.refrescar_sync_estado();
                self.status = "Configuración guardada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error));
            }
        }
        Ok(())
    }

    /// Inicia sesión. Usa la red, pero es una acción explícita y corta como en
    /// C (5 s de conexión como máximo por servidor).
    pub(crate) fn save_sync_login_form(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, mensaje)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(mensaje));
            return Ok(());
        }
        let email = form.value("email").unwrap_or("").to_string();
        let password = form.value("password").unwrap_or("").to_string();
        match sync_service::iniciar_sesion(self.db.conn(), &email, &password) {
            Ok(servidor) => {
                self.go_back();
                self.refrescar_sync_estado();
                self.status = format!("Sesión iniciada contra {servidor}");
            }
            Err(error) => {
                // La contraseña no queda en el formulario si falla.
                if let Some(campo) = form.fields.iter_mut().find(|campo| campo.key == "password") {
                    campo.value.clear();
                }
                self.form = Some(form);
                self.modal = Some(Modal::Error(error));
            }
        }
        Ok(())
    }

    /// Lanza la sincronización en un hilo con su propia conexión. Con la base
    /// en memoria (sin archivo que abrir) corre en el acto.
    pub(crate) fn lanzar_sync(&mut self, todos: bool) {
        let Some(ruta) = self.db.path().map(std::path::Path::to_path_buf) else {
            let resultado = if todos {
                sync_service::subir_todos(self.db.conn_mut())
            } else {
                sync_service::ejecutar(self.db.conn_mut())
            };
            self.aplicar_resultado_sync(resultado);
            return;
        };
        if self.sync.lanzar(ruta, todos) {
            self.sync_mensaje = "Sincronizando en segundo plano…".to_string();
            self.status = self.sync_mensaje.clone();
        } else {
            self.status = "Ya hay una sincronización en curso".to_string();
        }
    }

    fn aplicar_resultado_sync(&mut self, resultado: Result<sync_service::Resultado, String>) {
        self.sync_mensaje = match resultado {
            Ok(resultado) => resultado.resumen(),
            Err(error) => error,
        };
        self.status = self.sync_mensaje.clone();
        self.inicio_sucio = true;
        self.refrescar_sync_estado();
    }

    /// Recoge el resultado del hilo (bucle de eventos). Devuelve si cambió algo.
    pub fn tick_sync(&mut self) -> bool {
        let Some(resultado) = self.sync.recibir() else {
            return false;
        };
        self.aplicar_resultado_sync(resultado);
        true
    }

    /// Espera el hilo de sincronización (solo tests).
    pub fn esperar_sync(&mut self, limite: std::time::Duration) -> bool {
        let Some(resultado) = self.sync.esperar(limite) else {
            return false;
        };
        self.aplicar_resultado_sync(resultado);
        true
    }

    /// Teclas del conflicto: 1 conserva local, 2 conserva el servidor (C).
    pub(crate) fn handle_sync_conflicto_key(&mut self, key: KeyEvent) -> Result<bool> {
        let conservar_local = match key.code {
            KeyCode::Char('1') => true,
            KeyCode::Char('2') => false,
            _ => return Ok(false),
        };
        match sync_service::resolver_conflicto(self.db.conn_mut(), conservar_local) {
            Ok(_) => {
                self.status = "Conflicto resuelto".to_string();
                self.inicio_sucio = true;
                self.refrescar_sync_estado();
                if self.sync_estado.conflicto.is_none() {
                    self.go_back();
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(true)
    }

    /// Lo que C hace al arrancar, una vez por sesión al entrar al menú: importa
    /// `Importaciones/partidos.json` del móvil si está el marcador y lanza la
    /// sincronización PocketBase en segundo plano si está activada.
    pub fn sincronizar_al_iniciar(&mut self) {
        if self.sincronizacion_inicio_hecha {
            return;
        }
        self.sincronizacion_inicio_hecha = true;
        match mobile_service::sincronizar_pendientes(self.db.conn_mut(), &self.paths.import_dir) {
            Ok(Some((importados, destino))) => {
                self.inicio_sucio = true;
                let _ = self.refrescar_inicio();
                self.status = format!(
                    "Móvil: {importados} partidos importados (archivo movido a {})",
                    destino.display()
                );
            }
            Ok(None) => {}
            Err(error) => self.status = format!("Móvil: {error}"),
        }
        let auto = sync_service::Config::cargar(self.db.conn())
            .is_ok_and(|cfg| cfg.auto_sync && cfg.con_sesion());
        // Sin archivo (tests en memoria) no se sincroniza al iniciar: nunca hay red.
        if auto && self.db.path().is_some() {
            self.lanzar_sync(false);
        }
    }

    /// Exporta `mifutbolmobile_import.json` (Ajustes → Ctrl+W).
    pub fn exportar_mobile_ajustes(&mut self) {
        let usuario = self.usuario_actual.as_ref().map(|u| u.nombre.clone());
        match mobile_service::exportar(self.db.conn(), usuario.as_deref(), &self.paths.export_dir) {
            Ok(ruta) => {
                self.status = format!("Archivo para MiFutbolMobile: {}", ruta.display());
                self.modal = Some(Modal::Info(format!(
                    "Archivo para MiFutbolMobile exportado a:\n{}",
                    ruta.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.0)),
        }
    }

    /// Importa `Importaciones/partidos.json` del móvil (Ajustes → Ctrl+Y).
    pub fn importar_mobile_ajustes(&mut self) {
        let ruta = self.paths.import_dir.join(mobile_service::ARCHIVO_PARTIDOS);
        match mobile_service::importar_partidos(self.db.conn_mut(), &ruta) {
            Ok((importados, total)) => {
                self.inicio_sucio = true;
                self.modal = Some(Modal::Info(format!(
                    "Sincronización completada: {importados} de {total} partidos importados.\n{}",
                    ruta.display()
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error)),
        }
    }

    /// Alterna la importación automática del móvil al iniciar (Ajustes → Ctrl+A).
    pub fn alternar_mobile_auto_ajustes(&mut self) {
        match mobile_service::alternar_auto(&self.paths.import_dir) {
            Ok(true) => {
                self.status = format!(
                    "Importación del móvil al iniciar: ACTIVADA ({} se importará al abrir)",
                    mobile_service::ARCHIVO_PARTIDOS
                );
            }
            Ok(false) => self.status = "Importación del móvil al iniciar: DESACTIVADA".to_string(),
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }
}
