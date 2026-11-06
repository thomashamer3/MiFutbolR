//! Navegación, filtros y formularios del módulo Partidos.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{
    App, ConfirmAction, Field, FieldKind, Form, Modal, Screen, SearchState, SearchTarget,
};
use crate::db::helpers::Page;
use crate::domain::dates;
use crate::domain::models::{
    Atajaste, Clima, DolorFisico, EstadoCancha, FormMode, FormatoPartido, Partido, Resultado,
    Tarjeta, TipoPartido,
};
use crate::domain::validation::{self, DomainError};
use crate::services::botin_service;
use crate::services::camiseta_service;
use crate::services::cancha_service;
use crate::services::partido_service;

impl App {
    /// Teclas del listado de partidos.
    pub(crate) fn handle_partidos_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_partido_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_partido_selection(-1),
            KeyCode::PageDown => self.cambiar_pagina(1)?,
            KeyCode::PageUp => self.cambiar_pagina(-1)?,
            KeyCode::Enter => self.open_partido_detail()?,
            KeyCode::Char('n') => self.open_partido_wizard()?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_partido_id() {
                    self.open_partido_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_partido(),
            KeyCode::Char('F') => self.alternar_favorito(),
            KeyCode::Char('G') => {
                self.partido_filter.favorito = !self.partido_filter.favorito;
                self.partido_page = 1;
                self.refresh_partidos()?;
            }
            KeyCode::Char('f') => {
                self.partido_filter.tipo_partido = match self.partido_filter.tipo_partido {
                    None => Some(TipoPartido::Amistoso),
                    Some(TipoPartido::Amistoso) => Some(TipoPartido::Torneo),
                    Some(TipoPartido::Torneo) => Some(TipoPartido::Entrenamiento),
                    Some(TipoPartido::Entrenamiento) => None,
                };
                self.partido_page = 1;
                self.refresh_partidos()?;
            }
            KeyCode::Char('v') => {
                self.partido_filter.resultado = match self.partido_filter.resultado {
                    None => Some(Resultado::Victoria),
                    Some(Resultado::Victoria) => Some(Resultado::Empate),
                    Some(Resultado::Empate) => Some(Resultado::Derrota),
                    _ => None,
                };
                self.partido_page = 1;
                self.refresh_partidos()?;
            }
            KeyCode::Char('o') => {
                self.partido_filter.orden = self.partido_filter.orden.next();
                self.refresh_partidos()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Partidos,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_partidos()?;
                self.status = "Listado de partidos actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Teclas del detalle.
    pub(crate) fn handle_partido_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('e') => {
                if let Some(partido) = &self.partido_detail {
                    let id = partido.id;
                    self.open_partido_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_partido(),
            KeyCode::Char('F') => self.alternar_favorito(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Recarga el listado aplicando paginación y filtros.
    pub fn refresh_partidos(&mut self) -> Result<()> {
        let total = partido_service::contar(self.db.conn(), &self.partido_filter)?;
        self.partido_total = total;
        let page_size = self.partido_page_size.max(1);
        let max_page = ((total + page_size - 1) / page_size).max(1);
        if self.partido_page > max_page {
            self.partido_page = max_page;
        }
        if self.partido_page < 1 {
            self.partido_page = 1;
        }
        let page = Page::numbered(self.partido_page, page_size);
        self.partidos = partido_service::listar(self.db.conn(), &self.partido_filter, page)?;
        self.partido_resumen = partido_service::resumen(self.db.conn(), &self.partido_filter)?;
        if self.partidos.is_empty() {
            self.partidos_table.select(None);
        } else {
            let index = self
                .partidos_table
                .selected()
                .unwrap_or(0)
                .min(self.partidos.len() - 1);
            self.partidos_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_partido_id(&self) -> Option<i64> {
        let index = self.partidos_table.selected()?;
        self.partidos.get(index).map(|row| row.id)
    }

    fn move_partido_selection(&mut self, delta: isize) {
        if self.partidos.is_empty() {
            return;
        }
        let len = self.partidos.len() as isize;
        let current = self
            .partidos_table
            .selected()
            .unwrap_or(0)
            .min(self.partidos.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.partidos_table.select(Some(next));
    }

    fn cambiar_pagina(&mut self, delta: i64) -> Result<()> {
        let page_size = self.partido_page_size.max(1);
        let max_page = ((self.partido_total + page_size - 1) / page_size).max(1);
        let destino = (self.partido_page + delta).clamp(1, max_page);
        if destino != self.partido_page {
            self.partido_page = destino;
            self.partidos_table.select(Some(0));
            self.refresh_partidos()?;
        }
        Ok(())
    }

    fn open_partido_detail(&mut self) -> Result<()> {
        let Some(id) = self.selected_partido_id() else {
            self.status = "No hay partidos para mostrar".to_string();
            return Ok(());
        };
        match partido_service::obtener(self.db.conn(), id) {
            Ok(partido) => {
                self.partido_detail = Some(partido);
                self.push_screen(Screen::PartidoDetail(id));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn open_partido_form_for(&mut self, id: i64) -> Result<()> {
        match partido_service::obtener(self.db.conn(), id) {
            Ok(partido) => {
                self.form = Some(self.partido_form(Some(&partido)));
                self.form_original_partido = Some(partido);
                self.push_screen(Screen::PartidoForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn prerrequisitos_partido(&self) -> Result<(), String> {
        let canchas = cancha_service::listar(self.db.conn(), &Default::default())
            .map_err(|error| error.to_string())?;
        if canchas.is_empty() {
            return Err("Antes de registrar un partido cree al menos una cancha.".to_string());
        }
        let camisetas = camiseta_service::listar(self.db.conn(), &Default::default())
            .map_err(|error| error.to_string())?;
        if camisetas.is_empty() {
            return Err("Antes de registrar un partido cree al menos una camiseta.".to_string());
        }
        Ok(())
    }

    fn confirm_delete_partido(&mut self) {
        let Some(id) = self.selected_partido_id() else {
            self.status = "No hay partido seleccionado".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar partido",
            "Se eliminará el partido y su transacción financiera automática. (S = sí, N = no)",
            ConfirmAction::DeletePartido(id),
        ));
    }

    /// Elimina un partido.
    pub(crate) fn delete_partido(&mut self, id: i64) -> Result<()> {
        match partido_service::eliminar(self.db.conn_mut(), id) {
            Ok(()) => {
                self.status = "Partido eliminado".to_string();
                if matches!(self.current_screen(), Screen::PartidoDetail(_)) {
                    self.stack.pop();
                }
                self.partido_detail = None;
                self.refresh_partidos()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn alternar_favorito(&mut self) {
        let Some(id) = self.selected_partido_id() else {
            return;
        };
        match partido_service::alternar_favorito(self.db.conn(), id) {
            Ok(favorito) => {
                self.status = if favorito {
                    "Marcado como favorito".to_string()
                } else {
                    "Quitado de favoritos".to_string()
                };
                if let Err(error) = self.refresh_partidos() {
                    self.modal = Some(Modal::Error(error.to_string()));
                }
                if let Screen::PartidoDetail(id) = self.current_screen()
                    && let Ok(partido) = partido_service::obtener(self.db.conn(), id)
                {
                    self.partido_detail = Some(partido);
                }
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
    }

    /// Guarda el formulario de partido.
    pub(crate) fn save_partido_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let mut partido = match self.partido_desde_form() {
            Ok(partido) => partido,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                partido_service::crear(self.db.conn_mut(), &mut partido).map(|_| ())
            }
            FormMode::Edit => partido_service::actualizar(self.db.conn_mut(), &mut partido),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_partido = None;
                self.stack.pop();
                self.refresh_partidos()?;
                if let Screen::PartidoDetail(id) = self.current_screen()
                    && let Ok(partido) = partido_service::obtener(self.db.conn(), id)
                {
                    self.partido_detail = Some(partido);
                }
                self.status = "Partido guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn partido_form(&self, partido: Option<&Partido>) -> Form {
        let canchas = opciones_ids(
            cancha_service::listar(self.db.conn(), &Default::default())
                .unwrap_or_default()
                .into_iter()
                .map(|row| (row.id, row.nombre)),
        );
        let camisetas = opciones_ids(
            camiseta_service::listar(self.db.conn(), &Default::default())
                .unwrap_or_default()
                .into_iter()
                .map(|row| (row.id, row.nombre)),
        );
        let mut botines = vec![("0".to_string(), "Ninguno".to_string())];
        botines.extend(opciones_ids(
            botin_service::listar(self.db.conn(), &Default::default())
                .unwrap_or_default()
                .into_iter()
                .map(|row| (row.id, row.nombre)),
        ));

        let text = |key: &str| {
            partido.map(|p| match key {
                "rival_nombre" => p.rival_nombre.clone(),
                "tipo_rival" => p.tipo_rival.clone(),
                "posicion_jugada" => p.posicion_jugada.clone(),
                "comentario" => p.comentario_personal.clone(),
                "eventos_clave" => p.eventos_clave.clone(),
                "lo_mejor" => p.lo_mejor.clone(),
                "que_mejorar" => p.que_mejorar.clone(),
                "goles_detalle" => p.goles_detalle.clone(),
                "asistencias_detalle" => p.asistencias_detalle.clone(),
                "tags" => p.tags.join(", "),
                "temperatura" => p.temperatura_c.map(|v| v.to_string()).unwrap_or_default(),
                "precio" => p.precio.display_ars(),
                "goles_equipo" => p.goles_equipo.to_string(),
                "goles_rival" => p.goles_rival.to_string(),
                "minutos" => p.minutos_jugados.to_string(),
                "intensidad" => p.intensidad.to_string(),
                "rendimiento" => p.rendimiento_general.to_string(),
                "cansancio" => p.cansancio.to_string(),
                "estado_animo" => p.estado_animo.to_string(),
                "rating_tecnico" => p.rating_tecnico.to_string(),
                "rating_fisico" => p.rating_fisico.to_string(),
                "rating_mental" => p.rating_mental.to_string(),
                "arbitraje_score" => p.arbitraje_score.to_string(),
                "goles" => p.goles.to_string(),
                "asistencias" => p.asistencias.to_string(),
                "goles_en_contra" => p.goles_en_contra.to_string(),
                _ => String::new(),
            })
        };

        let id_value = |opciones: &[(String, String)], actual: i64| {
            if opciones
                .iter()
                .any(|(value, _)| value == &actual.to_string())
            {
                actual.to_string()
            } else {
                opciones
                    .first()
                    .map(|(value, _)| value.clone())
                    .unwrap_or_default()
            }
        };

        let cancha_actual = partido.map_or(0, |p| p.cancha_id);
        let camiseta_actual = partido.map_or(0, |p| p.camiseta_id);
        let botin_actual = partido.map_or(0, |p| p.botin_id.unwrap_or(0));
        let tipo_actual = partido.map_or(1, |p| p.tipo_partido.as_i64());
        let fecha = partido.map_or_else(
            || dates::display_datetime(dates::now_local()),
            |p| dates::display_datetime(p.fecha_hora),
        );
        let resultado_actual = partido.map_or(1, |p| p.resultado.as_i64());

        Form::new(
            if partido.is_some() {
                "Editar partido"
            } else {
                "Nuevo partido"
            },
            vec![
                Field::select(
                    "tipo_partido",
                    "Tipo de partido",
                    tipo_actual.to_string(),
                    opciones_num(TipoPartido::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field {
                    key: "fecha_hora",
                    label: "Fecha y hora".to_string(),
                    value: fecha,
                    kind: FieldKind::DateTime,
                    required: true,
                    help: Some("DD/MM/AAAA HH:MM".to_string()),
                },
                Field::select(
                    "cancha_id",
                    "Cancha",
                    id_value(&canchas, cancha_actual),
                    canchas,
                ),
                Field::select(
                    "camiseta_id",
                    "Camiseta",
                    id_value(&camisetas, camiseta_actual),
                    camisetas,
                ),
                Field::select("botin_id", "Botín", botin_actual.to_string(), botines),
                Field::select(
                    "formato_partido",
                    "Formato",
                    partido
                        .map_or("F5", |p| p.formato_partido.as_str())
                        .to_string(),
                    opciones_text(FormatoPartido::ALL.iter().map(|v| (v.as_str(), v.label()))),
                ),
                Field::select(
                    "resultado",
                    "Resultado",
                    resultado_actual.to_string(),
                    opciones_num(Resultado::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field {
                    key: "goles",
                    label: "Goles".to_string(),
                    value: text("goles").unwrap_or_else(|| "0".to_string()),
                    kind: FieldKind::Int { min: 0, max: 99 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "asistencias",
                    label: "Asistencias".to_string(),
                    value: text("asistencias").unwrap_or_else(|| "0".to_string()),
                    kind: FieldKind::Int { min: 0, max: 99 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "goles_en_contra",
                    label: "Goles en contra".to_string(),
                    value: text("goles_en_contra").unwrap_or_else(|| "0".to_string()),
                    kind: FieldKind::Int { min: 0, max: 99 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "goles_equipo",
                    label: "Goles del equipo".to_string(),
                    value: text("goles_equipo").unwrap_or_else(|| "-1".to_string()),
                    kind: FieldKind::Int { min: -1, max: 99 },
                    required: false,
                    help: Some("-1 = no cargado".to_string()),
                },
                Field {
                    key: "goles_rival",
                    label: "Goles del rival".to_string(),
                    value: text("goles_rival").unwrap_or_else(|| "-1".to_string()),
                    kind: FieldKind::Int { min: -1, max: 99 },
                    required: false,
                    help: Some("-1 = no cargado".to_string()),
                },
                Field::text(
                    "rival_nombre",
                    "Rival",
                    text("rival_nombre").unwrap_or_default(),
                    100,
                ),
                Field::text(
                    "tipo_rival",
                    "Tipo de rival",
                    text("tipo_rival").unwrap_or_default(),
                    40,
                ),
                Field::text(
                    "posicion_jugada",
                    "Posición jugada",
                    text("posicion_jugada").unwrap_or_default(),
                    40,
                ),
                Field {
                    key: "minutos",
                    label: "Minutos jugados".to_string(),
                    value: text("minutos").unwrap_or_else(|| "0".to_string()),
                    kind: FieldKind::Int { min: 0, max: 180 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "intensidad",
                    label: "Intensidad".to_string(),
                    value: text("intensidad").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "rendimiento",
                    label: "Rendimiento".to_string(),
                    value: text("rendimiento").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "cansancio",
                    label: "Cansancio".to_string(),
                    value: text("cansancio").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "estado_animo",
                    label: "Estado de ánimo".to_string(),
                    value: text("estado_animo").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "rating_tecnico",
                    label: "Rating técnico".to_string(),
                    value: text("rating_tecnico").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "rating_fisico",
                    label: "Rating físico".to_string(),
                    value: text("rating_fisico").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field {
                    key: "rating_mental",
                    label: "Rating mental".to_string(),
                    value: text("rating_mental").unwrap_or_else(|| "5".to_string()),
                    kind: FieldKind::Int { min: 1, max: 10 },
                    required: true,
                    help: None,
                },
                Field::select(
                    "estado_cancha",
                    "Estado de cancha",
                    partido.map_or(2, |p| p.estado_cancha.as_i64()).to_string(),
                    opciones_num(EstadoCancha::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field::select(
                    "clima",
                    "Clima",
                    partido.map_or(1, |p| p.clima.as_i64()).to_string(),
                    opciones_num(Clima::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field {
                    key: "arbitraje_score",
                    label: "Arbitraje (0-5)".to_string(),
                    value: text("arbitraje_score").unwrap_or_else(|| "0".to_string()),
                    kind: FieldKind::Int { min: 0, max: 5 },
                    required: true,
                    help: None,
                },
                Field::select(
                    "tarjeta",
                    "Tarjeta",
                    partido.map_or(1, |p| p.tarjeta.as_i64()).to_string(),
                    opciones_num(Tarjeta::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field::select(
                    "dolor_fisico",
                    "Dolor físico",
                    partido.map_or(0, |p| p.dolor_fisico.as_i64()).to_string(),
                    opciones_num(DolorFisico::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field::select(
                    "atajaste",
                    "¿Atajó todo?",
                    partido
                        .map_or(0, |p| p.atajaste_todo_el_partido.as_i64())
                        .to_string(),
                    opciones_num(Atajaste::ALL.iter().map(|v| (v.as_i64(), v.label()))),
                ),
                Field {
                    key: "precio",
                    label: "Precio".to_string(),
                    value: text("precio").unwrap_or_else(|| "$0,00".to_string()),
                    kind: FieldKind::Money,
                    required: false,
                    help: None,
                },
                Field {
                    key: "temperatura",
                    label: "Temperatura °C".to_string(),
                    value: text("temperatura").unwrap_or_default(),
                    kind: FieldKind::Decimal {
                        min: -50.0,
                        max: 60.0,
                    },
                    required: false,
                    help: None,
                },
                Field::text(
                    "comentario",
                    "Comentario",
                    text("comentario").unwrap_or_default(),
                    256,
                ),
                Field::text(
                    "eventos_clave",
                    "Eventos clave",
                    text("eventos_clave").unwrap_or_default(),
                    300,
                ),
                Field::text(
                    "lo_mejor",
                    "Lo mejor",
                    text("lo_mejor").unwrap_or_default(),
                    300,
                ),
                Field::text(
                    "que_mejorar",
                    "Qué mejorar",
                    text("que_mejorar").unwrap_or_default(),
                    300,
                ),
                Field::text("tags", "Etiquetas", text("tags").unwrap_or_default(), 300),
                Field::text(
                    "goles_detalle",
                    "Detalle de goles",
                    text("goles_detalle").unwrap_or_default(),
                    512,
                ),
                Field::text(
                    "asistencias_detalle",
                    "Detalle de asistencias",
                    text("asistencias_detalle").unwrap_or_default(),
                    512,
                ),
            ],
        )
    }

    fn partido_desde_form(&self) -> Result<Partido, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let numero = |key: &str| -> Result<i64, DomainError> {
            get(key)
                .parse()
                .map_err(|_| DomainError::Message(format!("Valor inválido en {key}")))
        };
        let codigo = |key: &str| -> Result<i64, DomainError> {
            get(key)
                .parse()
                .map_err(|_| DomainError::Message(format!("Opción inválida en {key}")))
        };

        let fecha_hora = validation::datetime(&get("fecha_hora"), "fecha y hora")?;
        let temperatura = if get("temperatura").is_empty() {
            None
        } else {
            Some(
                get("temperatura")
                    .parse::<f64>()
                    .map_err(|_| DomainError::Message("Temperatura inválida".to_string()))?,
            )
        };
        let original = self.form_original_partido.as_ref();
        Ok(Partido {
            id: original.map_or(0, |partido| partido.id),
            cancha_id: numero("cancha_id")?,
            camiseta_id: numero("camiseta_id")?,
            botin_id: {
                let valor = numero("botin_id")?;
                if valor == 0 { None } else { Some(valor) }
            },
            fecha_hora,
            mes_anio: String::new(),
            goles: numero("goles")?,
            asistencias: numero("asistencias")?,
            resultado: Resultado::try_from(codigo("resultado")?)?,
            rendimiento_general: numero("rendimiento")?,
            cansancio: numero("cansancio")?,
            estado_animo: numero("estado_animo")?,
            comentario_personal: get("comentario"),
            clima: Clima::try_from(codigo("clima")?)?,
            dia: crate::domain::models::FranjaHoraria::NoDefinida,
            precio: validation::money(&get("precio"), "precio")?,
            tipo_partido: TipoPartido::try_from(codigo("tipo_partido")?)?,
            rival_nombre: get("rival_nombre"),
            tipo_rival: get("tipo_rival"),
            posicion_jugada: get("posicion_jugada"),
            minutos_jugados: numero("minutos")?,
            intensidad: numero("intensidad")?,
            condicion_cancha: String::new(),
            arbitraje: String::new(),
            eventos_clave: get("eventos_clave"),
            rating_tecnico: numero("rating_tecnico")?,
            rating_fisico: numero("rating_fisico")?,
            rating_mental: numero("rating_mental")?,
            estado_cancha: EstadoCancha::try_from(codigo("estado_cancha")?)?,
            goles_equipo: numero("goles_equipo")?,
            goles_rival: numero("goles_rival")?,
            formato_partido: FormatoPartido::from_str(&get("formato_partido"))
                .map_err(|_| DomainError::Message("Formato inválido".to_string()))?,
            tarjeta: Tarjeta::try_from(codigo("tarjeta")?)?,
            goles_en_contra: numero("goles_en_contra")?,
            dolor_fisico: DolorFisico::try_from(codigo("dolor_fisico")?)?,
            temperatura_c: temperatura,
            arbitraje_score: numero("arbitraje_score")?,
            lo_mejor: get("lo_mejor"),
            que_mejorar: get("que_mejorar"),
            tags: get("tags")
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect(),
            goles_detalle: get("goles_detalle"),
            asistencias_detalle: get("asistencias_detalle"),
            atajaste_todo_el_partido: Atajaste::try_from(codigo("atajaste")?)?,
            diferencia_gol: 0,
            favorito: original.is_some_and(|partido| partido.favorito),
        })
    }
}

fn opciones_num<'a, I>(items: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (i64, &'a str)>,
{
    items
        .into_iter()
        .map(|(value, label)| (value.to_string(), label.to_string()))
        .collect()
}

fn opciones_text<'a, I>(items: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    items
        .into_iter()
        .map(|(value, label)| (value.to_string(), label.to_string()))
        .collect()
}

fn opciones_ids<I>(items: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (i64, String)>,
{
    items
        .into_iter()
        .map(|(id, nombre)| (id.to_string(), nombre))
        .collect()
}

/// Campos del asistente por paso (claves de campo).
pub(crate) const WIZARD_PASOS: &[(&str, &[&str])] = &[
    (
        "Paso 1/4 · Básico",
        &[
            "tipo_partido",
            "fecha_hora",
            "cancha_id",
            "camiseta_id",
            "botin_id",
            "formato_partido",
            "goles",
            "asistencias",
            "goles_en_contra",
            "resultado",
            "goles_equipo",
            "goles_rival",
            "rival_nombre",
            "tipo_rival",
        ],
    ),
    (
        "Paso 2/4 · Contexto y métricas",
        &[
            "posicion_jugada",
            "minutos",
            "intensidad",
            "estado_cancha",
            "clima",
            "arbitraje_score",
            "tarjeta",
            "dolor_fisico",
            "atajaste",
            "precio",
            "temperatura",
        ],
    ),
    (
        "Paso 3/4 · Rendimiento y notas",
        &[
            "rendimiento",
            "cansancio",
            "estado_animo",
            "rating_tecnico",
            "rating_fisico",
            "rating_mental",
            "comentario",
            "eventos_clave",
            "lo_mejor",
            "que_mejorar",
            "tags",
            "goles_detalle",
            "asistencias_detalle",
        ],
    ),
];

impl App {
    /// Abre el asistente de creación de partido.
    pub(crate) fn open_partido_wizard(&mut self) -> Result<()> {
        if let Err(mensaje) = self.prerrequisitos_partido() {
            self.modal = Some(Modal::Error(mensaje));
            return Ok(());
        }
        self.form_original_partido = None;
        self.form = Some(self.partido_form(None));
        self.partido_wizard_step = 0;
        if let Some(form) = self.form.as_mut() {
            form.focus = wizard_field_indices(form, 0).first().copied().unwrap_or(0);
        }
        self.push_screen(Screen::PartidoWizard);
        Ok(())
    }

    /// Teclas del asistente.
    pub(crate) fn handle_partido_wizard_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.code == KeyCode::Esc {
            let dirty = self.form.as_ref().is_some_and(|form| form.dirty);
            if dirty {
                self.modal = Some(Modal::confirm(
                    "Descartar cambios",
                    "El asistente tiene cambios sin guardar. ¿Descartarlos? (S = sí, N = no)",
                    ConfirmAction::DiscardForm,
                ));
            } else {
                self.form = None;
                self.go_back();
            }
            return Ok(true);
        }

        if self.partido_wizard_step >= 3 {
            match key.code {
                KeyCode::Enter | KeyCode::Char('s')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.code == KeyCode::Enter =>
                {
                    self.save_partido_wizard()?;
                    return Ok(true);
                }
                KeyCode::PageUp => {
                    self.partido_wizard_step = 2;
                    self.focus_first_wizard_field();
                    return Ok(true);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::PageDown => {
                self.wizard_avanzar()?;
                return Ok(true);
            }
            KeyCode::PageUp if self.partido_wizard_step > 0 => {
                self.partido_wizard_step -= 1;
                self.focus_first_wizard_field();
                return Ok(true);
            }
            KeyCode::Tab => {
                self.wizard_move_focus(1);
                return Ok(true);
            }
            KeyCode::BackTab => {
                self.wizard_move_focus(-1);
                return Ok(true);
            }
            KeyCode::Enter => {
                self.wizard_avanzar()?;
                return Ok(true);
            }
            _ => {}
        }

        // Edición normal del campo activo.
        let is_select = self
            .form
            .as_ref()
            .and_then(|form| form.current())
            .is_some_and(|field| matches!(field.kind, FieldKind::Select { .. }));
        if let Some(form) = self.form.as_mut() {
            match key.code {
                KeyCode::Backspace => form.pop_char(),
                KeyCode::Left if is_select => form.cycle_current(false),
                KeyCode::Right if is_select => form.cycle_current(true),
                KeyCode::Char(' ') if form.current_is_toggle() => form.toggle_current(),
                KeyCode::Char(ch) => form.push_char(ch),
                _ => {}
            }
        }
        Ok(true)
    }

    fn wizard_avanzar(&mut self) -> Result<()> {
        if self.partido_wizard_step < WIZARD_PASOS.len() {
            if let Err(mensaje) = self.validar_paso_wizard() {
                self.modal = Some(Modal::Error(mensaje));
                return Ok(());
            }
            self.partido_wizard_step += 1;
            self.focus_first_wizard_field();
        } else {
            self.save_partido_wizard()?;
        }
        Ok(())
    }

    fn validar_paso_wizard(&mut self) -> std::result::Result<(), String> {
        let paso = self.partido_wizard_step.min(WIZARD_PASOS.len() - 1);
        let Some(form) = self.form.as_ref() else {
            return Err("No hay formulario activo".to_string());
        };
        for indice in wizard_field_indices(form, paso) {
            if let Err(mensaje) = form.fields[indice].validate() {
                if let Some(form) = self.form.as_mut() {
                    form.focus = indice;
                    form.error = Some(mensaje.clone());
                }
                return Err(mensaje);
            }
        }
        if let Some(form) = self.form.as_mut() {
            form.error = None;
        }
        Ok(())
    }

    fn focus_first_wizard_field(&mut self) {
        let mut destino = 0;
        if let Some(form) = self.form.as_ref() {
            destino = wizard_field_indices(form, self.partido_wizard_step)
                .first()
                .copied()
                .unwrap_or(0);
        }
        if let Some(form) = self.form.as_mut() {
            form.focus = destino;
        }
    }

    fn wizard_move_focus(&mut self, delta: isize) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        let indices = wizard_field_indices(form, self.partido_wizard_step);
        if indices.is_empty() {
            return;
        }
        let actual = indices
            .iter()
            .position(|indice| *indice == form.focus)
            .unwrap_or(0) as isize;
        let largo = indices.len() as isize;
        let siguiente = (actual + delta).rem_euclid(largo) as usize;
        let destino = indices[siguiente];
        if let Some(form) = self.form.as_mut() {
            form.focus = destino;
        }
    }

    /// Guarda el partido desde el asistente.
    pub(crate) fn save_partido_wizard(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let mut partido = match self.partido_desde_form() {
            Ok(partido) => partido,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        match partido_service::crear(self.db.conn_mut(), &mut partido) {
            Ok(_) => {
                self.form = None;
                self.form_original_partido = None;
                self.partido_wizard_step = 0;
                self.stack.pop();
                self.refresh_partidos()?;
                self.status = "Partido guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }
}

/// Índices de los campos visibles en un paso.
pub(crate) fn wizard_field_indices(form: &Form, paso: usize) -> Vec<usize> {
    let claves = WIZARD_PASOS
        .get(paso)
        .map(|(_, claves)| *claves)
        .unwrap_or(&[]);
    form.fields
        .iter()
        .enumerate()
        .filter(|(_, field)| claves.contains(&field.key))
        .map(|(indice, _)| indice)
        .collect()
}
