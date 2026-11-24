//! Colecciones e inventario (paridad con MiFutbolC).

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, ConfirmAction, Field, FieldKind, Form, Modal, Screen};
use crate::domain::colecciones::{ESTADOS_ITEM, InventarioItem, TIPOS_ITEM};
use crate::domain::dates;
use crate::domain::models::FormMode;
use crate::services::{camiseta_service, colecciones_service};

/// Qué edita el formulario de colecciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormularioColeccion {
    /// Ítem del inventario (id 0 = nuevo).
    #[default]
    Item,
    /// Colección (id 0 = nueva).
    Coleccion,
    /// Agregar el ítem indicado a una colección.
    Asignar(i64),
}

fn opciones(lista: &[(i64, &str)]) -> Vec<(String, String)> {
    lista
        .iter()
        .map(|(codigo, texto)| (codigo.to_string(), (*texto).to_string()))
        .collect()
}

impl App {
    /// Abre Colecciones.
    pub(crate) fn open_colecciones(&mut self) -> Result<()> {
        self.refresh_colecciones()?;
        if !matches!(self.current_screen(), Screen::ColeccionesList) {
            self.push_screen(Screen::ColeccionesList);
        }
        Ok(())
    }

    /// Recarga inventario y colecciones.
    pub(crate) fn refresh_colecciones(&mut self) -> Result<()> {
        let conn = self.db.conn();
        match colecciones_service::listar_items(conn)
            .and_then(|items| Ok((items, colecciones_service::listar_colecciones(conn)?)))
        {
            Ok((items, colecciones)) => {
                self.inventario = items;
                self.colecciones = colecciones;
                let ajustar = |state: &mut ratatui::widgets::TableState, total: usize| {
                    let actual = state.selected().unwrap_or(0);
                    state.select((total > 0).then(|| actual.min(total - 1)));
                };
                ajustar(&mut self.inventario_table, self.inventario.len());
                ajustar(&mut self.colecciones_table, self.colecciones.len());
            }
            Err(error) => self.status = format!("No se pudieron cargar las colecciones: {error}"),
        }
        Ok(())
    }

    fn item_seleccionado(&self) -> Option<InventarioItem> {
        self.inventario_table
            .selected()
            .and_then(|indice| self.inventario.get(indice))
            .cloned()
    }

    fn coleccion_seleccionada_id(&self) -> Option<i64> {
        self.colecciones_table
            .selected()
            .and_then(|indice| self.colecciones.get(indice))
            .map(|coleccion| coleccion.id)
    }

    /// Teclas del listado (pestañas Inventario / Colecciones).
    pub(crate) fn handle_colecciones_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        if matches!(
            key.code,
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right
        ) {
            self.colecciones_ver_colecciones = !self.colecciones_ver_colecciones;
            return Ok(true);
        }
        if key.code == KeyCode::Char('H') {
            self.open_equipamiento()?;
            return Ok(true);
        }
        if key.code == KeyCode::Char('r') {
            self.refresh_colecciones()?;
            self.status = "Colecciones actualizadas".to_string();
            return Ok(true);
        }
        if key.code == KeyCode::Char('x') {
            match colecciones_service::exportar_backup(self.db.conn(), &self.paths.export_dir) {
                Ok(ruta) => {
                    self.modal = Some(Modal::Info(format!(
                        "Backup de colecciones exportado en:
{}",
                        ruta.display()
                    )));
                }
                Err(error) => self.modal = Some(Modal::Error(error.to_string())),
            }
            return Ok(true);
        }
        if key.code == KeyCode::Char('i') {
            self.importar_backup_colecciones()?;
            return Ok(true);
        }
        if self.colecciones_ver_colecciones {
            self.handle_colecciones_tab_key(key)
        } else {
            self.handle_inventario_key(key)
        }
    }

    /// Une `Importaciones/colecciones_inventario.json` con lo existente.
    fn importar_backup_colecciones(&mut self) -> Result<()> {
        let ruta = self
            .paths
            .import_dir
            .join(colecciones_service::ARCHIVO_BACKUP);
        if !ruta.exists() {
            self.modal = Some(Modal::Error(format!("No existe {}", ruta.display())));
            return Ok(());
        }
        match colecciones_service::importar_backup(self.db.conn_mut(), &ruta) {
            Ok(resumen) => {
                self.refresh_colecciones()?;
                self.modal = Some(Modal::Info(format!(
                    "Importación completada. Colecciones procesadas: {} | Ítems procesados: {} | Relaciones nuevas: {}",
                    resumen.colecciones, resumen.items, resumen.relaciones
                )));
            }
            Err(error) => self.modal = Some(Modal::Error(error.to_string())),
        }
        Ok(())
    }

    fn handle_inventario_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.inventario_table, self.inventario.len(), 1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.inventario_table, self.inventario.len(), -1)
            }
            KeyCode::Char('n') => {
                self.coleccion_edit_id = 0;
                self.coleccion_formulario = FormularioColeccion::Item;
                self.form = Some(self.item_form(None));
                self.push_screen(Screen::ColeccionForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(item) = self.item_seleccionado() {
                    self.coleccion_edit_id = item.id;
                    self.coleccion_formulario = FormularioColeccion::Item;
                    self.form = Some(self.item_form(Some(&item)));
                    self.push_screen(Screen::ColeccionForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(item) = self.item_seleccionado() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar ítem",
                        format!("¿Eliminar «{}» del inventario?", item.nombre_visible),
                        ConfirmAction::DeleteInventarioItem(item.id),
                    ));
                }
            }
            KeyCode::Char('s') => {
                match colecciones_service::sincronizar_camisetas(self.db.conn()) {
                    Ok(cantidad) => {
                        self.refresh_colecciones()?;
                        self.status = format!("Camisetas sincronizadas: {cantidad} nuevas");
                    }
                    Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                }
            }
            KeyCode::Char('a') => {
                if let Some(item) = self.item_seleccionado() {
                    if self.colecciones.is_empty() {
                        self.status = "Primero creá una colección".to_string();
                        return Ok(true);
                    }
                    let opciones = self
                        .colecciones
                        .iter()
                        .map(|c| (c.id.to_string(), c.nombre.clone()))
                        .collect::<Vec<_>>();
                    let primera = opciones[0].0.clone();
                    self.coleccion_formulario = FormularioColeccion::Asignar(item.id);
                    self.form = Some(Form::new(
                        format!("Agregar «{}» a una colección", item.nombre_visible),
                        vec![Field::select("coleccion", "Colección", primera, opciones)],
                    ));
                    self.push_screen(Screen::ColeccionForm(FormMode::Create));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn handle_colecciones_tab_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                Self::mover_tabla_seleccion(&mut self.colecciones_table, self.colecciones.len(), 1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Self::mover_tabla_seleccion(&mut self.colecciones_table, self.colecciones.len(), -1)
            }
            KeyCode::Char('n') => {
                self.coleccion_edit_id = 0;
                self.coleccion_formulario = FormularioColeccion::Coleccion;
                self.form = Some(coleccion_form("", ""));
                self.push_screen(Screen::ColeccionForm(FormMode::Create));
            }
            KeyCode::Char('e') => {
                if let Some(coleccion) = self
                    .colecciones_table
                    .selected()
                    .and_then(|indice| self.colecciones.get(indice))
                    .cloned()
                {
                    self.coleccion_edit_id = coleccion.id;
                    self.coleccion_formulario = FormularioColeccion::Coleccion;
                    self.form = Some(coleccion_form(&coleccion.nombre, &coleccion.descripcion));
                    self.push_screen(Screen::ColeccionForm(FormMode::Edit));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self.coleccion_seleccionada_id() {
                    self.modal = Some(Modal::confirm(
                        "Eliminar colección",
                        "¿Eliminar esta colección? Los ítems siguen en el inventario.",
                        ConfirmAction::DeleteColeccion(id),
                    ));
                }
            }
            KeyCode::Enter => {
                if let Some(id) = self.coleccion_seleccionada_id() {
                    self.refresh_coleccion_detalle(id);
                    self.push_screen(Screen::ColeccionDetail(id));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn refresh_coleccion_detalle(&mut self, id: i64) {
        match colecciones_service::items_de(self.db.conn(), id) {
            Ok(items) => {
                self.coleccion_detalle = items;
                let total = self.coleccion_detalle.len();
                let actual = self.coleccion_detalle_table.selected().unwrap_or(0);
                self.coleccion_detalle_table
                    .select((total > 0).then(|| actual.min(total - 1)));
            }
            Err(error) => self.status = format!("No se pudo cargar la colección: {error}"),
        }
    }

    /// Teclas del detalle de una colección.
    pub(crate) fn handle_coleccion_detail_key(&mut self, id: i64, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => Self::mover_tabla_seleccion(
                &mut self.coleccion_detalle_table,
                self.coleccion_detalle.len(),
                1,
            ),
            KeyCode::Up | KeyCode::Char('k') => Self::mover_tabla_seleccion(
                &mut self.coleccion_detalle_table,
                self.coleccion_detalle.len(),
                -1,
            ),
            KeyCode::Char('u') => {
                if let Some(item_id) = self
                    .coleccion_detalle_table
                    .selected()
                    .and_then(|indice| self.coleccion_detalle.get(indice))
                    .map(|item| item.id)
                {
                    match colecciones_service::quitar(self.db.conn(), id, item_id) {
                        Ok(()) => {
                            self.refresh_coleccion_detalle(id);
                            self.refresh_colecciones()?;
                            self.status = "Ítem quitado de la colección".to_string();
                        }
                        Err(error) => self.modal = Some(Modal::Error(error.to_string())),
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Elimina un ítem confirmado.
    pub(crate) fn eliminar_inventario_item(&mut self, id: i64) {
        let resultado = colecciones_service::eliminar_item(self.db.conn(), id);
        self.confirmar_eliminacion(resultado, App::refresh_colecciones);
    }

    /// Elimina una colección confirmada.
    pub(crate) fn eliminar_coleccion(&mut self, id: i64) {
        let resultado = colecciones_service::eliminar_coleccion(self.db.conn(), id);
        self.confirmar_eliminacion(resultado, App::refresh_colecciones);
    }

    /// Guarda el formulario activo (ítem, colección o asignación).
    pub(crate) fn save_coleccion_form(&mut self) -> Result<()> {
        let Some(mut form) = self.form.take() else {
            return Ok(());
        };
        if let Err((_, message)) = form.validate() {
            self.form = Some(form);
            self.modal = Some(Modal::Error(message));
            return Ok(());
        }
        let texto = |clave: &str| form.value(clave).unwrap_or("").trim().to_string();
        let conn = self.db.conn();
        let resultado: Result<String, colecciones_service::ServiceError> = match self
            .coleccion_formulario
        {
            FormularioColeccion::Item => {
                let camiseta = texto("camiseta").parse::<i64>().unwrap_or(0);
                let fecha = texto("fecha_compra");
                let item = InventarioItem {
                    id: self.coleccion_edit_id,
                    tipo: texto("tipo").parse().unwrap_or(1),
                    nombre: texto("nombre"),
                    nombre_visible: String::new(),
                    estado: texto("estado").parse().unwrap_or(0),
                    valor: texto("valor").replace(',', ".").parse().unwrap_or(0.0),
                    fecha_compra: dates::parse_date(&fecha)
                        .map_or(String::new(), dates::to_iso_date),
                    camiseta_id: (camiseta > 0).then_some(camiseta),
                    colecciones: 0,
                };
                colecciones_service::guardar_item(conn, &item).map(|_| "Ítem guardado".into())
            }
            FormularioColeccion::Coleccion => colecciones_service::guardar_coleccion(
                conn,
                self.coleccion_edit_id,
                &texto("nombre"),
                &texto("descripcion"),
            )
            .map(|_| "Colección guardada".into()),
            FormularioColeccion::Asignar(item_id) => {
                colecciones_service::agregar(conn, texto("coleccion").parse().unwrap_or(0), item_id)
                    .map(|nuevo| {
                        if nuevo {
                            "Ítem agregado a la colección".into()
                        } else {
                            "El ítem ya estaba en esa colección".into()
                        }
                    })
            }
        };
        match resultado {
            Ok(mensaje) => {
                self.form = None;
                self.refresh_colecciones()?;
                self.go_back();
                self.status = mensaje;
            }
            Err(error) => {
                self.form = Some(form);
                self.modal = Some(Modal::Error(error.to_string()));
            }
        }
        Ok(())
    }

    fn item_form(&self, item: Option<&InventarioItem>) -> Form {
        let mut camisetas = vec![("0".to_string(), "Ninguna".to_string())];
        camisetas.extend(
            camiseta_service::listar(self.db.conn(), &Default::default())
                .unwrap_or_default()
                .into_iter()
                .map(|row| (row.id.to_string(), row.nombre)),
        );
        let fecha = item
            .and_then(|i| dates::parse_date(&i.fecha_compra))
            .map_or(String::new(), dates::display_date);
        Form::new(
            if item.is_some() {
                "Editar ítem"
            } else {
                "Nuevo ítem"
            },
            vec![
                Field::select(
                    "tipo",
                    "Tipo",
                    item.map_or(1, |i| i.tipo).to_string(),
                    opciones(&TIPOS_ITEM),
                ),
                Field::text(
                    "nombre",
                    "Nombre (vacío si vinculás camiseta)",
                    item.map_or_else(String::new, |i| i.nombre.clone()),
                    100,
                ),
                Field::select(
                    "camiseta",
                    "Camiseta vinculada",
                    item.and_then(|i| i.camiseta_id).unwrap_or(0).to_string(),
                    camisetas,
                ),
                Field::select(
                    "estado",
                    "Estado",
                    item.map_or(0, |i| i.estado).to_string(),
                    opciones(&ESTADOS_ITEM),
                ),
                Field {
                    key: "valor",
                    label: "Valor".to_string(),
                    value: item.map_or_else(String::new, |i| format!("{}", i.valor)),
                    kind: FieldKind::Decimal {
                        min: 0.0,
                        max: 1e12,
                    },
                    required: false,
                    help: None,
                },
                Field {
                    key: "fecha_compra",
                    label: "Fecha de compra".to_string(),
                    value: fecha,
                    kind: FieldKind::Date,
                    required: false,
                    help: Some("DD/MM/AAAA".to_string()),
                },
            ],
        )
    }
}

fn coleccion_form(nombre: &str, descripcion: &str) -> Form {
    Form::new(
        "Colección",
        vec![
            Field::text("nombre", "Nombre", nombre, 80).required(),
            Field::text("descripcion", "Descripción", descripcion, 240),
        ],
    )
}
