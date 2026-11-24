//! Historial de equipamiento (`equipamiento_historial.c`).

use crate::domain::money::Cents;

/// Tipos de ítem de C (`item_tipo_texto`).
pub const TIPOS: &[(i64, &str)] = &[
    (1, "Botines"),
    (2, "Camiseta"),
    (3, "Short"),
    (4, "Canilleras"),
    (5, "Guantes"),
    (6, "Otro"),
];

/// Estados físicos de C.
pub const ESTADOS: &[(i64, &str)] = &[
    (1, "Nuevo"),
    (2, "Bueno"),
    (3, "Usado"),
    (4, "Gastado"),
    (5, "Descarte"),
];

/// Texto de un código de una de las tablas anteriores.
#[must_use]
pub fn etiqueta(tabla: &[(i64, &'static str)], codigo: i64) -> &'static str {
    tabla
        .iter()
        .find(|(c, _)| *c == codigo)
        .map_or("?", |(_, texto)| texto)
}

/// Ítem de equipamiento ya comprado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equipamiento {
    /// Id (0 = nuevo).
    pub id: i64,
    /// Tipo 1-6.
    pub tipo: i64,
    /// Marca.
    pub marca: String,
    /// Modelo.
    pub modelo: String,
    /// Fecha de compra ISO o vacía.
    pub fecha_compra: String,
    /// Precio.
    pub precio: Cents,
    /// Partidos en que se usó.
    pub partidos_usados: i64,
    /// Estado físico 1-5.
    pub estado_fisico: i64,
    /// Puntaje personal 1-10.
    pub rating: i64,
    /// Notas.
    pub notas: String,
    /// ¿Se sigue usando?
    pub activo: bool,
}
