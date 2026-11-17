//! Bienestar (paridad con MiFutbolC): hábitos, entrenamientos, comidas,
//! sesiones mentales, perfil de salud y asistente de preparación.
//!
//! Las secciones son tablas planas; se describen con columnas para compartir
//! un único CRUD, formulario y tabla.

/// Tipo de dato de una columna.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TipoDato {
    /// Fecha ISO `YYYY-MM-DD` (en pantalla `DD/MM/AAAA`; vacío = hoy).
    Fecha,
    /// Entero en rango.
    Entero(i64, i64),
    /// Decimal opcional en rango.
    Decimal(f64, f64),
    /// Booleano guardado como 0/1.
    Booleano,
    /// Una de las opciones (se guarda el texto, como MiFutbolC).
    Opcion(&'static [&'static str]),
    /// Texto libre con longitud máxima.
    Texto(usize),
}

/// Columna editable de una sección.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columna {
    /// Nombre de la columna SQL (lista cerrada: nunca viene del usuario).
    pub clave: &'static str,
    /// Etiqueta visible.
    pub etiqueta: &'static str,
    /// Tipo.
    pub tipo: TipoDato,
}

const fn col(clave: &'static str, etiqueta: &'static str, tipo: TipoDato) -> Columna {
    Columna {
        clave,
        etiqueta,
        tipo,
    }
}

const ESCALA: TipoDato = TipoDato::Entero(0, 10);

/// Tipos de entrenamiento (valores de MiFutbolC).
pub const TIPOS_ENTRENAMIENTO: &[&str] = &["Fuerza", "Resistencia", "Tecnica", "Recuperacion"];
/// Tipos de comida.
pub const TIPOS_COMIDA: &[&str] = &["Desayuno", "Almuerzo", "Cena", "Snack"];
/// Calidad de comida.
pub const CALIDADES: &[&str] = &["Mala", "Regular", "Buena"];
/// Tipos de sesión mental.
pub const TIPOS_MENTAL: &[&str] = &["Charla", "Visualizacion", "Reflexion"];
/// Momento de la sesión mental respecto de un partido.
pub const MOMENTOS: &[&str] = &["N/A", "Antes", "Despues"];

/// Sección del módulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seccion {
    /// Mentalidad y hábitos diarios.
    Habitos,
    /// Entrenamientos.
    Entrenamientos,
    /// Comidas.
    Comidas,
    /// Sesiones mentales.
    Mental,
    /// Perfil de salud (una sola fila).
    Salud,
}

impl Seccion {
    /// Orden de las pestañas.
    pub const ALL: [Self; 5] = [
        Self::Habitos,
        Self::Entrenamientos,
        Self::Comidas,
        Self::Mental,
        Self::Salud,
    ];

    /// Etiqueta de la pestaña.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Habitos => "Hábitos",
            Self::Entrenamientos => "Entrenamientos",
            Self::Comidas => "Comidas",
            Self::Mental => "Mental",
            Self::Salud => "Salud",
        }
    }

    /// Tabla SQL.
    #[must_use]
    pub const fn tabla(self) -> &'static str {
        match self {
            Self::Habitos => "bienestar_habito",
            Self::Entrenamientos => "bienestar_entrenamiento",
            Self::Comidas => "bienestar_comida",
            Self::Mental => "bienestar_sesion_mental",
            Self::Salud => "bienestar_salud",
        }
    }

    /// Columnas editables, en orden de formulario y tabla.
    #[must_use]
    pub const fn columnas(self) -> &'static [Columna] {
        match self {
            Self::Habitos => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("dormi_bien", "Dormí bien", TipoDato::Booleano),
                    col("hidratacion", "Hidratación (0-10)", ESCALA),
                    col("alcohol", "Alcohol", TipoDato::Booleano),
                    col("estado_animico", "Estado anímico", TipoDato::Texto(40)),
                    col("nervios", "Nervios (0-10)", ESCALA),
                    col("confianza", "Confianza (0-10)", ESCALA),
                    col("motivacion", "Motivación (0-10)", ESCALA),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Entrenamientos => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_ENTRENAMIENTO)),
                    col("duracion_min", "Duración (min)", TipoDato::Entero(1, 600)),
                    col("intensidad", "Intensidad (1-10)", TipoDato::Entero(1, 10)),
                    col("omitido", "Omitido", TipoDato::Booleano),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Comidas => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_COMIDA)),
                    col("calidad", "Calidad", TipoDato::Opcion(CALIDADES)),
                    col("descripcion", "Descripción", TipoDato::Texto(240)),
                ];
                C
            }
            Self::Mental => {
                const C: &[Columna] = &[
                    col("fecha", "Fecha", TipoDato::Fecha),
                    col("tipo", "Tipo", TipoDato::Opcion(TIPOS_MENTAL)),
                    col("momento", "Momento", TipoDato::Opcion(MOMENTOS)),
                    col("confianza", "Confianza (0-10)", ESCALA),
                    col("estres", "Estrés (0-10)", ESCALA),
                    col("motivacion", "Motivación (0-10)", ESCALA),
                    col("presion", "Presión (0-10)", ESCALA),
                    col("concentracion", "Concentración (0-10)", ESCALA),
                    col(
                        "pensamientos_clave",
                        "Pensamientos clave",
                        TipoDato::Texto(240),
                    ),
                    col("texto_libre", "Texto libre", TipoDato::Texto(400)),
                ];
                C
            }
            Self::Salud => {
                const C: &[Columna] = &[
                    col("altura_cm", "Altura (cm)", TipoDato::Decimal(0.0, 300.0)),
                    col("peso_kg", "Peso (kg)", TipoDato::Decimal(0.0, 500.0)),
                    col("tipo_sangre", "Tipo de sangre", TipoDato::Texto(5)),
                    col("ultima_revision", "Última revisión", TipoDato::Fecha),
                    col("notas", "Notas", TipoDato::Texto(240)),
                ];
                C
            }
        }
    }

    /// Cantidad de columnas que entran en la tabla del listado.
    #[must_use]
    pub const fn columnas_listado(self) -> usize {
        match self {
            Self::Habitos => 8,
            Self::Entrenamientos | Self::Salud => 5,
            Self::Comidas => 4,
            Self::Mental => 8,
        }
    }
}

/// Fila de una sección: valores ya formateados para mostrar/editar, en el
/// orden de [`Seccion::columnas`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registro {
    /// Id.
    pub id: i64,
    /// Valores como texto (fechas `DD/MM/AAAA`, booleanos `Sí`/`No`).
    pub valores: Vec<String>,
}

/// Entradas del asistente (promedios de los registros recientes).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EntradasAsistente {
    /// Rendimiento promedio de los últimos 10 partidos (0-10).
    pub rendimiento: f64,
    /// Lesiones activas.
    pub lesiones: i64,
    /// Fracción de días con buen sueño (últimos 14 hábitos, 0-1).
    pub dormi: f64,
    /// Hidratación promedio (0-10).
    pub hidratacion: f64,
    /// Fracción de días con alcohol (0-1).
    pub alcohol: f64,
    /// Nervios promedio (0-10).
    pub nervios: f64,
    /// Confianza promedio en hábitos (0-10).
    pub confianza_habito: f64,
    /// Motivación promedio en hábitos (0-10).
    pub motivacion_habito: f64,
    /// Calidad de comida promedio de las últimas 14 (Buena=3, Regular=2, Mala=1; 0 sin datos).
    pub comida: f64,
    /// Entrenamientos no omitidos entre los últimos 14.
    pub entrenamientos: i64,
    /// Intensidad promedio de esos entrenamientos.
    pub intensidad: f64,
    /// Duración promedio (min).
    pub duracion: f64,
    /// Entrenamientos intensos (≥8, no omitidos) en los últimos 7 días.
    pub intensos_semana: i64,
    /// Promedios de las últimas 10 sesiones mentales (0-10).
    pub confianza: f64,
    /// Motivación.
    pub motivacion: f64,
    /// Concentración.
    pub concentracion: f64,
    /// Estrés.
    pub estres: f64,
    /// Presión.
    pub presion: f64,
}

/// Resultado del asistente de entrenamientos personalizados.
#[derive(Debug, Clone, PartialEq)]
pub struct Asistente {
    /// Puntaje de preparación 0-100.
    pub preparacion: i64,
    /// Riesgo de lesión 0-100.
    pub riesgo: i64,
    /// Nivel de carga sugerido: 0 baja, 1 moderada, 2 alta.
    pub nivel: u8,
    /// Puntajes parciales: rendimiento, hábitos, nutrición, mental, entrenamiento.
    pub parciales: [i64; 5],
    /// Resumen de una línea (formato MiFutbolC).
    pub resumen: String,
    /// Rutina semanal sugerida.
    pub rutina: String,
    /// Alerta de alta intensidad, si corresponde.
    pub alerta_intensidad: Option<String>,
}

/// Recomendación guardada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomendacion {
    /// Fecha ISO.
    pub fecha: String,
    /// Resumen.
    pub resumen: String,
}
