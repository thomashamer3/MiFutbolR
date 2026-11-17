//! Colecciones e inventario de equipamiento (paridad con MiFutbolC).

/// Tipos de ítem: (código, etiqueta).
pub const TIPOS_ITEM: [(i64, &str); 3] = [(1, "Camiseta"), (2, "Botines"), (3, "Accesorio")];
/// Estados de ítem: (código, etiqueta).
pub const ESTADOS_ITEM: [(i64, &str); 2] = [(0, "Nuevo"), (1, "Usado")];

/// Etiqueta de un código de una lista `(código, etiqueta)`.
#[must_use]
pub fn etiqueta(lista: &[(i64, &'static str)], codigo: i64) -> &'static str {
    lista
        .iter()
        .find(|(valor, _)| *valor == codigo)
        .map_or("?", |(_, texto)| texto)
}

/// Ítem del inventario.
#[derive(Debug, Clone, PartialEq)]
pub struct InventarioItem {
    /// Id.
    pub id: i64,
    /// 1 Camiseta, 2 Botines, 3 Accesorio.
    pub tipo: i64,
    /// Nombre propio (vacío si es una camiseta vinculada).
    pub nombre: String,
    /// Nombre a mostrar (el de la camiseta vinculada si corresponde).
    pub nombre_visible: String,
    /// 0 Nuevo, 1 Usado.
    pub estado: i64,
    /// Valor.
    pub valor: f64,
    /// Fecha de compra ISO (o vacío).
    pub fecha_compra: String,
    /// Camiseta vinculada.
    pub camiseta_id: Option<i64>,
    /// En cuántas colecciones está.
    pub colecciones: i64,
}

/// Colección.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coleccion {
    /// Id.
    pub id: i64,
    /// Nombre (único).
    pub nombre: String,
    /// Descripción.
    pub descripcion: String,
    /// Cantidad de ítems.
    pub items: i64,
}
