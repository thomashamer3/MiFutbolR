//! Historial de equipamiento (Colecciones → `H`, como el ítem 12 del menú de
//! colecciones de MiFutbolC).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::dates;
use crate::domain::equipamiento::{ESTADOS, Equipamiento, TIPOS};
use crate::domain::models::FormMode;
use crate::domain::validation;
use crate::services::equipamiento_service;

fn opciones(lista: &[(i64, &str)]) -> Vec<(String, String)> {
    lista
        .iter()
        .map(|(codigo, texto)| (codigo.to_string(), (*texto).to_string()))
        .collect()
}

impl App {
    /// Abre el historial de equipamiento.
    pub(crate) fn open_equipamiento(&mut self) -> Result<()> {
        self.refresh_equipamiento()?;
        if !matches!(self.current_screen(), Screen::EquipamientoList) {
            self.push_screen(Screen::EquipamientoList);
        }
        Ok(())
    }

    /// Recarga el listado.
    pub(crate) fn refresh_equipamiento(&mut self) -> Result<()> {
        match equipamiento_service::listar(self.db.conn()) {
            Ok(items) => {
                self.equipamiento = items;
                let total = self.equipamiento.len();
                let actual = self.equipamiento_table.selected().unwrap_or(0);
                self.equipamiento_table
                    .select((total > 0).then(|| actual.min(total - 1)));
            }
            Err(error) => self.status = format!("No se pudo cargar el equipamiento: {error}"),
        }
        Ok(())
    }

    fn equipamiento_seleccionado(&self) -> Option<Equipamiento> {
        self.equipamiento_table
            .selected()
            .and_then(|indice| self.equipamiento.get(indice))
            .cloned()
    }

    /// Teclas del listado.
    pub(crate) fn handle_equipamiento_key(&mut self, key: KeyEvent) -> Result<bool> {
        let total = self.equipamiento.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.equipamiento_table, total, 1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.equipamiento_table, total, -1);
            }
            KeyCode::Char('n') => {
                self.equipamiento_edit_id = None;
                self.form = Some(equipamiento_form(None));
                self.push_screen(Screen::EquipamientoForm(FormMode::Create));
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(item) = self.equipamiento_seleccionado() {
                    self.equipamiento_edit_id = Some(item.id);
                    self.form = Some(equipamiento_form(Some(&item)));
                    self.push_screen(Screen::EquipamientoForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(item) = self.equipamiento_seleccionado() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar ítem",
                        format!(
                            "¿Eliminar «{} {}»? (S = sí, N = no)",
                            item.marca, item.modelo
                        ),
                        ConfirmAction::DeleteEquipamiento(item.id),
                    ));
                }
            }
            KeyCode::Char('s') => match equipamiento_service::estadisticas(self.db.conn()) {
                Ok(reporte) => self.abrir_reporte(reporte),
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            },
            KeyCode::Char('r') => {
                self.refresh_equipamiento()?;
                self.status = "Equipamiento actualizado".to_string();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Guarda el formulario.
    pub(crate) fn save_equipamiento_form(&mut self, mode: FormMode) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let texto = |clave: &str| form.value(clave).unwrap_or("").trim().to_string();
        let entero = |clave: &str| texto(clave).parse::<i64>().unwrap_or(0);
        let item = validation::money(&texto("precio"), "precio").map(|precio| Equipamiento {
            id: self.equipamiento_edit_id.unwrap_or(0),
            tipo: entero("tipo"),
            marca: texto("marca"),
            modelo: texto("modelo"),
            fecha_compra: dates::parse_date(&texto("fecha_compra"))
                .map(|d| d.strftime("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            precio,
            partidos_usados: entero("partidos_usados"),
            estado_fisico: entero("estado_fisico"),
            rating: entero("rating"),
            notas: texto("notas"),
            activo: texto("activo") == "Sí",
        });
        let resultado = item.map_err(|e| e.to_string()).and_then(|item| {
            match mode {
                FormMode::Create => equipamiento_service::crear(self.db.conn(), &item).map(|_| ()),
                FormMode::Edit => equipamiento_service::actualizar(self.db.conn(), &item),
            }
            .map_err(|e| e.to_string())
        });
        match resultado {
            Ok(()) => {
                self.form = None;
                self.refresh_equipamiento()?;
                self.go_back();
                self.status = "Equipamiento guardado".to_string();
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error));
            }
        }
        Ok(())
    }
}

fn equipamiento_form(item: Option<&Equipamiento>) -> Form {
    let numero = |clave: &'static str, etiqueta: &str, valor: i64, min: i64, max: i64| Field {
        key: clave,
        label: etiqueta.to_string(),
        value: valor.to_string(),
        kind: FieldKind::Int { min, max },
        required: true,
        help: None,
    };
    let texto = |f: fn(&Equipamiento) -> String| item.map_or_else(String::new, f);
    let campos = vec![
        Field::select(
            "tipo",
            "Tipo",
            item.map_or(1, |e| e.tipo).to_string(),
            opciones(TIPOS),
        ),
        Field::text("marca", "Marca", texto(|e| e.marca.clone()), 128),
        Field::text("modelo", "Modelo", texto(|e| e.modelo.clone()), 128),
        Field::date(
            "fecha_compra",
            "Fecha de compra",
            item.and_then(|e| dates::parse_date(&e.fecha_compra))
                .map(dates::display_date)
                .unwrap_or_default(),
        ),
        Field {
            key: "precio",
            label: "Precio".to_string(),
            value: item.map_or_else(|| "$0,00".to_string(), |e| e.precio.display_ars()),
            kind: FieldKind::Money,
            required: false,
            help: None,
        },
        numero(
            "partidos_usados",
            "Partidos usados",
            item.map_or(0, |e| e.partidos_usados),
            0,
            9999,
        ),
        Field::select(
            "estado_fisico",
            "Estado físico",
            item.map_or(1, |e| e.estado_fisico).to_string(),
            opciones(ESTADOS),
        ),
        numero(
            "rating",
            "Rating (1-10)",
            item.map_or(5, |e| e.rating),
            1,
            10,
        ),
        Field::text("notas", "Notas", texto(|e| e.notas.clone()), 240),
        Field::boolean("activo", "Activo", item.is_none_or(|e| e.activo)),
    ];
    Form::new("Equipamiento", campos)
}
