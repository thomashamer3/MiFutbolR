//! Bienestar (paridad con MiFutbolC).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use rusqlite::Connection;

use crate::app::{App, ConfirmAction, Eleccion, Field, FieldKind, Form, Modal, Screen};
use crate::db::bienestar_repo;
use crate::domain::bienestar::{Registro, Seccion, TipoDato};
use crate::domain::dates;
use crate::domain::models::FormMode;
use crate::domain::reporte::Reporte;
use crate::services::bienestar_service;

impl App {
    /// Abre Bienestar.
    pub(crate) fn open_bienestar(&mut self) -> Result<()> {
        self.refresh_bienestar()?;
        if !matches!(self.current_screen(), Screen::BienestarList) {
            self.push_screen(Screen::BienestarList);
        }
        Ok(())
    }

    /// Recarga la sección activa.
    pub(crate) fn refresh_bienestar(&mut self) -> Result<()> {
        match bienestar_service::listar(self.db.conn(), self.bienestar_seccion) {
            Ok(registros) => {
                self.bienestar_registros = registros;
                self.bienestar_table
                    .select((!self.bienestar_registros.is_empty()).then_some(0));
            }
            Err(error) => self.status = format!("No se pudo cargar Bienestar: {error}"),
        }
        Ok(())
    }

    fn bienestar_seleccionado(&self) -> Option<Registro> {
        self.bienestar_table
            .selected()
            .and_then(|indice| self.bienestar_registros.get(indice))
            .cloned()
    }

    fn cambiar_seccion_bienestar(&mut self, delta: isize) -> Result<()> {
        let total = Seccion::ALL.len() as isize;
        let actual = Seccion::ALL
            .iter()
            .position(|seccion| *seccion == self.bienestar_seccion)
            .unwrap_or(0) as isize;
        self.bienestar_seccion = Seccion::ALL[(actual + delta).rem_euclid(total) as usize];
        self.refresh_bienestar()
    }

    /// Teclas del listado.
    pub(crate) fn handle_bienestar_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        let seccion = self.bienestar_seccion;
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => Self::mover_tabla_seleccion(
                &mut self.bienestar_table,
                self.bienestar_registros.len(),
                1,
            ),
            KeyCode::Up | KeyCode::Char('k') => Self::mover_tabla_seleccion(
                &mut self.bienestar_table,
                self.bienestar_registros.len(),
                -1,
            ),
            KeyCode::Tab | KeyCode::Right => self.cambiar_seccion_bienestar(1)?,
            KeyCode::BackTab | KeyCode::Left => self.cambiar_seccion_bienestar(-1)?,
            KeyCode::Char('n') if seccion != Seccion::Salud => {
                self.bienestar_edit_id = None;
                self.form = Some(bienestar_form(self.db.conn(), seccion, None));
                self.push_screen(Screen::BienestarForm(FormMode::Create));
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(registro) = self.bienestar_seleccionado() {
                    self.bienestar_edit_id = Some(registro.id);
                    self.form = Some(bienestar_form(self.db.conn(), seccion, Some(&registro)));
                    self.push_screen(Screen::BienestarForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') if seccion != Seccion::Salud => {
                if let Some(registro) = self.bienestar_seleccionado() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar registro",
                        format!("¿Eliminar este registro de {}?", seccion.label()),
                        ConfirmAction::DeleteBienestar(registro.id),
                    ));
                }
            }
            KeyCode::Char('o') if seccion == Seccion::Entrenamientos => {
                if let Some(registro) = self.bienestar_seleccionado() {
                    match bienestar_service::alternar_omitido(self.db.conn(), registro.id) {
                        Ok(()) => {
                            let indice = self.bienestar_table.selected();
                            self.refresh_bienestar()?;
                            self.bienestar_table.select(indice);
                        }
                        Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                    }
                }
            }
            KeyCode::Char('v') => {
                if let (Some(indice), Some(registro)) =
                    (seccion.columna_archivo(), self.bienestar_seleccionado())
                {
                    let ruta = registro.valores[indice].clone();
                    if bienestar_service::es_imagen(&ruta) {
                        self.abrir_imagen(&ruta);
                    } else {
                        self.status =
                            match bienestar_service::comando_abrir(&self.paths.data_dir, &ruta) {
                                Ok(_) if !self.abrir_con_sistema => format!(
                                    "Archivo: {}",
                                    self.paths.data_dir.join(&ruta).display()
                                ),
                                Ok(mut comando) => match comando.spawn() {
                                    Ok(_) => format!("Abriendo {ruta} con la app del sistema"),
                                    Err(error) => format!("No se pudo abrir {ruta}: {error}"),
                                },
                                Err(error) => error.to_string(),
                            };
                    }
                }
            }
            KeyCode::Char('a') => self.open_bienestar_asistente(),
            KeyCode::Char('A') => match bienestar_service::analisis(self.db.conn()) {
                Ok(reporte) => self.abrir_reporte(reporte),
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            },
            KeyCode::Char('p') => {
                let mes = Field::text(
                    "mes",
                    "Mes (MM/AAAA)",
                    dates::today().strftime("%m/%Y").to_string(),
                    7,
                )
                .required();
                self.abrir_eleccion(Eleccion::InformeBienestar, vec![mes]);
            }
            KeyCode::Char('r') => {
                self.refresh_bienestar()?;
                self.status = "Bienestar actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Elimina el registro confirmado de la sección activa.
    pub(crate) fn eliminar_bienestar(&mut self, id: i64) {
        let resultado = bienestar_service::eliminar(self.db.conn(), self.bienestar_seccion, id);
        self.confirmar_eliminacion(resultado, App::refresh_bienestar);
    }

    fn open_bienestar_asistente(&mut self) {
        match bienestar_service::asistente(self.db.conn(), dates::today()) {
            Ok(asistente) => {
                self.bienestar_asistente = Some(asistente);
                self.bienestar_recomendaciones =
                    bienestar_service::recomendaciones(self.db.conn()).unwrap_or_default();
                if !matches!(self.current_screen(), Screen::BienestarAsistente) {
                    self.push_screen(Screen::BienestarAsistente);
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Teclas del asistente.
    pub(crate) fn handle_bienestar_asistente_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('s') => {
                if let Some(asistente) = self.bienestar_asistente.clone() {
                    match bienestar_service::guardar_recomendacion(
                        self.db.conn(),
                        dates::today(),
                        &asistente,
                    ) {
                        Ok(()) => {
                            self.bienestar_recomendaciones =
                                bienestar_service::recomendaciones(self.db.conn())
                                    .unwrap_or_default();
                            self.status = "Recomendación guardada".to_string();
                        }
                        Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                    }
                }
            }
            KeyCode::Char('r') => self.open_bienestar_asistente(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Genera el informe personal mensual en PDF (Exportaciones/).
    pub(crate) fn informe_bienestar(&mut self, form: &Form) -> Result<Option<Reporte>, String> {
        let usuario = self.usuario_actual.as_ref().map(|u| u.nombre.clone());
        let ruta = crate::services::informe_service::exportar_bienestar_mes(
            self.db.conn(),
            form.value("mes").unwrap_or(""),
            usuario.as_deref(),
            &self.paths.export_dir,
        )
        .map_err(|error| error.0)?;
        self.status = format!("Informe mensual: {}", ruta.display());
        Ok(None)
    }

    /// Guarda el formulario.
    pub(crate) fn save_bienestar_form(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let seccion = self.bienestar_seccion;
        let mut textos = seccion
            .columnas()
            .iter()
            .map(|columna| form.value(columna.clave).unwrap_or("").to_string())
            .collect::<Vec<_>>();
        if let Err(error) =
            bienestar_service::adjuntar(seccion, &mut textos, &self.paths.images_dir)
        {
            self.form = Some(form);
            self.modal = Some(Modal::Error(error.to_string()));
            return Ok(());
        }
        match bienestar_service::guardar(
            self.db.conn(),
            seccion,
            self.bienestar_edit_id,
            &textos,
            dates::today(),
        ) {
            Ok(_) => {
                self.form = None;
                self.refresh_bienestar()?;
                self.go_back();
                self.status = format!("{} guardado", seccion.label());
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }
}

fn bienestar_form(conn: &Connection, seccion: Seccion, registro: Option<&Registro>) -> Form {
    let campos = seccion
        .columnas()
        .iter()
        .enumerate()
        .map(|(indice, columna)| {
            let actual = registro.and_then(|r| r.valores.get(indice)).cloned();
            let campo = |kind, value: String| Field {
                key: columna.clave,
                label: columna.etiqueta.to_string(),
                value,
                kind,
                required: false,
                help: None,
            };
            match columna.tipo {
                TipoDato::Fecha => {
                    let mut field = campo(
                        FieldKind::Date,
                        actual.unwrap_or_else(|| {
                            if columna.clave == "fecha" {
                                dates::display_date(dates::today())
                            } else {
                                String::new()
                            }
                        }),
                    );
                    field.help = Some("DD/MM/AAAA (vacío = hoy)".to_string());
                    field
                }
                TipoDato::Entero(min, max) => campo(
                    FieldKind::Int { min, max },
                    actual.unwrap_or_else(|| min.to_string()),
                ),
                TipoDato::Decimal(min, max) => {
                    campo(FieldKind::Decimal { min, max }, actual.unwrap_or_default())
                }
                TipoDato::Booleano => Field::boolean(
                    columna.clave,
                    columna.etiqueta,
                    actual.as_deref() == Some("Sí"),
                ),
                TipoDato::Opcion(opciones) => Field::select(
                    columna.clave,
                    columna.etiqueta,
                    actual.unwrap_or_else(|| opciones[0].to_string()),
                    opciones
                        .iter()
                        .map(|opcion| ((*opcion).to_string(), (*opcion).to_string()))
                        .collect(),
                ),
                TipoDato::Texto(max) => Field::text(
                    columna.clave,
                    columna.etiqueta,
                    actual.unwrap_or_default(),
                    max,
                ),
                TipoDato::Referencia(referencia) => {
                    let opciones = bienestar_repo::opciones(conn, referencia).unwrap_or_default();
                    let valor = actual
                        .or_else(|| opciones.first().map(|(id, _)| id.clone()))
                        .unwrap_or_default();
                    Field::select(columna.clave, columna.etiqueta, valor, opciones)
                }
                TipoDato::Archivo => {
                    let mut field = Field::text(
                        columna.clave,
                        columna.etiqueta,
                        actual.unwrap_or_default(),
                        400,
                    );
                    field.help = Some("Ruta del archivo; se copia a Imagenes/".to_string());
                    field
                }
            }
        })
        .collect();
    Form::new(seccion.label(), campos)
}
