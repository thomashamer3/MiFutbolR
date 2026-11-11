//! Modelos de dominio: structs y enums del negocio.
//!
//! Los nombres de campos siguen la especificación (§7.1). Los enums numéricos
//! replican los códigos legacy y exponen `as_i64`, `label`, `ALL` y
//! `TryFrom<i64>`; los enums guardados como texto exponen `as_str`/`from_str`.

use chrono::{NaiveDate, NaiveDateTime};

use super::money::Cents;
use super::validation::DomainError;

/// Declara un enum numérico con conversión, etiqueta y catálogo completo.
macro_rules! int_enum {
    ($name:ident { $($variant:ident = $value:expr => $label:expr),+ $(,)? }) => {
        /// Enum numérico persistido como `INTEGER`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(i64)]
        pub enum $name {
            $(
                /// Variante del enum.
                $variant = $value,
            )+
        }

        impl $name {
            /// Todas las variantes en orden de presentación.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// Código numérico persistido.
            #[must_use]
            pub const fn as_i64(self) -> i64 {
                self as i64
            }

            /// Etiqueta en español para la UI.
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $($name::$variant => $label),+
                }
            }

            /// Convierte desde el código de la BD.
            pub fn from_i64(value: i64) -> Result<Self, DomainError> {
                match value {
                    $($value => Ok($name::$variant)),+,
                    other => Err(DomainError::InvalidEnum {
                        field: stringify!($name),
                        value: other,
                    }),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.label())
            }
        }

        impl TryFrom<i64> for $name {
            type Error = DomainError;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                Self::from_i64(value)
            }
        }
    };
}

int_enum!(Resultado {
    NoAplica = 0 => "No aplica",
    Victoria = 1 => "Victoria",
    Empate = 2 => "Empate",
    Derrota = 3 => "Derrota",
});

int_enum!(TipoPartido {
    Amistoso = 1 => "Amistoso",
    Torneo = 2 => "Torneo",
    Entrenamiento = 3 => "Entrenamiento",
});

int_enum!(Clima {
    Despejado = 1 => "Despejado",
    Nublado = 2 => "Nublado",
    Lluvia = 3 => "Lluvia",
    Ventoso = 4 => "Ventoso",
    MuchoCalor = 5 => "Mucho calor",
    MuchoFrio = 6 => "Mucho frío",
    Frio = 7 => "Frío",
    Calor = 8 => "Calor",
    LloviznaLeve = 9 => "Llovizna leve",
    LluviaModerada = 10 => "Lluvia moderada",
    LluviaFuerte = 11 => "Lluvia fuerte",
    CanchaInundada = 12 => "Cancha inundada",
});

int_enum!(EstadoCancha {
    NoDefinido = 0 => "No definido",
    Excelente = 1 => "Excelente",
    Buena = 2 => "Buena",
    Regular = 3 => "Regular",
    Mala = 4 => "Mala",
    Pesima = 5 => "Pésima",
});

int_enum!(Tarjeta {
    No = 1 => "Sin tarjeta",
    Amarilla = 2 => "Amarilla",
    Roja = 3 => "Roja",
});

int_enum!(DolorFisico {
    Ninguna = 0 => "Ninguna",
    Leve = 1 => "Leve",
    Moderada = 2 => "Moderada",
    Fuerte = 3 => "Fuerte",
});

int_enum!(Arbitraje {
    NoDefinido = 0 => "No definido",
    MuyMalo = 1 => "Muy malo",
    Regular = 2 => "Regular",
    Normal = 3 => "Normal",
    Bueno = 4 => "Bueno",
    Excelente = 5 => "Excelente",
});

int_enum!(FranjaHoraria {
    NoDefinida = 0 => "No definida",
    Madrugada = 1 => "Madrugada",
    Manana = 2 => "Mañana",
    Mediodia = 3 => "Mediodía",
    Tarde = 4 => "Tarde",
    Atardecer = 5 => "Atardecer",
    Noche = 6 => "Noche",
});

int_enum!(TipoEquipo {
    Fijo = 0 => "Fijo",
    Momentaneo = 1 => "Momentáneo",
});

int_enum!(TipoFutbol {
    F5 = 0 => "F5",
    F7 = 1 => "F7",
    F8 = 2 => "F8",
    F11 = 3 => "F11",
});

int_enum!(Posicion {
    Arquero = 0 => "Arquero",
    Defensor = 1 => "Defensor",
    Mediocampista = 2 => "Mediocampista",
    Delantero = 3 => "Delantero",
});

int_enum!(TipoTransaccion {
    Ingreso = 0 => "Ingreso",
    Gasto = 1 => "Gasto",
});

int_enum!(CategoriaFinanciera {
    Transporte = 0 => "Transporte",
    Equipamiento = 1 => "Equipamiento",
    Cuotas = 2 => "Cuotas",
    Torneos = 3 => "Torneos",
    Arbitraje = 4 => "Arbitraje",
    Canchas = 5 => "Canchas",
    Medicina = 6 => "Medicina",
    Otros = 7 => "Otros",
});

int_enum!(TipoCancha {
    F5 = 5 => "F5",
    F7 = 7 => "F7",
    F8 = 8 => "F8",
    F11 = 11 => "F11",
    Otro = 99 => "Otro",
});

int_enum!(Superficie {
    NoSe = 0 => "No se",
    Natural = 1 => "Natural",
    Sintetico = 2 => "Sintético",
    Cemento = 3 => "Cemento",
    Otra = 4 => "Otra",
});

int_enum!(TechadaEstado {
    Descubierta = 0 => "Descubierta",
    Techada = 1 => "Techada",
    NoSe = 2 => "No se",
});

int_enum!(Atajaste {
    NoEspecificado = 0 => "No especificado",
    Si = 1 => "Sí",
    No = 2 => "No",
});

int_enum!(Theme {
    Light = 0 => "Claro",
    Dark = 1 => "Oscuro",
    Blue = 2 => "Azul",
    Green = 3 => "Verde",
    Red = 4 => "Rojo",
    Purple = 5 => "Púrpura",
    Classic = 6 => "Clásico",
    HighContrast = 7 => "Alto contraste",
});

int_enum!(Language {
    Espanol = 0 => "Español",
    English = 1 => "English",
});

int_enum!(Mode {
    Simple = 0 => "Simple",
    Advanced = 1 => "Avanzado",
    Custom = 2 => "Personalizado",
});

int_enum!(TextSize {
    Small = 0 => "Compacto",
    Medium = 1 => "Normal",
    Large = 2 => "Grande",
});

/// Estado físico de una camiseta (se guarda como `TEXT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EstadoFisico {
    /// Recién comprada.
    Nueva,
    /// En buen estado.
    Buena,
    /// Con uso visible.
    Desgastada,
    /// Dañada.
    Rota,
    /// Otra situación.
    Otra,
}

impl EstadoFisico {
    /// Todas las variantes.
    pub const ALL: &[Self] = &[
        Self::Nueva,
        Self::Buena,
        Self::Desgastada,
        Self::Rota,
        Self::Otra,
    ];

    /// Texto persistido.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nueva => "Nueva",
            Self::Buena => "Buena",
            Self::Desgastada => "Desgastada",
            Self::Rota => "Rota",
            Self::Otra => "Otra",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.as_str()
    }

    /// Convierte desde el texto de la BD (tolerante a mayúsculas).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        let normalized = value.trim().to_lowercase();
        match normalized.as_str() {
            "nueva" => Ok(Self::Nueva),
            "buena" => Ok(Self::Buena),
            "desgastada" => Ok(Self::Desgastada),
            "rota" => Ok(Self::Rota),
            "otra" | "" => Ok(Self::Otra),
            other => Err(DomainError::Message(format!(
                "Estado físico inválido: {other}"
            ))),
        }
    }
}

impl std::fmt::Display for EstadoFisico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Estado de una lesión (se guarda como `TEXT` con mayúsculas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EstadoLesion {
    /// Lesión reciente.
    Activa,
    /// En tratamiento.
    EnTratamiento,
    /// Evolucionando bien.
    Mejorando,
    /// Dada de alta.
    Recuperado,
    /// Volvió a lesionarse.
    Recaida,
}

impl EstadoLesion {
    /// Todas las variantes.
    pub const ALL: &[Self] = &[
        Self::Activa,
        Self::EnTratamiento,
        Self::Mejorando,
        Self::Recuperado,
        Self::Recaida,
    ];

    /// Texto persistido (compatible con el CHECK de la tabla).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Activa => "ACTIVA",
            Self::EnTratamiento => "EN TRATAMIENTO",
            Self::Mejorando => "MEJORANDO",
            Self::Recuperado => "RECUPERADO",
            Self::Recaida => "RECAIDA",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.as_str()
    }

    /// Convierte desde el texto de la BD.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value.trim().to_uppercase().as_str() {
            "ACTIVA" => Ok(Self::Activa),
            "EN TRATAMIENTO" => Ok(Self::EnTratamiento),
            "MEJORANDO" => Ok(Self::Mejorando),
            "RECUPERADO" => Ok(Self::Recuperado),
            "RECAIDA" | "RECAÍDA" => Ok(Self::Recaida),
            other => Err(DomainError::Message(format!(
                "Estado de lesión inválido: {other}"
            ))),
        }
    }
}

impl std::fmt::Display for EstadoLesion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Formato de partido (se guarda como `TEXT`: F5/F7/F8/F11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatoPartido {
    /// Fútbol 5.
    F5,
    /// Fútbol 7.
    F7,
    /// Fútbol 8.
    F8,
    /// Fútbol 11.
    F11,
}

impl FormatoPartido {
    /// Todas las variantes.
    pub const ALL: &[Self] = &[Self::F5, Self::F7, Self::F8, Self::F11];

    /// Texto persistido.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F5 => "F5",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F11 => "F11",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.as_str()
    }

    /// Convierte desde el texto de la BD (`''` → F5 por defecto).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value.trim().to_uppercase().as_str() {
            "F5" | "" => Ok(Self::F5),
            "F7" => Ok(Self::F7),
            "F8" => Ok(Self::F8),
            "F11" => Ok(Self::F11),
            other => Err(DomainError::Message(format!(
                "Formato de partido inválido: {other}"
            ))),
        }
    }

    /// Equivalente numérico de `equipo.tipo_futbol`.
    #[must_use]
    pub const fn tipo_futbol(self) -> TipoFutbol {
        match self {
            Self::F5 => TipoFutbol::F5,
            Self::F7 => TipoFutbol::F7,
            Self::F8 => TipoFutbol::F8,
            Self::F11 => TipoFutbol::F11,
        }
    }
}

impl std::fmt::Display for FormatoPartido {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Modo de un formulario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    /// Alta de una entidad nueva.
    Create,
    /// Edición de una entidad existente.
    Edit,
}

impl FormMode {
    /// `true` si es edición.
    #[must_use]
    pub const fn is_edit(self) -> bool {
        matches!(self, Self::Edit)
    }
}

/// Tipo de racha de resultados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RachaTipo {
    /// Serie de victorias.
    Positiva,
    /// Serie de derrotas.
    Negativa,
    /// Sin serie relevante.
    Neutral,
}

impl RachaTipo {
    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Positiva => "Racha positiva",
            Self::Negativa => "Racha negativa",
            Self::Neutral => "Sin racha",
        }
    }
}

/// Forma física deseada por el usuario para el próximo partido.
#[derive(Debug, Clone)]
pub struct Camiseta {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Color principal.
    pub color_principal: String,
    /// Color secundario.
    pub color_secundario: String,
    /// Marca.
    pub marca: String,
    /// Modelo.
    pub modelo: String,
    /// Temporada.
    pub temporada: String,
    /// Estado físico.
    pub estado_fisico: EstadoFisico,
    /// Fecha de compra.
    pub fecha_compra: Option<NaiveDate>,
    /// Costo.
    pub costo: Cents,
    /// Observaciones.
    pub observaciones: String,
    /// Proveedor.
    pub proveedor: String,
    /// ¿Fue regalo?
    pub fue_regalo: bool,
    /// ¿De quién?
    pub regalo_de: String,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
    /// ¿Activa en catálogo?
    pub activa: bool,
    /// ¿Ya salió en el sorteo?
    pub sorteada: bool,
}

/// Botín (calzado).
#[derive(Debug, Clone)]
pub struct Botin {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
    /// ¿Activo en catálogo?
    pub activa: bool,
    /// ¿Ya salió en el sorteo?
    pub sorteada: bool,
}

/// Servicios disponibles en una cancha.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServiciosCancha {
    /// Vestuarios.
    pub vestuarios: bool,
    /// Duchas.
    pub duchas: bool,
    /// Buffet.
    pub buffet: bool,
    /// Estacionamiento.
    pub estacionamiento: bool,
}

/// Cancha.
#[derive(Debug, Clone)]
pub struct Cancha {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Teléfono.
    pub telefono: String,
    /// Dirección.
    pub direccion: String,
    /// Localidad.
    pub localidad: String,
    /// Tipos de cancha (selección múltiple).
    pub tipos: Vec<TipoCancha>,
    /// Superficies (selección múltiple).
    pub superficies: Vec<Superficie>,
    /// ¿Techada?
    pub techada: TechadaEstado,
    /// Iluminación.
    pub tiene_iluminacion: bool,
    /// Apertura en minutos desde las 00:00.
    pub horario_apertura_min: Option<i64>,
    /// Cierre en minutos desde las 00:00.
    pub horario_cierre_min: Option<i64>,
    /// Precio de hora diurna.
    pub precio_dia: Cents,
    /// Precio de hora nocturna.
    pub precio_noche: Cents,
    /// Servicios.
    pub servicios: ServiciosCancha,
    /// Cantidad de canchas.
    pub cantidad_canchas: i64,
    /// Estado (texto libre).
    pub estado: String,
    /// Descripción.
    pub descripcion: String,
    /// Contacto alternativo.
    pub contacto_alt: String,
    /// Latitud.
    pub latitud: Option<f64>,
    /// Longitud.
    pub longitud: Option<f64>,
    /// ¿Tiene grabación?
    pub tiene_grabacion: bool,
    /// Marcador (0/1/2).
    pub tiene_marcador: TechadaEstado,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
    /// ¿Activa en catálogo?
    pub activa: bool,
}

/// Jugador de un equipo.
#[derive(Debug, Clone)]
pub struct Jugador {
    /// Identificador.
    pub id: i64,
    /// Equipo al que pertenece.
    pub equipo_id: i64,
    /// Nombre.
    pub nombre: String,
    /// Dorsal (0–99).
    pub numero: i64,
    /// Posición.
    pub posicion: Posicion,
    /// ¿Capitán?
    pub es_capitan: bool,
}

/// Equipo.
#[derive(Debug, Clone)]
pub struct Equipo {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Tipo.
    pub tipo: TipoEquipo,
    /// Tipo de fútbol.
    pub tipo_futbol: TipoFutbol,
    /// Cantidad de jugadores.
    pub num_jugadores: i64,
    /// Partido asociado (si es momentáneo).
    pub partido_id: Option<i64>,
    /// ¿Activo en catálogo?
    pub activa: bool,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
    /// Jugadores del equipo.
    pub jugadores: Vec<Jugador>,
}

/// Partido jugado.
#[derive(Debug, Clone)]
pub struct Partido {
    /// Identificador.
    pub id: i64,
    /// Cancha.
    pub cancha_id: i64,
    /// Camiseta.
    pub camiseta_id: i64,
    /// Botín.
    pub botin_id: Option<i64>,
    /// Fecha y hora.
    pub fecha_hora: NaiveDateTime,
    /// Clave `YYYY-MM` (cacheada).
    pub mes_anio: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Resultado.
    pub resultado: Resultado,
    /// Rendimiento general (0–10).
    pub rendimiento_general: i64,
    /// Cansancio (0–10).
    pub cansancio: i64,
    /// Estado de ánimo (0–10).
    pub estado_animo: i64,
    /// Comentario personal.
    pub comentario_personal: String,
    /// Clima.
    pub clima: Clima,
    /// Franja horaria.
    pub dia: FranjaHoraria,
    /// Precio pagado.
    pub precio: Cents,
    /// Tipo de partido.
    pub tipo_partido: TipoPartido,
    /// Rival.
    pub rival_nombre: String,
    /// Tipo de rival.
    pub tipo_rival: String,
    /// Posición jugada.
    pub posicion_jugada: String,
    /// Minutos jugados.
    pub minutos_jugados: i64,
    /// Intensidad (0–10).
    pub intensidad: i64,
    /// Condición de la cancha (texto).
    pub condicion_cancha: String,
    /// Arbitraje (texto).
    pub arbitraje: String,
    /// Eventos clave.
    pub eventos_clave: String,
    /// Rating técnico (0–10).
    pub rating_tecnico: i64,
    /// Rating físico (0–10).
    pub rating_fisico: i64,
    /// Rating mental (0–10).
    pub rating_mental: i64,
    /// Estado de la cancha.
    pub estado_cancha: EstadoCancha,
    /// Goles del equipo (-1 = no cargado).
    pub goles_equipo: i64,
    /// Goles del rival (-1 = no cargado).
    pub goles_rival: i64,
    /// Formato.
    pub formato_partido: FormatoPartido,
    /// Tarjeta.
    pub tarjeta: Tarjeta,
    /// Goles en contra.
    pub goles_en_contra: i64,
    /// Dolor físico.
    pub dolor_fisico: DolorFisico,
    /// Temperatura.
    pub temperatura_c: Option<f64>,
    /// Puntaje de arbitraje (0–5).
    pub arbitraje_score: i64,
    /// Lo mejor del partido.
    pub lo_mejor: String,
    /// Qué mejorar.
    pub que_mejorar: String,
    /// Etiquetas.
    pub tags: Vec<String>,
    /// Detalle de goles.
    pub goles_detalle: String,
    /// Detalle de asistencias.
    pub asistencias_detalle: String,
    /// ¿Atajó todo el partido?
    pub atajaste_todo_el_partido: Atajaste,
    /// Diferencia de gol (derivada).
    pub diferencia_gol: i64,
    /// ¿Favorito?
    pub favorito: bool,
}

/// Lesión registrada.
#[derive(Debug, Clone)]
pub struct Lesion {
    /// Identificador.
    pub id: i64,
    /// Jugador (texto libre).
    pub jugador: String,
    /// Tipo de lesión.
    pub tipo: String,
    /// Descripción.
    pub descripcion: String,
    /// Fecha y hora.
    pub fecha: NaiveDateTime,
    /// Camiseta que se usaba.
    pub camiseta_id: i64,
    /// Estado.
    pub estado: EstadoLesion,
    /// Partido asociado.
    pub partido_id: Option<i64>,
}

/// Transacción financiera.
#[derive(Debug, Clone)]
pub struct TransaccionFinanciera {
    /// Identificador.
    pub id: i64,
    /// Fecha.
    pub fecha: NaiveDate,
    /// Ingreso o gasto.
    pub tipo: TipoTransaccion,
    /// Categoría.
    pub categoria: CategoriaFinanciera,
    /// Descripción.
    pub descripcion: String,
    /// Monto.
    pub monto: Cents,
    /// Ítem específico.
    pub item_especifico: String,
    /// Partido relacionado.
    pub partido_id: Option<i64>,
}

/// Presupuesto mensual.
#[derive(Debug, Clone, PartialEq)]
pub struct PresupuestoMensual {
    /// Identificador.
    pub id: i64,
    /// Clave `YYYY-MM`.
    pub mes_anio: String,
    /// Presupuesto total.
    pub presupuesto_total: Cents,
    /// Límite de gasto.
    pub limite_gasto: Cents,
    /// ¿Alertas activas?
    pub alertas_habilitadas: bool,
    /// Fecha de creación.
    pub fecha_creacion: NaiveDate,
    /// Fecha de modificación.
    pub fecha_modificacion: NaiveDate,
}

/// Identidad de la carrera futbolística.
#[derive(Debug, Clone, Default)]
pub struct CarreraIdentidad {
    /// Nombre.
    pub nombre: String,
    /// Edad.
    pub edad: i64,
    /// Pie hábil.
    pub pie_habil: String,
    /// Posiciones.
    pub posiciones: String,
    /// Altura en cm.
    pub altura_cm: Option<f64>,
    /// Peso en kg.
    pub peso_kg: Option<f64>,
    /// Estilo de juego.
    pub estilo: String,
    /// Dorsal favorito.
    pub dorsal_favorito: i64,
    /// Objetivos.
    pub objetivos: String,
    /// Historia.
    pub historia: String,
}

/// Hito de carrera asociado a un partido.
#[derive(Debug, Clone)]
pub struct HitoCarrera {
    /// Identificador.
    pub id: i64,
    /// Partido.
    pub partido_id: i64,
    /// Tipo de hito.
    pub tipo_hito: String,
    /// Nota.
    pub nota: String,
    /// Fecha de creación.
    pub created_at: NaiveDateTime,
}

/// Preferencias de la aplicación.
#[derive(Debug, Clone, PartialEq)]
pub struct AppSettings {
    /// Tema.
    pub theme: Theme,
    /// Idioma.
    pub language: Language,
    /// Modo de menú.
    pub mode: Mode,
    /// Tamaño de texto.
    pub text_size: TextSize,
    /// Visor de imágenes externo.
    pub image_viewer: String,
    /// Botín predeterminado.
    pub botin_predeterminado: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            language: Language::Espanol,
            mode: Mode::Simple,
            text_size: TextSize::Medium,
            image_viewer: String::new(),
            botin_predeterminado: 0,
        }
    }
}

/// Estado del filtro de listados de catálogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstadoFiltro {
    /// Todas las filas.
    #[default]
    Todas,
    /// Solo activas.
    Activas,
    /// Solo inactivas.
    Inactivas,
}

impl EstadoFiltro {
    /// Código que espera el SQL (0=todas, 1=activas, 2=inactivas).
    #[must_use]
    pub const fn sql_code(self) -> i64 {
        match self {
            Self::Todas => 0,
            Self::Activas => 1,
            Self::Inactivas => 2,
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Todas => "Todas",
            Self::Activas => "Activas",
            Self::Inactivas => "Inactivas",
        }
    }

    /// Siguiente estado del ciclo (para la tecla de filtro).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Todas => Self::Activas,
            Self::Activas => Self::Inactivas,
            Self::Inactivas => Self::Todas,
        }
    }
}

/// Orden del listado de camisetas (§8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenCamisetas {
    /// Por id ascendente.
    #[default]
    Id,
    /// Por nombre.
    Nombre,
    /// Por cantidad de partidos.
    Partidos,
    /// Por goles.
    Goles,
    /// Por victorias.
    Victorias,
}

impl OrdenCamisetas {
    /// Todos los órdenes disponibles.
    pub const ALL: &[Self] = &[
        Self::Id,
        Self::Nombre,
        Self::Partidos,
        Self::Goles,
        Self::Victorias,
    ];

    /// Fragmento `ORDER BY` seguro (sin interpolación de datos).
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::Id => "c.id ASC",
            Self::Nombre => "c.nombre ASC, c.id ASC",
            Self::Partidos => "partidos DESC, c.id ASC",
            Self::Goles => "goles DESC, c.id ASC",
            Self::Victorias => "victorias DESC, c.id ASC",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Nombre => "Nombre",
            Self::Partidos => "Partidos",
            Self::Goles => "Goles",
            Self::Victorias => "Victorias",
        }
    }

    /// Siguiente orden del ciclo.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Id => Self::Nombre,
            Self::Nombre => Self::Partidos,
            Self::Partidos => Self::Goles,
            Self::Goles => Self::Victorias,
            Self::Victorias => Self::Id,
        }
    }
}

/// Fila del listado de camisetas con agregados de partidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamisetaRow {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// ¿Activa?
    pub activa: bool,
    /// Partidos jugados con esta camiseta.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
}

/// Filtro del listado de camisetas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CamisetaFilter {
    /// Texto de búsqueda (nombre, `LIKE`).
    pub texto: String,
    /// Estado activo/inactivo.
    pub estado: EstadoFiltro,
    /// Orden.
    pub orden: OrdenCamisetas,
}

/// Ficha de una camiseta con estadísticas derivadas (§8.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FichaCamiseta {
    /// Partidos jugados.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Lesiones asociadas.
    pub lesiones: i64,
}

/// Dependencias de una camiseta antes de borrar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CamisetaDependencias {
    /// Partidos que la usan.
    pub partidos: i64,
    /// Lesiones asociadas.
    pub lesiones: i64,
}

impl CamisetaDependencias {
    /// `true` si hay cualquier dependencia.
    #[must_use]
    pub const fn has_dependencies(self) -> bool {
        self.partidos > 0 || self.lesiones > 0
    }
}

/// Orden del listado de canchas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenCanchas {
    /// Por id.
    #[default]
    Id,
    /// Por nombre.
    Nombre,
    /// Por partidos jugados.
    Partidos,
    /// Por rendimiento promedio.
    Rendimiento,
}

impl OrdenCanchas {
    /// Todos los órdenes.
    pub const ALL: &[Self] = &[Self::Id, Self::Nombre, Self::Partidos, Self::Rendimiento];

    /// Fragmento `ORDER BY` estático.
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::Id => "c.id ASC",
            Self::Nombre => "c.nombre ASC, c.id ASC",
            Self::Partidos => "partidos DESC, c.id ASC",
            Self::Rendimiento => "rendimiento_promedio DESC, c.id ASC",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Nombre => "Nombre",
            Self::Partidos => "Partidos",
            Self::Rendimiento => "Rendimiento",
        }
    }

    /// Siguiente orden del ciclo.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Id => Self::Nombre,
            Self::Nombre => Self::Partidos,
            Self::Partidos => Self::Rendimiento,
            Self::Rendimiento => Self::Id,
        }
    }
}

/// Fila del listado de canchas con agregados.
#[derive(Debug, Clone, PartialEq)]
pub struct CanchaRow {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Localidad.
    pub localidad: String,
    /// ¿Activa?
    pub activa: bool,
    /// Partidos jugados.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
    /// Rendimiento general promedio.
    pub rendimiento_promedio: f64,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
}

/// Filtro del listado de canchas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CanchaFilter {
    /// Búsqueda por nombre o localidad.
    pub texto: String,
    /// Estado.
    pub estado: EstadoFiltro,
    /// Orden.
    pub orden: OrdenCanchas,
}

/// Dependencias de una cancha antes de borrar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CanchaDependencias {
    /// Partidos jugados allí.
    pub partidos: i64,
}

impl CanchaDependencias {
    /// `true` si hay partidos.
    #[must_use]
    pub const fn has_dependencies(self) -> bool {
        self.partidos > 0
    }
}

/// Orden del listado de botines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenBotines {
    /// Por id.
    #[default]
    Id,
    /// Por nombre.
    Nombre,
    /// Por partidos.
    Partidos,
}

impl OrdenBotines {
    /// Todos los órdenes.
    pub const ALL: &[Self] = &[Self::Id, Self::Nombre, Self::Partidos];

    /// Fragmento `ORDER BY` estático.
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::Id => "b.id ASC",
            Self::Nombre => "b.nombre ASC, b.id ASC",
            Self::Partidos => "partidos DESC, b.id ASC",
        }
    }

    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Nombre => "Nombre",
            Self::Partidos => "Partidos",
        }
    }

    /// Siguiente orden del ciclo.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Id => Self::Nombre,
            Self::Nombre => Self::Partidos,
            Self::Partidos => Self::Id,
        }
    }
}

/// Fila del listado de botines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotinRow {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// ¿Activo?
    pub activa: bool,
    /// ¿Ya sorteado?
    pub sorteada: bool,
    /// Partidos usándolo.
    pub partidos: i64,
    /// ¿Es el predeterminado?
    pub predeterminado: bool,
}

/// Filtro del listado de botines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BotinFilter {
    /// Búsqueda por nombre.
    pub texto: String,
    /// Estado.
    pub estado: EstadoFiltro,
    /// Orden.
    pub orden: OrdenBotines,
}

/// Dependencias de un botín.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BotinDependencias {
    /// Partidos que lo usaron.
    pub partidos: i64,
}

impl BotinDependencias {
    /// `true` si hay partidos.
    #[must_use]
    pub const fn has_dependencies(self) -> bool {
        self.partidos > 0
    }
}

/// Orden del listado de equipos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenEquipos {
    /// Por id.
    #[default]
    Id,
    /// Por nombre.
    Nombre,
    /// Por cantidad de jugadores.
    Jugadores,
}

impl OrdenEquipos {
    /// Todos los órdenes.
    pub const ALL: &[Self] = &[Self::Id, Self::Nombre, Self::Jugadores];

    /// Fragmento `ORDER BY` estático.
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::Id => "e.id ASC",
            Self::Nombre => "e.nombre ASC, e.id ASC",
            Self::Jugadores => "jugadores DESC, e.id ASC",
        }
    }

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Nombre => "Nombre",
            Self::Jugadores => "Jugadores",
        }
    }

    /// Siguiente del ciclo.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Id => Self::Nombre,
            Self::Nombre => Self::Jugadores,
            Self::Jugadores => Self::Id,
        }
    }
}

/// Fila del listado de equipos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipoRow {
    /// Identificador.
    pub id: i64,
    /// Nombre.
    pub nombre: String,
    /// Fijo o momentáneo.
    pub tipo: TipoEquipo,
    /// Modalidad.
    pub tipo_futbol: TipoFutbol,
    /// Jugadores declarados.
    pub num_jugadores: i64,
    /// Jugadores cargados.
    pub jugadores_count: i64,
    /// ¿Activo?
    pub activa: bool,
    /// Ruta de la imagen.
    pub imagen_ruta: String,
}

/// Filtro del listado de equipos.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipoFilter {
    /// Búsqueda por nombre.
    pub texto: String,
    /// Estado.
    pub estado: EstadoFiltro,
    /// Orden.
    pub orden: OrdenEquipos,
}

/// Dependencias que impiden borrar un equipo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EquipoDependencias {
    /// Partidos de torneo.
    pub partido_torneo: i64,
    /// Historial de equipos.
    pub equipo_historial: i64,
    /// Torneos que lo fijan.
    pub torneos: i64,
}

impl EquipoDependencias {
    /// `true` si hay dependencias.
    #[must_use]
    pub const fn has_dependencies(self) -> bool {
        self.partido_torneo > 0 || self.equipo_historial > 0 || self.torneos > 0
    }
}

/// Orden del listado de partidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenPartidos {
    /// Más recientes primero.
    #[default]
    FechaDesc,
    /// Más antiguos primero.
    FechaAsc,
}

impl OrdenPartidos {
    /// Todos los órdenes.
    pub const ALL: &[Self] = &[Self::FechaDesc, Self::FechaAsc];

    /// Fragmento `ORDER BY` estático.
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::FechaDesc => "p.fecha_hora DESC, p.id DESC",
            Self::FechaAsc => "p.fecha_hora ASC, p.id ASC",
        }
    }

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FechaDesc => "Recientes",
            Self::FechaAsc => "Antiguos",
        }
    }

    /// Alterna el orden.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::FechaDesc => Self::FechaAsc,
            Self::FechaAsc => Self::FechaDesc,
        }
    }
}

/// Fila del listado de partidos con nombres resueltos.
#[derive(Debug, Clone, PartialEq)]
pub struct PartidoRow {
    /// Identificador.
    pub id: i64,
    /// Fecha y hora.
    pub fecha_hora: NaiveDateTime,
    /// Cancha.
    pub cancha: String,
    /// Camiseta.
    pub camiseta: String,
    /// Rival.
    pub rival: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Resultado.
    pub resultado: Resultado,
    /// Tipo.
    pub tipo_partido: TipoPartido,
    /// Precio pagado.
    pub precio: Cents,
    /// ¿Favorito?
    pub favorito: bool,
}

/// Filtro del listado de partidos.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartidoFilter {
    /// Texto en rival, comentario o tags.
    pub texto: String,
    /// Cancha.
    pub cancha_id: Option<i64>,
    /// Camiseta.
    pub camiseta_id: Option<i64>,
    /// Resultado.
    pub resultado: Option<Resultado>,
    /// Tipo de partido.
    pub tipo_partido: Option<TipoPartido>,
    /// Desde (fecha inclusive).
    pub desde: Option<NaiveDate>,
    /// Hasta (fecha inclusive).
    pub hasta: Option<NaiveDate>,
    /// Solo favoritos.
    pub favorito: bool,
    /// Orden.
    pub orden: OrdenPartidos,
}

/// Estadísticas rápidas del pie del listado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PartidoResumen {
    /// Partidos que cumplen el filtro.
    pub total: i64,
    /// Goles totales.
    pub goles: i64,
    /// Asistencias totales.
    pub asistencias: i64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
}

/// Orden del listado de lesiones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrdenLesiones {
    /// Más recientes primero.
    #[default]
    FechaDesc,
    /// Más antiguas primero.
    FechaAsc,
}

impl OrdenLesiones {
    /// Todos los órdenes.
    pub const ALL: &[Self] = &[Self::FechaDesc, Self::FechaAsc];

    /// Fragmento `ORDER BY` estático.
    #[must_use]
    pub const fn sql_order(self) -> &'static str {
        match self {
            Self::FechaDesc => "l.fecha DESC, l.id DESC",
            Self::FechaAsc => "l.fecha ASC, l.id ASC",
        }
    }

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FechaDesc => "Recientes",
            Self::FechaAsc => "Antiguas",
        }
    }

    /// Alterna el orden.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::FechaDesc => Self::FechaAsc,
            Self::FechaAsc => Self::FechaDesc,
        }
    }
}

/// Fila del listado de lesiones con nombres resueltos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LesionRow {
    /// Identificador.
    pub id: i64,
    /// Jugador.
    pub jugador: String,
    /// Tipo.
    pub tipo: String,
    /// Fecha.
    pub fecha: NaiveDateTime,
    /// Estado.
    pub estado: EstadoLesion,
    /// Camiseta usada.
    pub camiseta: String,
    /// Partido asociado (`id · fecha` o vacío).
    pub partido: String,
}

/// Filtro del listado de lesiones.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LesionFilter {
    /// Búsqueda por jugador, tipo o descripción.
    pub texto: String,
    /// Estado, o `None` para todos.
    pub estado: Option<EstadoLesion>,
    /// Orden.
    pub orden: OrdenLesiones,
}

/// Estadísticas agregadas de lesiones (§13.5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LesionStats {
    /// Total.
    pub total: i64,
    /// Activas.
    pub activas: i64,
    /// Recuperadas.
    pub recuperadas: i64,
    /// Días promedio entre lesiones consecutivas.
    pub promedio_dias: Option<f64>,
    /// Conteo por tipo (de mayor a menor).
    pub por_tipo: Vec<(String, i64)>,
    /// Conteo por mes `YYYY-MM`.
    pub por_mes: Vec<(String, i64)>,
}

/// Categoría del hub de estadísticas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstadisticaKind {
    /// Resumen general y récords.
    #[default]
    Generales,
    /// Serie mensual.
    PorMes,
    /// Serie anual.
    PorAnio,
    /// Rankings de rendimiento.
    Rendimiento,
    /// Estadísticas de canchas.
    Canchas,
    /// Goles y asistencias.
    Goles,
    /// Meta-análisis.
    Meta,
}

impl EstadisticaKind {
    /// Todas las categorías.
    pub const ALL: &[Self] = &[
        Self::Generales,
        Self::PorMes,
        Self::PorAnio,
        Self::Rendimiento,
        Self::Canchas,
        Self::Goles,
        Self::Meta,
    ];

    /// Etiqueta.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Generales => "Generales",
            Self::PorMes => "Por mes",
            Self::PorAnio => "Por año",
            Self::Rendimiento => "Rendimiento",
            Self::Canchas => "Canchas",
            Self::Goles => "Goles y asistencias",
            Self::Meta => "Meta-análisis",
        }
    }

    /// Descripción para el hub.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Generales => "Totales, promedios, rachas, consistencia y récords.",
            Self::PorMes => "Partidos, goles, asistencias y rendimiento por mes.",
            Self::PorAnio => "Serie anual comparativa.",
            Self::Rendimiento => "Clima, día, franja, cansancio, ánimo, intensidad y tendencia.",
            Self::Canchas => "Uso, goles y rendimiento por cancha, tipo y localidad.",
            Self::Goles => "Promedios, eficiencia, efectividad y partidos atípicos.",
            Self::Meta => "Consistencia, outliers, contexto, impacto y eficiencia.",
        }
    }
}

/// Fila de promedio agrupado.
#[derive(Debug, Clone, PartialEq)]
pub struct PromedioGrupo {
    /// Clave legible del grupo.
    pub clave: String,
    /// Cantidad de partidos.
    pub partidos: i64,
    /// Promedio principal.
    pub promedio: f64,
}

/// Agregados generales (§14.2).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EstadisticasGenerales {
    /// Total de partidos.
    pub total: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Promedio de goles.
    pub promedio_goles: f64,
    /// Promedio de asistencias.
    pub promedio_asistencias: f64,
    /// Promedio de rendimiento.
    pub promedio_rendimiento: f64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
    /// Porcentaje de victorias.
    pub porcentaje_victorias: f64,
    /// Racha actual: etiqueta y largo.
    pub racha_actual: Option<(String, usize)>,
    /// Mejor racha de victorias.
    pub mejor_racha: usize,
    /// Desviación estándar del rendimiento.
    pub consistencia: f64,
    /// Mejor partido (texto).
    pub mejor_partido: Option<String>,
    /// Peor partido (texto).
    pub peor_partido: Option<String>,
}

/// Agregado mensual/anual.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PeriodoStats {
    /// Clave `YYYY-MM` u `YYYY`.
    pub clave: String,
    /// Partidos.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Rendimiento promedio.
    pub rendimiento: f64,
}

/// Ranking de canchas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CanchaStats {
    /// Nombre.
    pub nombre: String,
    /// Partidos.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Rendimiento promedio.
    pub rendimiento: f64,
}

/// Métricas de rendimiento (§14.3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RendimientoStats {
    /// Promedio general.
    pub promedio: f64,
    /// Por clima.
    pub por_clima: Vec<PromedioGrupo>,
    /// Por día de la semana.
    pub por_dia_semana: Vec<PromedioGrupo>,
    /// Por franja horaria.
    pub por_franja: Vec<PromedioGrupo>,
    /// Por cansancio.
    pub por_cansancio: Vec<PromedioGrupo>,
    /// Por estado de ánimo.
    pub por_animo: Vec<PromedioGrupo>,
    /// Por intensidad.
    pub por_intensidad: Vec<PromedioGrupo>,
    /// Por dolor físico.
    pub por_dolor: Vec<PromedioGrupo>,
    /// Por arbitraje.
    pub por_arbitraje: Vec<PromedioGrupo>,
    /// Por rango de temperatura.
    pub por_temperatura: Vec<PromedioGrupo>,
    /// Últimos rendimientos (sparkline).
    pub tendencia: Vec<f64>,
}

/// Métricas de canchas (§14.4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CanchasStats {
    /// Total de canchas.
    pub total: i64,
    /// Activas.
    pub activas: i64,
    /// Partidos jugados en canchas.
    pub partidos: i64,
    /// Ranking de canchas.
    pub ranking: Vec<CanchaStats>,
    /// Techadas vs descubiertas.
    pub por_techada: Vec<PromedioGrupo>,
    /// Por tipo de cancha.
    pub por_tipo: Vec<PromedioGrupo>,
    /// Por superficie.
    pub por_superficie: Vec<PromedioGrupo>,
    /// Por franja horaria.
    pub por_franja: Vec<PromedioGrupo>,
    /// Por localidad.
    pub por_localidad: Vec<PromedioGrupo>,
}

/// Métricas de meta-análisis (IMP-15).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetaStats {
    /// Partidos analizados.
    pub partidos: i64,
    /// Promedio de rendimiento.
    pub promedio: f64,
    /// Desvío estándar.
    pub desvio: f64,
    /// Coeficiente de variación (%).
    pub cv: f64,
    /// Mínimo.
    pub minimo: i64,
    /// Máximo.
    pub maximo: i64,
    /// Cantidad de partidos atípicos (|z| > 2).
    pub outliers: i64,
    /// Rendimiento con cansancio alto.
    pub cansancio_alto: f64,
    /// Rendimiento con cansancio bajo.
    pub cansancio_bajo: f64,
    /// Rendimiento con ánimo alto.
    pub animo_alto: f64,
    /// Rendimiento con ánimo bajo.
    pub animo_bajo: f64,
    /// Goles por partido.
    pub goles_por_partido: f64,
    /// Asistencias por partido.
    pub asistencias_por_partido: f64,
    /// Mejor contexto (cancha, promedio).
    pub mejor_contexto: Option<(String, f64)>,
    /// Peor contexto (cancha, promedio).
    pub peor_contexto: Option<(String, f64)>,
    /// Últimos rendimientos.
    pub tendencia: Vec<f64>,
}

/// Métricas de goles y asistencias (§14.5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GolesStats {
    /// Promedio de goles.
    pub promedio_goles: f64,
    /// Promedio de asistencias.
    pub promedio_asistencias: f64,
    /// Goles por clima.
    pub por_clima: Vec<PromedioGrupo>,
    /// Goles por franja.
    pub por_franja: Vec<PromedioGrupo>,
    /// Goles promedio con cansancio alto.
    pub goles_cansancio_alto: f64,
    /// Goles promedio con cansancio bajo.
    pub goles_cansancio_bajo: f64,
    /// Eficiencia (goles / rendimiento).
    pub eficiencia: f64,
    /// Efectividad ((goles+asistencias)/partidos).
    pub efectividad: f64,
    /// Partidos atípicos (|z| > 2 en goles).
    pub atipicos: i64,
    /// Partidos exigentes bien rendidos.
    pub exigentes_bien: i64,
    /// Partidos fáciles mal rendidos.
    pub faciles_mal: i64,
}

/// Paquete completo de estadísticas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Estadisticas {
    /// Generales.
    pub generales: EstadisticasGenerales,
    /// Mensuales.
    pub por_mes: Vec<PeriodoStats>,
    /// Anuales.
    pub por_anio: Vec<PeriodoStats>,
    /// Récords como `(nombre, valor)`.
    pub records: Vec<(String, String)>,
    /// Rendimiento.
    pub rendimiento: RendimientoStats,
    /// Canchas.
    pub canchas: CanchasStats,
    /// Goles.
    pub goles: GolesStats,
    /// Meta-análisis.
    pub meta: MetaStats,
}

/// Filtro del listado de transacciones.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FinanciamientoFilter {
    /// Tipo (ingreso/gasto).
    pub tipo: Option<TipoTransaccion>,
    /// Categoría.
    pub categoria: Option<CategoriaFinanciera>,
    /// Desde (inclusive).
    pub desde: Option<NaiveDate>,
    /// Hasta (inclusive).
    pub hasta: Option<NaiveDate>,
    /// Texto en descripción o ítem.
    pub texto: String,
}

/// Resumen financiero del filtro.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FinanzasResumen {
    /// Ingresos.
    pub ingresos: Cents,
    /// Gastos (valor positivo).
    pub gastos: Cents,
    /// Balance (ingresos - gastos).
    pub balance: Cents,
}

/// Total gastado por categoría.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriaTotal {
    /// Categoría.
    pub categoria: CategoriaFinanciera,
    /// Cantidad de transacciones.
    pub cantidad: i64,
    /// Total gastado.
    pub total: Cents,
}

/// Nivel de alerta del presupuesto mensual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertaPresupuesto {
    /// Dentro de los límites.
    #[default]
    Ok,
    /// Superó el límite de gasto.
    LimiteSuperado,
    /// Superó el presupuesto total.
    PresupuestoSuperado,
}

impl AlertaPresupuesto {
    /// Etiqueta para la UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "Dentro del presupuesto",
            Self::LimiteSuperado => "ALERTA: límite de gasto superado",
            Self::PresupuestoSuperado => "ALERTA FUERTE: presupuesto total superado",
        }
    }
}

/// Presupuesto con el gasto real y su alerta.
#[derive(Debug, Clone, PartialEq)]
pub struct PresupuestoVista {
    /// Presupuesto configurado.
    pub presupuesto: PresupuestoMensual,
    /// Gasto real del mes.
    pub gasto_real: Cents,
    /// Alerta calculada.
    pub alerta: AlertaPresupuesto,
}

/// Logro de carrera calculado (§16.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarreraBadge {
    /// Nombre.
    pub nombre: String,
    /// Condición o descripción.
    pub descripcion: String,
    /// ¿Obtenido?
    pub obtenido: bool,
}

/// Resumen de carrera (§16.4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarreraResumen {
    /// Partidos.
    pub partidos: i64,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Victorias.
    pub victorias: i64,
    /// Empates.
    pub empates: i64,
    /// Derrotas.
    pub derrotas: i64,
    /// Rendimiento promedio.
    pub rendimiento_promedio: f64,
    /// Mejor año por aporte (goles + asistencias).
    pub mejor_anio: Option<String>,
    /// Mejor racha de victorias.
    pub mejor_racha: usize,
    /// Títulos.
    pub titulos: i64,
    /// Podios.
    pub podios: i64,
    /// Temporadas registradas.
    pub temporadas: i64,
    /// Badges calculados.
    pub badges: Vec<CarreraBadge>,
}

/// Hito de carrera con datos del partido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitoRow {
    /// Identificador.
    pub id: i64,
    /// Partido.
    pub partido_id: i64,
    /// Fecha del partido.
    pub fecha: NaiveDateTime,
    /// Rival.
    pub rival: String,
    /// Tipo de hito.
    pub tipo_hito: String,
    /// Nota.
    pub nota: String,
}

/// Evento de la línea de tiempo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvento {
    /// Fecha.
    pub fecha: NaiveDateTime,
    /// Categoría (Partido, Hito, Lesión…).
    pub categoria: String,
    /// Descripción.
    pub descripcion: String,
}

/// Partido destacado del Hall of Fame.
#[derive(Debug, Clone, PartialEq)]
pub struct HallPartido {
    /// Identificador.
    pub id: i64,
    /// Fecha.
    pub fecha: NaiveDateTime,
    /// Rival.
    pub rival: String,
    /// Goles.
    pub goles: i64,
    /// Asistencias.
    pub asistencias: i64,
    /// Rendimiento.
    pub rendimiento: i64,
}

/// Hall of Fame personal (§16.5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarreraHall {
    /// Top 5 partidos por score.
    pub partidos: Vec<HallPartido>,
    /// Mejor racha.
    pub mejor_racha: usize,
    /// Racha actual.
    pub racha_actual: Option<(String, usize)>,
}

/// Perfil dinámico: últimos 5 vs anteriores 5.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarreraPerfil {
    /// Etiqueta: En racha / Estable / Bajón.
    pub etiqueta: String,
    /// Delta de rendimiento.
    pub delta_rendimiento: f64,
    /// Goles recientes.
    pub goles_recientes: i64,
    /// Goles previos.
    pub goles_previos: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn numeric_enums_roundtrip() {
        for value in Resultado::ALL {
            assert_eq!(Resultado::try_from(value.as_i64()).unwrap(), *value);
        }
        for value in CategoriaFinanciera::ALL {
            assert_eq!(
                CategoriaFinanciera::try_from(value.as_i64()).unwrap(),
                *value
            );
        }
        assert!(Resultado::try_from(99).is_err());
        assert!(Clima::try_from(0).is_err());
    }

    #[test]
    fn labels_are_distinct_and_non_empty() {
        for value in Resultado::ALL {
            assert!(!value.label().is_empty());
        }
        assert_eq!(TipoPartido::Amistoso.as_i64(), 1);
        assert_eq!(Theme::HighContrast.as_i64(), 7);
    }

    #[test]
    fn text_enums_roundtrip() {
        for value in EstadoFisico::ALL {
            assert_eq!(EstadoFisico::from_str(value.as_str()).unwrap(), *value);
        }
        for value in EstadoLesion::ALL {
            assert_eq!(EstadoLesion::from_str(value.as_str()).unwrap(), *value);
        }
        for value in FormatoPartido::ALL {
            assert_eq!(FormatoPartido::from_str(value.as_str()).unwrap(), *value);
        }
        assert_eq!(
            EstadoLesion::from_str("mejorando").unwrap(),
            EstadoLesion::Mejorando
        );
        assert!(EstadoLesion::from_str("otro").is_err());
    }

    #[test]
    fn filter_cycles_cover_all_variants() {
        let mut estado = EstadoFiltro::default();
        for _ in 0..3 {
            estado = estado.next();
        }
        assert_eq!(estado, EstadoFiltro::Todas);

        let mut orden = OrdenCamisetas::default();
        for _ in 0..OrdenCamisetas::ALL.len() {
            orden = orden.next();
        }
        assert_eq!(orden, OrdenCamisetas::Id);
        assert_eq!(EstadoFiltro::Inactivas.sql_code(), 2);
    }

    #[test]
    fn defaults_are_sane() {
        let settings = AppSettings::default();
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.language, Language::Espanol);
    }
}
