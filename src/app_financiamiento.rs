//! Navegación, formularios y presupuesto del módulo Financiamiento.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    App, ConfirmAction, Field, FieldKind, Form, Modal, Screen, SearchState, SearchTarget,
};
use crate::db::helpers::Page;
use crate::domain::dates;
use crate::domain::models::{
    CategoriaFinanciera, FormMode, PresupuestoMensual, TipoTransaccion, TransaccionFinanciera,
};
use crate::domain::validation::{self, DomainError};
use crate::services::financiamiento_service;
use crate::services::partido_service;

impl App {
    /// Teclas del listado.
    pub(crate) fn handle_financiamiento_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        self.ensure_mes();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_transaccion_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_transaccion_selection(-1),
            KeyCode::Char('n') => self.open_transaccion_form(FormMode::Create)?,
            KeyCode::Char('e') => {
                if let Some(id) = self.selected_transaccion_id() {
                    self.open_transaccion_form_for(id)?;
                }
            }
            KeyCode::Char('d') => self.confirm_delete_transaccion(),
            KeyCode::Char('p') => self.open_presupuesto_form()?,
            KeyCode::Char('f') => {
                self.finanzas_filter.tipo = match self.finanzas_filter.tipo {
                    None => Some(TipoTransaccion::Ingreso),
                    Some(TipoTransaccion::Ingreso) => Some(TipoTransaccion::Gasto),
                    _ => None,
                };
                self.refresh_financiamiento()?;
            }
            KeyCode::Char('c') => {
                self.finanzas_filter.categoria = match self.finanzas_filter.categoria {
                    None => Some(CategoriaFinanciera::Transporte),
                    Some(categoria) => siguiente_categoria(categoria),
                };
                self.refresh_financiamiento()?;
            }
            KeyCode::Char('/') => {
                self.search = Some(SearchState {
                    target: SearchTarget::Financiamiento,
                    buffer: String::new(),
                });
            }
            KeyCode::Char('r') => {
                self.refresh_financiamiento()?;
                self.status = "Finanzas actualizadas".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn ensure_mes(&mut self) {
        if self.finanzas_mes.is_empty() {
            self.finanzas_mes = financiamiento_service::mes_actual();
        }
    }

    /// Recarga listado, resumen, categorías y presupuesto.
    pub fn refresh_financiamiento(&mut self) -> Result<()> {
        self.ensure_mes();
        self.transacciones = financiamiento_service::listar(self.db.conn(), &self.finanzas_filter)?;
        self.finanzas_resumen =
            financiamiento_service::resumen(self.db.conn(), &self.finanzas_filter)?;
        self.finanzas_categorias = financiamiento_service::por_categoria(self.db.conn())?;
        self.presupuesto = financiamiento_service::presupuesto(self.db.conn(), &self.finanzas_mes)?;
        if self.transacciones.is_empty() {
            self.transacciones_table.select(None);
        } else {
            let index = self
                .transacciones_table
                .selected()
                .unwrap_or(0)
                .min(self.transacciones.len() - 1);
            self.transacciones_table.select(Some(index));
        }
        Ok(())
    }

    /// Id seleccionado.
    #[must_use]
    pub fn selected_transaccion_id(&self) -> Option<i64> {
        let index = self.transacciones_table.selected()?;
        self.transacciones.get(index).map(|row| row.id)
    }

    fn move_transaccion_selection(&mut self, delta: isize) {
        if self.transacciones.is_empty() {
            return;
        }
        let len = self.transacciones.len() as isize;
        let current = self
            .transacciones_table
            .selected()
            .unwrap_or(0)
            .min(self.transacciones.len() - 1) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.transacciones_table.select(Some(next));
    }

    fn open_transaccion_form(&mut self, mode: FormMode) -> Result<()> {
        match mode {
            FormMode::Create => {
                self.form_original_transaccion = None;
                self.form = Some(self.transaccion_form(None));
                self.push_screen(Screen::TransaccionForm(FormMode::Create));
            }
            FormMode::Edit => {
                if let Some(id) = self.selected_transaccion_id() {
                    self.open_transaccion_form_for(id)?;
                }
            }
        }
        Ok(())
    }

    fn open_transaccion_form_for(&mut self, id: i64) -> Result<()> {
        match financiamiento_service::obtener(self.db.conn(), id) {
            Ok(transaccion) => {
                self.form = Some(self.transaccion_form(Some(&transaccion)));
                self.form_original_transaccion = Some(transaccion);
                self.push_screen(Screen::TransaccionForm(FormMode::Edit));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn confirm_delete_transaccion(&mut self) {
        let Some(id) = self.selected_transaccion_id() else {
            self.status = "No hay transacción seleccionada".to_string();
            return;
        };
        self.modal = Some(Modal::confirm(
            "Eliminar transacción",
            "¿Eliminar la transacción? El partido asociado no se modifica. (S = sí, N = no)",
            ConfirmAction::DeleteTransaccion(id),
        ));
    }

    /// Elimina una transacción.
    pub(crate) fn delete_transaccion(&mut self, id: i64) -> Result<()> {
        match financiamiento_service::eliminar(self.db.conn(), id) {
            Ok(()) => {
                self.status = "Transacción eliminada".to_string();
                self.refresh_financiamiento()?;
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    /// Guarda una transacción.
    pub(crate) fn save_transaccion_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let transaccion = match self.transaccion_desde_form() {
            Ok(transaccion) => transaccion,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        let result = match mode {
            FormMode::Create => {
                let mut nueva = transaccion;
                financiamiento_service::crear(self.db.conn(), &mut nueva).map(|_| ())
            }
            FormMode::Edit => financiamiento_service::actualizar(self.db.conn(), &transaccion),
        };

        match result {
            Ok(()) => {
                self.form = None;
                self.form_original_transaccion = None;
                self.stack.pop();
                self.refresh_financiamiento()?;
                self.status = "Transacción guardada".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn transaccion_form(&self, transaccion: Option<&TransaccionFinanciera>) -> Form {
        let partidos =
            partido_service::listar(self.db.conn(), &Default::default(), Page::numbered(1, 50))
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    (
                        row.id.to_string(),
                        format!(
                            "#{} {} {}",
                            row.id,
                            dates::display_datetime(row.fecha_hora),
                            row.rival
                        ),
                    )
                });
        let mut partido_opciones = vec![("0".to_string(), "Ninguno".to_string())];
        partido_opciones.extend(partidos);

        let tipo = transaccion.map_or(1, |transaccion| transaccion.tipo.as_i64());
        let categoria = transaccion.map_or(5, |transaccion| transaccion.categoria.as_i64());
        let partido_actual =
            transaccion.map_or(0, |transaccion| transaccion.partido_id.unwrap_or(0));

        Form::new(
            if transaccion.is_some() {
                "Editar transacción"
            } else {
                "Nueva transacción"
            },
            vec![
                Field::text(
                    "descripcion",
                    "Descripción",
                    transaccion
                        .map_or_else(String::new, |transaccion| transaccion.descripcion.clone()),
                    200,
                )
                .required(),
                Field {
                    key: "fecha",
                    label: "Fecha".to_string(),
                    value: transaccion.map_or_else(
                        || dates::display_date(dates::today()),
                        |transaccion| dates::display_date(transaccion.fecha),
                    ),
                    kind: FieldKind::Date,
                    required: true,
                    help: Some("DD/MM/AAAA".to_string()),
                },
                Field::select(
                    "tipo",
                    "Tipo",
                    tipo.to_string(),
                    TipoTransaccion::ALL
                        .iter()
                        .map(|valor| (valor.as_i64().to_string(), valor.label().to_string()))
                        .collect(),
                ),
                Field::select(
                    "categoria",
                    "Categoría",
                    categoria.to_string(),
                    CategoriaFinanciera::ALL
                        .iter()
                        .map(|valor| (valor.as_i64().to_string(), valor.label().to_string()))
                        .collect(),
                ),
                Field {
                    key: "monto",
                    label: "Monto".to_string(),
                    value: transaccion.map_or_else(
                        || "$0,00".to_string(),
                        |transaccion| transaccion.monto.display_ars(),
                    ),
                    kind: FieldKind::Money,
                    required: true,
                    help: None,
                },
                Field::text(
                    "item",
                    "Ítem específico",
                    transaccion.map_or_else(String::new, |transaccion| {
                        transaccion.item_especifico.clone()
                    }),
                    120,
                ),
                Field::select(
                    "partido_id",
                    "Partido",
                    partido_actual.to_string(),
                    partido_opciones,
                ),
            ],
        )
    }

    fn transaccion_desde_form(&self) -> Result<TransaccionFinanciera, DomainError> {
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
        let partido_id = numero("partido_id")?;
        let original = self.form_original_transaccion.as_ref();
        Ok(TransaccionFinanciera {
            id: original.map_or(0, |transaccion| transaccion.id),
            fecha: validation::date(&get("fecha"), "fecha")?,
            tipo: TipoTransaccion::try_from(numero("tipo")?)?,
            categoria: CategoriaFinanciera::try_from(numero("categoria")?)?,
            descripcion: get("descripcion"),
            monto: validation::money(&get("monto"), "monto")?,
            item_especifico: get("item"),
            partido_id: if partido_id == 0 {
                None
            } else {
                Some(partido_id)
            },
        })
    }

    fn open_presupuesto_form(&mut self) -> Result<()> {
        self.ensure_mes();
        let actual = financiamiento_service::presupuesto(self.db.conn(), &self.finanzas_mes)?;
        self.form = Some(presupuesto_form(actual.as_ref(), &self.finanzas_mes));
        self.push_screen(Screen::PresupuestoForm);
        Ok(())
    }

    /// Guarda el presupuesto del mes.
    pub(crate) fn save_presupuesto_form(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        self.form = Some(form);

        let presupuesto = match self.presupuesto_desde_form() {
            Ok(presupuesto) => presupuesto,
            Err(error) => {
                self.modal = Some(Modal::Error(error.to_string()));
                return Ok(());
            }
        };

        match financiamiento_service::guardar_presupuesto(self.db.conn(), &presupuesto) {
            Ok(()) => {
                self.form = None;
                self.stack.pop();
                self.refresh_financiamiento()?;
                self.status = "Presupuesto guardado".to_string();
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn presupuesto_desde_form(&self) -> Result<PresupuestoMensual, DomainError> {
        let form = self
            .form
            .as_ref()
            .ok_or_else(|| DomainError::Message("No hay formulario activo".to_string()))?;
        let get = |key: &str| form.value(key).unwrap_or("").trim().to_string();
        let existente = self.presupuesto.as_ref();
        Ok(PresupuestoMensual {
            id: existente.map_or(0, |vista| vista.presupuesto.id),
            mes_anio: get("mes"),
            presupuesto_total: validation::money(&get("presupuesto_total"), "presupuesto total")?,
            limite_gasto: validation::money(&get("limite_gasto"), "límite de gasto")?,
            alertas_habilitadas: get("alertas") == "Sí",
            fecha_creacion: existente
                .map_or_else(dates::today, |vista| vista.presupuesto.fecha_creacion),
            fecha_modificacion: dates::today(),
        })
    }
}

/// Formulario de presupuesto mensual.
fn presupuesto_form(
    actual: Option<&crate::domain::models::PresupuestoVista>,
    mes_default: &str,
) -> Form {
    let presupuesto = actual.map(|vista| &vista.presupuesto);
    Form::new(
        "Presupuesto mensual",
        vec![
            Field::text(
                "mes",
                "Mes (YYYY-MM)",
                presupuesto.map_or_else(|| mes_default.to_string(), |p| p.mes_anio.clone()),
                7,
            )
            .required(),
            Field {
                key: "presupuesto_total",
                label: "Presupuesto total".to_string(),
                value: presupuesto.map_or_else(
                    || "$0,00".to_string(),
                    |p| p.presupuesto_total.display_ars(),
                ),
                kind: FieldKind::Money,
                required: true,
                help: None,
            },
            Field {
                key: "limite_gasto",
                label: "Límite de gasto".to_string(),
                value: presupuesto
                    .map_or_else(|| "$0,00".to_string(), |p| p.limite_gasto.display_ars()),
                kind: FieldKind::Money,
                required: true,
                help: None,
            },
            Field::boolean(
                "alertas",
                "¿Alertas habilitadas?",
                presupuesto.is_none_or(|p| p.alertas_habilitadas),
            ),
        ],
    )
}

fn siguiente_categoria(categoria: CategoriaFinanciera) -> Option<CategoriaFinanciera> {
    match categoria {
        CategoriaFinanciera::Transporte => Some(CategoriaFinanciera::Equipamiento),
        CategoriaFinanciera::Equipamiento => Some(CategoriaFinanciera::Cuotas),
        CategoriaFinanciera::Cuotas => Some(CategoriaFinanciera::Torneos),
        CategoriaFinanciera::Torneos => Some(CategoriaFinanciera::Arbitraje),
        CategoriaFinanciera::Arbitraje => Some(CategoriaFinanciera::Canchas),
        CategoriaFinanciera::Canchas => Some(CategoriaFinanciera::Medicina),
        CategoriaFinanciera::Medicina => Some(CategoriaFinanciera::Otros),
        CategoriaFinanciera::Otros => None,
    }
}
