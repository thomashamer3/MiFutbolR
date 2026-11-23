//! Cuentas locales y pantalla de bienvenida.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, Form, Modal, Screen};
use crate::db::Database;
use crate::domain::models::FormMode;
use crate::domain::usuario::Usuario;
use crate::services::{perfiles_import_service, usuario_service};

impl App {
    /// Abre el módulo de usuario.
    pub(crate) fn open_usuarios(&mut self) -> Result<()> {
        self.refresh_usuarios()?;
        if !matches!(self.current_screen(), Screen::UsuarioList) {
            self.push_screen(Screen::UsuarioList);
        }
        Ok(())
    }

    /// Recarga las cuentas del registro y preselecciona la última usada.
    pub(crate) fn refresh_usuarios(&mut self) -> Result<()> {
        match usuario_service::listar(self.cuentas.conn()) {
            Ok(usuarios) => self.usuarios = usuarios,
            Err(error) => self.status = format!("No se pudieron cargar las cuentas: {error}"),
        }
        let ultima = usuario_service::sesion(self.cuentas.conn())
            .ok()
            .and_then(|sesion| sesion.usuario)
            .and_then(|ultima| self.usuarios.iter().position(|u| u.id == ultima.id));
        self.usuarios_table.select(if self.usuarios.is_empty() {
            None
        } else {
            Some(ultima.unwrap_or(0))
        });
        Ok(())
    }

    /// Muestra la portada de inicio de sesión como única pantalla.
    pub fn mostrar_inicio_sesion(&mut self) {
        self.stack = vec![Screen::Bienvenida];
        self.active_module = None;
        let _ = self.refresh_usuarios();
    }

    /// Teclas de la portada de inicio de sesión.
    pub(crate) fn handle_bienvenida_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.usuarios_table, self.usuarios.len(), 1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.usuarios_table, self.usuarios.len(), -1)
            }
            KeyCode::Enter => match self.usuario_seleccionado() {
                Some(usuario) => self.entrar_seleccionado(&usuario)?,
                None => self.entrar_al_menu()?,
            },
            KeyCode::Char('n') => self.open_usuario_form(FormMode::Create)?,
            KeyCode::Char('l') => {
                let nombre = self
                    .usuario_seleccionado()
                    .map(|usuario| usuario.nombre)
                    .unwrap_or_default();
                self.open_usuario_login(&nombre)?;
            }
            KeyCode::Char('i') => self.entrar_al_menu()?,
            KeyCode::Char('m') => self.importar_perfiles_c(),
            KeyCode::Char('q') | KeyCode::Esc => self.request_quit(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Importa los perfiles de MiFutbolC desde su directorio de datos por defecto.
    fn importar_perfiles_c(&mut self) {
        let Some(dir) = perfiles_import_service::directorio_c_por_defecto() else {
            self.modal = Some(Modal::Error(
                "No se encontró el directorio de MiFutbolC (LOCALAPPDATA).".to_string(),
            ));
            return;
        };
        match perfiles_import_service::importar_perfiles(self.cuentas.conn(), &self.paths, &dir) {
            Ok(resumen) => {
                let _ = self.refresh_usuarios();
                self.status = format!(
                    "{} perfiles importados de MiFutbolC",
                    resumen.perfiles.len()
                );
                if !resumen.advertencias.is_empty() {
                    self.modal = Some(Modal::Error(resumen.advertencias.join("\n")));
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Entra con la cuenta elegida: directo si no tiene contraseña.
    fn entrar_seleccionado(&mut self, usuario: &Usuario) -> Result<()> {
        if usuario.con_password {
            return self.open_usuario_login(&usuario.nombre);
        }
        match usuario_service::autenticar(self.cuentas.conn(), &usuario.nombre, None) {
            Ok(usuario) => self.entrar_como(usuario),
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Reconstruye la app sobre la base de datos propia de la cuenta.
    pub(crate) fn entrar_como(&mut self, usuario: Usuario) {
        let ruta = self.paths.db_usuario(usuario.id);
        let nueva = Database::open(&ruta)
            .map_err(anyhow::Error::from)
            .and_then(|db| App::new(db, self.paths.clone()));
        match nueva {
            Ok(app) => {
                *self = app;
                self.bienvenida_vista = true;
                self.status = format!(
                    "Sesión iniciada como {} · BD: {}",
                    usuario.nombre,
                    ruta.display()
                );
                if let Err(error) = usuario_service::marcar_bienvenida(self.db.conn()) {
                    self.status = format!("No se pudo guardar la bienvenida: {error}");
                }
                self.usuario_actual = Some(usuario);
                // App::new ya calculó el resumen de Inicio de esta base.
                self.stack = vec![Screen::MainMenu];
                self.ofrecer_restaurar_backup_inicio();
                self.sincronizar_al_iniciar();
            }
            Err(error) => {
                self.modal = Some(Modal::Error(format!(
                    "No se pudo abrir la base de {}: {error:#}",
                    usuario.nombre
                )));
            }
        }
    }

    /// Cierra la sesión y vuelve a la portada sobre la base de invitado.
    pub(crate) fn salir_a_invitado(&mut self) {
        if let Err(error) = usuario_service::cerrar_sesion(self.cuentas.conn()) {
            self.modal = Some(Modal::Error(error.to_string()));
            return;
        }
        let nueva = Database::open(&self.paths.db_file)
            .map_err(anyhow::Error::from)
            .and_then(|db| App::new(db, self.paths.clone()));
        match nueva {
            Ok(app) => {
                *self = app;
                self.mostrar_inicio_sesion();
                self.status = "Sesión cerrada".to_string();
            }
            Err(error) => {
                self.modal = Some(Modal::Error(format!(
                    "No se pudo abrir la base de invitado: {error:#}"
                )));
            }
        }
    }

    /// Marca la bienvenida como vista y abre el menú.
    pub(crate) fn entrar_al_menu(&mut self) -> Result<()> {
        if let Err(error) = usuario_service::marcar_bienvenida(self.db.conn()) {
            self.modal = Some(Modal::Error(error.to_string()));
            return Ok(());
        }
        self.bienvenida_vista = true;
        self.stack = vec![Screen::MainMenu];
        self.active_module = None;
        let _ = self.refrescar_inicio();
        self.ofrecer_restaurar_backup_inicio();
        self.sincronizar_al_iniciar();
        Ok(())
    }

    fn open_usuario_form(&mut self, mode: FormMode) -> Result<()> {
        self.usuario_edit_id = None;
        self.form = Some(usuario_form(mode, None));
        self.push_screen(Screen::UsuarioForm(mode));
        Ok(())
    }

    fn open_usuario_edit(&mut self, usuario: &Usuario) -> Result<()> {
        self.usuario_edit_id = Some(usuario.id);
        self.form = Some(usuario_form(FormMode::Edit, Some(usuario)));
        self.push_screen(Screen::UsuarioForm(FormMode::Edit));
        Ok(())
    }

    fn open_usuario_login(&mut self, nombre: &str) -> Result<()> {
        self.usuario_login_nombre = nombre.to_string();
        self.form = Some(login_form(nombre));
        self.push_screen(Screen::UsuarioLogin);
        Ok(())
    }

    /// Teclas del listado de cuentas.
    pub(crate) fn handle_usuario_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.usuarios_table, self.usuarios.len(), 1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.usuarios_table, self.usuarios.len(), -1)
            }
            KeyCode::Enter => {
                if let Some(usuario) = self.usuario_seleccionado() {
                    self.entrar_seleccionado(&usuario)?;
                }
            }
            KeyCode::Char('n') => self.open_usuario_form(FormMode::Create)?,
            KeyCode::Char('l') => {
                if let Some(usuario) = self.usuario_seleccionado() {
                    self.open_usuario_login(&usuario.nombre)?;
                }
            }
            KeyCode::Char('p') => {
                if let Some(usuario) = self.usuario_seleccionado() {
                    self.open_usuario_edit(&usuario)?;
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self
                    .usuarios_table
                    .selected()
                    .and_then(|indice| self.usuarios.get(indice))
                    .map(|usuario| usuario.id)
                {
                    self.modal = Some(Modal::confirm(
                        "Eliminar cuenta",
                        "La cuenta se quitará del inicio de sesión; su base de datos queda en disco. ¿Continuar?",
                        ConfirmAction::DeleteUsuario(id),
                    ));
                }
            }
            KeyCode::Char('c') => self.salir_a_invitado(),
            KeyCode::Char('r') => {
                self.refresh_usuarios()?;
                self.status = "Cuentas actualizadas".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn usuario_seleccionado(&self) -> Option<Usuario> {
        self.usuarios_table
            .selected()
            .and_then(|indice| self.usuarios.get(indice))
            .cloned()
    }

    /// Teclas del login (Enter envía).
    pub(crate) fn handle_usuario_login_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Enter => self.save_usuario_login()?,
            KeyCode::Esc => {
                self.form = None;
                self.go_back();
            }
            _ => {
                self.handle_form_key(key)?;
            }
        }
        Ok(true)
    }

    /// Guarda una cuenta o cambio de contraseña.
    pub(crate) fn save_usuario_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let password = form.value("password").unwrap_or("").to_string();
        let repetir = form.value("repetir").unwrap_or("").to_string();
        if password != repetir {
            self.form = Some(form);
            self.modal = Some(Modal::Error("Las contraseñas no coinciden".to_string()));
            return Ok(());
        }
        let password = if password.is_empty() {
            None
        } else {
            Some(password.as_str())
        };
        let resultado = match mode {
            FormMode::Create => {
                let nombre = form.value("nombre").unwrap_or("").to_string();
                usuario_service::crear(self.cuentas.conn(), &nombre, password).map(Some)
            }
            FormMode::Edit => {
                let Some(id) = self.usuario_edit_id else {
                    self.form = Some(form);
                    self.modal = Some(Modal::Error("Cuenta no válida".to_string()));
                    return Ok(());
                };
                let actual = form.value("actual");
                usuario_service::cambiar_password(self.cuentas.conn(), id, actual, password)
                    .map(|()| None)
            }
        };
        match resultado {
            Ok(Some(usuario)) => {
                self.form = None;
                self.entrar_como(usuario);
            }
            Ok(None) => {
                self.form = None;
                self.refresh_usuarios()?;
                self.go_back();
                self.status = "Contraseña actualizada".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }

    /// Tras confirmar: elimina directo o pide la contraseña actual si la cuenta tiene.
    pub(crate) fn confirmar_eliminar_usuario(&mut self, id: i64) {
        if self.usuarios.iter().any(|u| u.id == id && u.con_password) {
            self.usuario_edit_id = Some(id);
            self.form = Some(Form::new(
                "Eliminar cuenta",
                vec![Field::password("actual", "Contraseña actual", "", 100).required()],
            ));
            self.push_screen(Screen::UsuarioEliminar);
            return;
        }
        self.eliminar_usuario(id, None);
    }

    /// Elimina la cuenta validando la contraseña ingresada.
    pub(crate) fn save_usuario_eliminar(&mut self) -> Result<()> {
        let (Some(form), Some(id)) = (self.form.as_ref(), self.usuario_edit_id) else {
            return Ok(());
        };
        let actual = form.value("actual").unwrap_or("").to_string();
        if self.eliminar_usuario(id, Some(&actual)) {
            // Se eliminó otra cuenta: se cierra el formulario y se vuelve al listado.
            self.form = None;
            self.go_back();
        }
        Ok(())
    }

    /// Devuelve `true` si se eliminó y la app sigue en la misma base.
    fn eliminar_usuario(&mut self, id: i64, actual: Option<&str>) -> bool {
        let era_actual = self.usuario_actual.as_ref().is_some_and(|u| u.id == id);
        match usuario_service::eliminar(self.cuentas.conn(), id, actual) {
            Ok(()) if era_actual => {
                self.salir_a_invitado();
                false
            }
            Ok(()) => {
                let _ = self.refresh_usuarios();
                self.status = "Cuenta eliminada".to_string();
                true
            }
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                false
            }
        }
    }

    /// Autentica con el formulario de login.
    pub(crate) fn save_usuario_login(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let nombre = form.value("nombre").unwrap_or("").to_string();
        let password = form.value("password").unwrap_or("").to_string();
        match usuario_service::autenticar(self.cuentas.conn(), &nombre, Some(&password)) {
            Ok(usuario) => {
                self.form = None;
                self.entrar_como(usuario);
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn usuario_form(mode: FormMode, usuario: Option<&Usuario>) -> Form {
    let nombre = usuario.map_or(String::new(), |usuario| usuario.nombre.clone());
    let mut campos = Vec::new();
    if mode == FormMode::Create {
        campos.push(Field::text("nombre", "Nombre", nombre, 40).required());
    }
    if usuario.is_some_and(|usuario| usuario.con_password) {
        campos.push(Field::password("actual", "Contraseña actual", "", 100).required());
    }
    let etiqueta = if mode == FormMode::Create {
        "Contraseña (opcional)"
    } else {
        "Nueva contraseña (vacío = sin contraseña)"
    };
    campos.push(Field::password("password", etiqueta, "", 100));
    campos.push(Field::password("repetir", "Repetir contraseña", "", 100));
    let titulo = if mode == FormMode::Create {
        "Crear cuenta"
    } else {
        "Cambiar contraseña"
    };
    Form::new(titulo, campos)
}

fn login_form(nombre: &str) -> Form {
    Form::new(
        "Iniciar sesión",
        vec![
            Field::text("nombre", "Nombre", nombre, 40).required(),
            Field::password("password", "Contraseña (si tiene)", "", 100),
        ],
    )
}
