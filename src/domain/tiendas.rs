//! Tiendas deportivas (IMP-20).

/// Tienda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tienda {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Tipo (física/online).
    pub tipo: String,
    /// URL.
    pub url: String,
    /// Teléfono.
    pub telefono: String,
    /// Instagram.
    pub instagram: String,
    /// Productos.
    pub productos: String,
    /// Rango de precio.
    pub rango_precio: String,
    /// ¿Tiene envío?
    pub tiene_envio: bool,
    /// ¿Favorita?
    pub favorito: bool,
    /// Notas.
    pub notas: String,
}
