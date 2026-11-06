//! Navegación, formularios y acciones del módulo Canchas.
//!
//! Se implementa como `impl App` separado para no inflar `app.rs`.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    App, ConfirmAction, Field, FieldKind, Form, Modal, Screen, SearchState, SearchTarget,
};
use crate::domain::dates;
use crate::domain::models::{
    Cancha, FormMode, ServiciosCancha, Superficie, TechadaEstado, TipoCancha,
};
use crate::domain::validation::{self, DomainError};
use crate::services::cancha_service::{self, ResultadoEliminacion};

impl App {
    /// Teclas del listado de canchas.
    pub(crate) fn handle_canchas_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_cancha_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cancha_selection(-1),
            KeyCode::Enter => self.open_cancha_detail()?,
            KeyCode::Char('n') => self.open_cancha_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_cancha_id() {
                    self.open_cancha_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_cancha(),
            KeyCode::Char('a') => self.toggle_cancha_activa(),
            KeyCode::Char('f') => {
                self.cancha_filter.estado = self.cancha_filter.estado.next();
                self.refresh_canchas()?;
            }
            KeyCode::Char('o') => {
                self.cancha_filter.orden = self.cancha_filter.orden.next();
                self.refresh_canchas()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Canchas,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_canchas()?;
                self.status = "Listado de canchas actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle de cancha.
    pub(crate) fn handle_cancha_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('e') => {
                if let Some(cancha) = &self.cancha_detail {
                    let id = cancha.id;
                    self.open_cancha_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_cancha(),
            KeyCode::Char('a') => self.toggle_cancha_activa(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Recarga el listado.
    pub fn refresh_canchas(&mut self) -> Result<()> {
        let rows = cancha_service::listar(self.db.conn(), &self.cancha_filter)?;
        self.canchas = rows;
        if self.canchas.is_empty() {
            self.canchas_table.select(None);
        } else {
            let index = self
                .canchas_table
                .selected()
                .unwrap_or(0)
                .min(self.canchas.len() - 1);
            self.canchas_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_cancha_id(&self) -> Option<i64> {
        let index = self.canchas_table.selected()?;
        self.canchas.get(index).map(|row| row.id)
    }

    fn move_cancha_selection(&mut self, delta: isize) {
        if self.canchas.is_empty() {
            return;
        }
        let len = self.canchas.len() as isize;
        let current = self
            .canchas_table
            .selected()
            .unwrap_or(0)
            .min(self.canchas.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.canchas_table.select(Some(next));
    }

    fn open_cancha_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_cancha_id() else {
            self.status = "No hay canchas para mostrar".to_string();
            return Ok(());
        };
        match cancha_service::obtener(self.db.conn(), id) {
            Ok(cancha) => {
                self.cancha_detail = Some(cancha);
                self.push_screen(Screen::CanchaDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_cancha_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                self.form_original_cancha = None;
                self.form = Some(cancha_form(None));
                self.push_screen(Screen::CanchaForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_cancha_id() {
                    self.open_cancha_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_cancha_form_for(&mut self, id: i64) -> Result<()> {
        match cancha_service::obtener(self.db.conn(), id) {
            Ok(cancha) => {
                self.form = Some(cancha_form(Some(&cancha)));
                self.form_original_cancha = Some(cancha);
                self.push_screen(Screen::CanchaForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_cancha(&mut self) {
        let Some(id) = self.selected_cancha_id() else {
            self.status = "No hay cancha seleccionada".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar cancha",
            "¿Eliminar la cancha seleccionada? Si tiene partidos se desactivará. (S = sí, N = no)",
            ConfirmAction::DeleteCancha(id),
        ));
    }

    /// Elimina o desactiva una cancha según dependencias.
    pub(crate) fn delete_cancha(&mut self, id: i64) -> Result<()> {
        match cancha_service::eliminar(self.db.conn(), id) {
            Ok(ResultadoEliminacion::Eliminada) => {
                self.status = "Cancha eliminada".to_string();
            }
            Ok(ResultadoEliminacion::Desactivada(dependencias)) => {
                self.status = format!(
                    "Cancha desactivada ({} partidos asociados)",
                    dependencias.partidos
                );
            }
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        }
        if matches!(self.current_screen(), Screen::CanchaDetail(_)) {
            self.stack.pop();
        }
        self.cancha_detail = None;
        self.refresh_canchas()?;
        Ok(())
    }

    fn toggle_cancha_activa(&mut self) {
        let Some(id) = self.selected_cancha_id() else {
            return;
        };
        let actual = self.canchas.get(self.canchas_table.selected().unwrap_or(0));
        let Some(row) = actual else { return };
        let nuevo = !row.activa;
        match cancha_service::set_activa(self.db.conn(), id, nuevo) {
            Ok(()) => {
                self.status = format!("Cancha {}", if nuevo { "activada" } else { "desactivada" });
                if let Err(error) = self.refresh_canchas() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Guarda el formulario de cancha.
    pub(crate) fn save_cancha_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let cancha = match self.cancha_desde_form() {
            Ok(cancha) => cancha,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nueva = cancha;
                cancha_service::crear(self.db.conn_mut(), &mut nueva).map(|_| ())
            }
            FormMode::Edit => cancha_service::actualizar(self.db.conn_mut(), &cancha),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_cancha = None;
                self.stack.pop();
                self.refresh_canchas()?;
                if let Screen::CanchaDetail(id) = self.current_screen()
                    && let Ok(cancha) = cancha_service::obtener(self.db.conn(), id)
                {
                    self.cancha_detail = Some(cancha);
                }
                self.status = "Cancha guardada".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn cancha_desde_form(&self) -> Result<Cancha, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let checked = |key: &str| get(key) == "Sí";

        let mut tipos = Vec::new();
        for (key, tipo) in [
            ("tipo_f5", TipoCancha::F5),
            ("tipo_f7", TipoCancha::F7),
            ("tipo_f8", TipoCancha::F8),
            ("tipo_f11", TipoCancha::F11),
            ("tipo_99", TipoCancha::Otro),
        ] {
            if checked(key) {
                tipos.push(tipo);
            }
        }
        let mut superficies = Vec::new();
        for (key, superficie) in [
            ("sup_natural", Superficie::Natural),
            ("sup_sintetico", Superficie::Sintetico),
            ("sup_cemento", Superficie::Cemento),
            ("sup_otra", Superficie::Otra),
        ] {
            if checked(key) {
                superficies.push(superficie);
            }
        }
        if superficies.is_empty() {
            superficies.push(Superficie::NoSe);
        }

        let techada_code: i64 = get("techada")
            .parse()
            .map_err(|_| DomainError::Message("Techada inválida".to_string()))?;
        let marcador_code: i64 = get("marcador")
            .parse()
            .map_err(|_| DomainError::Message("Marcador inválido".to_string()))?;
        let cantidad: i64 = get("cantidad")
            .parse()
            .map_err(|_| DomainError::Message("Cantidad de canchas inválida".to_string()))?;
        let apertura = parse_minutes(&get("apertura"))?;
        let cierre = parse_minutes(&get("cierre"))?;
        let latitud = optional_f64(&get("latitud"), "latitud")?;
        let longitud = optional_f64(&get("longitud"), "longitud")?;
        let original = self.form_original_cancha.as_ref();

        Ok(Cancha {
            id: original.map_or(0, |cancha| cancha.id),
            nombre: get("nombre"),
            telefono: get("telefono"),
            direccion: get("direccion"),
            localidad: get("localidad"),
            tipos,
            superficies,
            techada: TechadaEstado::try_from(techada_code)?,
            tiene_iluminacion: checked("iluminacion"),
            horario_apertura_min: apertura,
            horario_cierre_min: cierre,
            precio_dia: validation::money(&get("precio_dia"), "precio día")?,
            precio_noche: validation::money(&get("precio_noche"), "precio noche")?,
            servicios: ServiciosCancha {
                vestuarios: checked("vestuarios"),
                duchas: checked("duchas"),
                buffet: checked("buffet"),
                estacionamiento: checked("estacionamiento"),
            },
            cantidad_canchas: cantidad,
            estado: get("estado"),
            descripcion: get("descripcion"),
            contacto_alt: get("contacto_alt"),
            latitud,
            longitud,
            tiene_grabacion: checked("grabacion"),
            tiene_marcador: TechadaEstado::try_from(marcador_code)?,
            imagen_ruta: get("imagen"),
            activa: original.is_none_or(|cancha| cancha.activa),
        })
    }
}

/// Formulario de cancha.
fn cancha_form(cancha: Option<&Cancha>) -> Form {
    let text = |key: &str| {
        cancha.map(|c| match key {
            "nombre" => c.nombre.clone(),
            "telefono" => c.telefono.clone(),
            "direccion" => c.direccion.clone(),
            "localidad" => c.localidad.clone(),
            "estado" => c.estado.clone(),
            "descripcion" => c.descripcion.clone(),
            "contacto_alt" => c.contacto_alt.clone(),
            "imagen" => c.imagen_ruta.clone(),
            "precio_dia" => c.precio_dia.display_ars(),
            "precio_noche" => c.precio_noche.display_ars(),
            "cantidad" => c.cantidad_canchas.to_string(),
            "apertura" => c
                .horario_apertura_min
                .map(format_minutes)
                .unwrap_or_default(),
            "cierre" => c.horario_cierre_min.map(format_minutes).unwrap_or_default(),
            "latitud" => c.latitud.map(|v| v.to_string()).unwrap_or_default(),
            "longitud" => c.longitud.map(|v| v.to_string()).unwrap_or_default(),
            _ => String::new(),
        })
    };

    let tiene_tipo = |tipo: TipoCancha| cancha.is_some_and(|c| c.tipos.contains(&tipo));
    let tiene_sup = |sup: Superficie| cancha.is_some_and(|c| c.superficies.contains(&sup));

    let techada_actual = cancha.map_or(2, |c| c.techada.as_i64()).to_string();
    let marcador_actual = cancha.map_or(0, |c| c.tiene_marcador.as_i64()).to_string();

    let techada_options = vec![
        ("0".to_string(), "Descubierta".to_string()),
        ("1".to_string(), "Techada".to_string()),
        ("2".to_string(), "No se".to_string()),
    ];
    let marcador_options = vec![
        ("0".to_string(), "No especificado".to_string()),
        ("1".to_string(), "Sí".to_string()),
        ("2".to_string(), "No".to_string()),
    ];

    let cantidad = text("cantidad").unwrap_or_else(|| "1".to_string());

    let fields = vec![
        Field::text("nombre", "Nombre", text("nombre").unwrap_or_default(), 100).required(),
        Field::text(
            "telefono",
            "Teléfono",
            text("telefono").unwrap_or_default(),
            40,
        ),
        Field::text(
            "direccion",
            "Dirección",
            text("direccion").unwrap_or_default(),
            200,
        ),
        Field::text(
            "localidad",
            "Localidad",
            text("localidad").unwrap_or_default(),
            100,
        ),
        Field::boolean("tipo_f5", "Tipo F5", tiene_tipo(TipoCancha::F5)),
        Field::boolean("tipo_f7", "Tipo F7", tiene_tipo(TipoCancha::F7)),
        Field::boolean("tipo_f8", "Tipo F8", tiene_tipo(TipoCancha::F8)),
        Field::boolean("tipo_f11", "Tipo F11", tiene_tipo(TipoCancha::F11)),
        Field::boolean("tipo_99", "Tipo otro", tiene_tipo(TipoCancha::Otro)),
        Field::boolean(
            "sup_natural",
            "Superficie natural",
            tiene_sup(Superficie::Natural),
        ),
        Field::boolean(
            "sup_sintetico",
            "Superficie sintética",
            tiene_sup(Superficie::Sintetico),
        ),
        Field::boolean(
            "sup_cemento",
            "Superficie cemento",
            tiene_sup(Superficie::Cemento),
        ),
        Field::boolean("sup_otra", "Superficie otra", tiene_sup(Superficie::Otra)),
        Field::select("techada", "Techada", techada_actual, techada_options),
        Field::boolean(
            "iluminacion",
            "Iluminación",
            cancha.is_some_and(|c| c.tiene_iluminacion),
        ),
        Field {
            key: "apertura",
            label: "Apertura".to_string(),
            value: text("apertura").unwrap_or_default(),
            kind: FieldKind::Time,
            required: false,
            help: Some("HH:MM".to_string()),
        },
        Field {
            key: "cierre",
            label: "Cierre".to_string(),
            value: text("cierre").unwrap_or_default(),
            kind: FieldKind::Time,
            required: false,
            help: Some("HH:MM".to_string()),
        },
        Field {
            key: "precio_dia",
            label: "Precio hora día".to_string(),
            value: text("precio_dia").unwrap_or_else(|| "$0,00".to_string()),
            kind: FieldKind::Money,
            required: false,
            help: None,
        },
        Field {
            key: "precio_noche",
            label: "Precio hora noche".to_string(),
            value: text("precio_noche").unwrap_or_else(|| "$0,00".to_string()),
            kind: FieldKind::Money,
            required: false,
            help: None,
        },
        Field::boolean(
            "vestuarios",
            "Vestuarios",
            cancha.is_some_and(|c| c.servicios.vestuarios),
        ),
        Field::boolean(
            "duchas",
            "Duchas",
            cancha.is_some_and(|c| c.servicios.duchas),
        ),
        Field::boolean(
            "buffet",
            "Buffet",
            cancha.is_some_and(|c| c.servicios.buffet),
        ),
        Field::boolean(
            "estacionamiento",
            "Estacionamiento",
            cancha.is_some_and(|c| c.servicios.estacionamiento),
        ),
        Field {
            key: "cantidad",
            label: "Cantidad de canchas".to_string(),
            value: cantidad,
            kind: FieldKind::Int { min: 1, max: 99 },
            required: true,
            help: None,
        },
        Field::text("estado", "Estado", text("estado").unwrap_or_default(), 50),
        Field::text(
            "descripcion",
            "Descripción",
            text("descripcion").unwrap_or_default(),
            200,
        ),
        Field::text(
            "contacto_alt",
            "Contacto alternativo",
            text("contacto_alt").unwrap_or_default(),
            120,
        ),
        Field {
            key: "latitud",
            label: "Latitud".to_string(),
            value: text("latitud").unwrap_or_default(),
            kind: FieldKind::Decimal {
                min: -90.0,
                max: 90.0,
            },
            required: false,
            help: None,
        },
        Field {
            key: "longitud",
            label: "Longitud".to_string(),
            value: text("longitud").unwrap_or_default(),
            kind: FieldKind::Decimal {
                min: -180.0,
                max: 180.0,
            },
            required: false,
            help: None,
        },
        Field::boolean(
            "grabacion",
            "Grabación",
            cancha.is_some_and(|c| c.tiene_grabacion),
        ),
        Field::select("marcador", "Marcador", marcador_actual, marcador_options),
        Field::text("imagen", "Imagen", text("imagen").unwrap_or_default(), 255),
    ];

    Form::new(
        if cancha.is_some() {
            "Editar cancha"
        } else {
            "Nueva cancha"
        },
        fields,
    )
}

fn parse_minutes(value: &str) -> Result<Option<i64>, DomainError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let time = dates::parse_time(value)
        .ok_or_else(|| DomainError::InvalidDate(format!("hora: {value}")))?;
    use chrono::Timelike;
    Ok(Some(i64::from(time.hour() * 60 + time.minute())))
}

fn format_minutes(minutes: i64) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn optional_f64(value: &str, field: &'static str) -> Result<Option<f64>, DomainError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let parsed: f64 = value
        .parse()
        .map_err(|_| DomainError::Message(format!("{field} inválida")))?;
    Ok(Some(parsed))
}
