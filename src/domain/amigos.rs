//! Amigos y ranking (IMP-22).

/// Amigo con estadísticas manuales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amigo {
    /// Id.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Posición.
    pub posicion: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Partidos.
    pub partidos: i64,
    /// Mejor rendimiento.
    pub mejor_rendimiento: i64,
    /// Notas.
    pub notas: String,
}

/// Fila del ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingAmigo {
    /// Nombre.
    pub nombre: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Partidos.
    pub partidos: i64,
    /// Mejor rendimiento.
    pub mejor_rendimiento: i64,
    /// ¿Es el usuario local?
    pub local: bool,
}
