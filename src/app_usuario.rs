//! Cuentas locales y pantalla de bienvenida.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, Form, Modal, Screen};
use crate::domain::models::FormMode;
use crate::domain::usuario::Usuario;
use crate::services::usuario_service;

impl App {
    /// Abre el módulo de usuario.
    pub(crate) fn open_usuarios(&mut self) -> Result<()> {
        self.refresh_usuarios()?;
        if !matches!(self.current_screen(), Screen::UsuarioList) {
            self.push_screen(Screen::UsuarioList);
        }
        Ok(())
    }

    /// Recarga cuentas y sesión.
    pub(crate) fn refresh_usuarios(&mut self) -> Result<()> {
        match usuario_service::listar(self.db.conn()) {
            Ok(usuarios) => {
                self.usuarios = usuarios;
                self.usuarios_table.select(if self.usuarios.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(error) => self.status = format!("No se pudieron cargar las cuentas: {error}"),
        }
        if let Ok(sesion) = usuario_service::sesion(self.db.conn()) {
            self.usuario_actual = sesion.usuario;
            self.bienvenida_vista = sesion.bienvenida_vista;
        }
        Ok(())
    }

    /// Teclas de la pantalla de bienvenida.
    pub(crate) fn handle_bienvenida_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('n') => self.open_usuario_form(FormMode::Create)?,
            KeyCode::Char('l') => self.open_usuario_login("")?,
            KeyCode::Enter | KeyCode::Esc => self.entrar_al_menu()?,
            KeyCode::Char('q') => self.should_quit = true,
            _ => return Ok(false),
        }
        Ok(true)
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
                let total = self.usuarios.len();
                if total > 0 {
                    let indice = (self.usuarios_table.selected().unwrap_or(0) + 1).min(total - 1);
                    self.usuarios_table.select(Some(indice));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let indice = self
                    .usuarios_table
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                self.usuarios_table.select(Some(indice));
            }
            KeyCode::Enter => {
                if let Some(usuario) = self.usuario_seleccionado() {
                    if usuario.con_password {
                        self.open_usuario_login(&usuario.nombre)?;
                    } else {
                        match usuario_service::autenticar(self.db.conn(), &usuario.nombre, None) {
                            Ok(_) => {
                                self.refresh_usuarios()?;
                                self.status = format!("Sesión iniciada como {}", usuario.nombre);
                            }
                            Err(error) => {
                                self.modal = Some(Modal::Error(error.to_string()));
                            }
                        }
                    }
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
                        "Se eliminarán sus datos de sesión. ¿Continuar?",
                        ConfirmAction::DeleteUsuario(id),
                    ));
                }
            }
            KeyCode::Char('c') => {
                if let Err(error) = usuario_service::cerrar_sesion(self.db.conn()) {
                    self.modal = Some(Modal::Error(error.to_string()));
                } else {
                    self.refresh_usuarios()?;
                    self.status = "Sesión cerrada".to_string();
                }
            }
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
                usuario_service::crear(self.db.conn(), &nombre, password).map(|_| ())
            }
            FormMode::Edit => {
                let Some(id) = self.usuario_edit_id else {
                    self.form = Some(form);
                    self.modal = Some(Modal::Error("Cuenta no válida".to_string()));
                    return Ok(());
                };
                usuario_service::cambiar_password(self.db.conn(), id, password)
            }
        };
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_usuarios()?;
                match mode {
                    FormMode::Create => {
                        self.stack = vec![Screen::Bienvenida];
                        self.status = "Cuenta creada. ¡Bienvenido!".to_string();
                    }
                    FormMode::Edit => {
                        self.go_back();
                        self.status = "Contraseña actualizada".to_string();
                    }
                }
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
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
        match usuario_service::autenticar(self.db.conn(), &nombre, Some(&password)) {
            Ok(usuario) => {
                self.form = None;
                self.refresh_usuarios()?;
                self.go_back();
                self.status = format!("Sesión iniciada como {}", usuario.nombre);
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
