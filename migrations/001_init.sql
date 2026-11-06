-- ============================================================
-- 001_init.sql — Esquema núcleo MiFutbolR
-- ============================================================

PRAGMA foreign_keys = ON;

-- ---------- Usuario y configuración ----------
CREATE TABLE IF NOT EXISTS usuario (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    nombre        TEXT NOT NULL DEFAULT '',
    password_salt TEXT NOT NULL DEFAULT '',
    password_hash TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS settings (
    id                   INTEGER PRIMARY KEY CHECK (id = 1),
    theme                INTEGER NOT NULL DEFAULT 0,
    language             INTEGER NOT NULL DEFAULT 0,
    mode                 INTEGER NOT NULL DEFAULT 0,
    text_size            INTEGER NOT NULL DEFAULT 1,
    image_viewer         TEXT    NOT NULL DEFAULT '',
    botin_predeterminado INTEGER NOT NULL DEFAULT 0,
    updated_at           TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS custom_menus (
    menu_name TEXT PRIMARY KEY,
    enabled   INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0,1))
);

-- ---------- Catálogos ----------
CREATE TABLE IF NOT EXISTS camiseta (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre            TEXT    NOT NULL,
    color_principal   TEXT    NOT NULL DEFAULT '',
    color_secundario  TEXT    NOT NULL DEFAULT '',
    marca             TEXT    NOT NULL DEFAULT '',
    modelo            TEXT    NOT NULL DEFAULT '',
    temporada         TEXT    NOT NULL DEFAULT '',
    estado_fisico     TEXT    NOT NULL DEFAULT '',
    fecha_compra      TEXT    NOT NULL DEFAULT '',
    costo_centavos    INTEGER NOT NULL DEFAULT 0 CHECK (costo_centavos >= 0),
    observaciones     TEXT    NOT NULL DEFAULT '',
    proveedor         TEXT    NOT NULL DEFAULT '',
    fue_regalo        INTEGER NOT NULL DEFAULT 0 CHECK (fue_regalo IN (0,1)),
    regalo_de         TEXT    NOT NULL DEFAULT '',
    imagen_ruta       TEXT    NOT NULL DEFAULT '',
    activa            INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0,1)),
    sorteada          INTEGER NOT NULL DEFAULT 0 CHECK (sorteada IN (0,1)),
    created_at        TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at        TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS botin (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre      TEXT    NOT NULL,
    imagen_ruta TEXT    NOT NULL DEFAULT '',   -- nuevo (legacy no la tenía)
    activa      INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0,1)),
    sorteada    INTEGER NOT NULL DEFAULT 0 CHECK (sorteada IN (0,1))
);

CREATE TABLE IF NOT EXISTS cancha (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre                      TEXT    NOT NULL,
    telefono                    TEXT    NOT NULL DEFAULT '',
    direccion                   TEXT    NOT NULL DEFAULT '',
    localidad                   TEXT    NOT NULL DEFAULT '',
    -- Códigos: 5/7/8/11/99 o 1000+bitmask (ver §9.2)
    tipo_cancha_codigo          INTEGER NOT NULL DEFAULT 0,
    -- Códigos: 0=no se, 1=natural, 2=sintetico, 3=cemento, 4=otra, 2000+bitmask
    superficie_codigo           INTEGER NOT NULL DEFAULT 0,
    -- 0=descubierta, 1=techada, 2=no se
    techada_estado_codigo       INTEGER NOT NULL DEFAULT 2 CHECK (techada_estado_codigo IN (0,1,2)),
    tiene_iluminacion           INTEGER NOT NULL DEFAULT 0 CHECK (tiene_iluminacion IN (0,1)),
    horario_apertura_min        INTEGER NOT NULL DEFAULT -1,
    horario_cierre_min          INTEGER NOT NULL DEFAULT -1,
    precio_hora_dia_centavos    INTEGER NOT NULL DEFAULT 0 CHECK (precio_hora_dia_centavos >= 0),
    precio_hora_noche_centavos  INTEGER NOT NULL DEFAULT 0 CHECK (precio_hora_noche_centavos >= 0),
    tiene_vestuarios            INTEGER NOT NULL DEFAULT 0 CHECK (tiene_vestuarios IN (0,1)),
    tiene_duchas                INTEGER NOT NULL DEFAULT 0 CHECK (tiene_duchas IN (0,1)),
    tiene_buffet                INTEGER NOT NULL DEFAULT 0 CHECK (tiene_buffet IN (0,1)),
    tiene_estacionamiento       INTEGER NOT NULL DEFAULT 0 CHECK (tiene_estacionamiento IN (0,1)),
    cantidad_canchas            INTEGER NOT NULL DEFAULT 1 CHECK (cantidad_canchas > 0),
    estado                      TEXT    NOT NULL DEFAULT '',
    descripcion                 TEXT    NOT NULL DEFAULT '',
    contacto_alt                TEXT    NOT NULL DEFAULT '',
    latitud                     REAL,
    longitud                    REAL,
    tiene_grabacion             INTEGER NOT NULL DEFAULT 0 CHECK (tiene_grabacion IN (0,1)),
    tiene_marcador              INTEGER NOT NULL DEFAULT 2 CHECK (tiene_marcador IN (0,1,2)),
    imagen_ruta                 TEXT    NOT NULL DEFAULT '',
    activa                      INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0,1)),
    created_at                  TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at                  TEXT    NOT NULL DEFAULT ''
);

-- Tablas hijas para la selección múltiple de cancha (recomendado).
-- Las columnas cancha.tipo_cancha_codigo / superficie_codigo quedan como caché legacy.
CREATE TABLE IF NOT EXISTS cancha_tipo (
    cancha_id   INTEGER NOT NULL REFERENCES cancha(id) ON DELETE CASCADE,
    tipo_codigo INTEGER NOT NULL CHECK (tipo_codigo IN (5, 7, 8, 11, 99)),
    PRIMARY KEY (cancha_id, tipo_codigo)
);

CREATE TABLE IF NOT EXISTS cancha_superficie (
    cancha_id         INTEGER NOT NULL REFERENCES cancha(id) ON DELETE CASCADE,
    superficie_codigo INTEGER NOT NULL CHECK (superficie_codigo BETWEEN 0 AND 4),
    PRIMARY KEY (cancha_id, superficie_codigo)
);

-- ---------- Equipos ----------
-- Nota: SQLite permite que equipo.partido_id referencie partido antes de su
-- creación; la FK se resuelve en tiempo de ejecución.
CREATE TABLE IF NOT EXISTS equipo (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre      TEXT    NOT NULL,
    -- 0=fijo, 1=momentaneo
    tipo        INTEGER NOT NULL DEFAULT 0 CHECK (tipo IN (0,1)),
    -- 0=F5, 1=F7, 2=F8, 3=F11
    tipo_futbol INTEGER NOT NULL DEFAULT 0 CHECK (tipo_futbol BETWEEN 0 AND 3),
    num_jugadores INTEGER NOT NULL DEFAULT 5 CHECK (num_jugadores BETWEEN 1 AND 11),
    -- NULL = no asignado a un partido
    partido_id  INTEGER REFERENCES partido(id) ON DELETE SET NULL,
    activa      INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0,1)),
    imagen_ruta TEXT    NOT NULL DEFAULT '',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at  TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS jugador (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    equipo_id  INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    nombre     TEXT    NOT NULL,
    numero     INTEGER NOT NULL CHECK (numero BETWEEN 0 AND 99),
    -- 0=arquero, 1=defensor, 2=mediocampista, 3=delantero
    posicion   INTEGER NOT NULL CHECK (posicion BETWEEN 0 AND 3),
    es_capitan INTEGER NOT NULL DEFAULT 0 CHECK (es_capitan IN (0,1)),
    UNIQUE (equipo_id, numero)
);

-- ---------- Partidos ----------
CREATE TABLE IF NOT EXISTS partido (
    id                         INTEGER PRIMARY KEY AUTOINCREMENT,
    cancha_id                  INTEGER NOT NULL REFERENCES cancha(id) ON DELETE RESTRICT,
    camiseta_id                INTEGER NOT NULL REFERENCES camiseta(id) ON DELETE RESTRICT,
    botin_id                   INTEGER REFERENCES botin(id) ON DELETE SET NULL,
    fecha_hora                 TEXT    NOT NULL,              -- ISO: YYYY-MM-DD HH:MM:SS
    mes_anio                   TEXT    NOT NULL DEFAULT '',   -- YYYY-MM
    goles                      INTEGER NOT NULL DEFAULT 0 CHECK (goles >= 0),
    asistencias                INTEGER NOT NULL DEFAULT 0 CHECK (asistencias >= 0),
    resultado                  INTEGER NOT NULL DEFAULT 0 CHECK (resultado IN (0,1,2,3)),
    rendimiento_general        INTEGER NOT NULL DEFAULT 0 CHECK (rendimiento_general BETWEEN 0 AND 10),
    cansancio                  INTEGER NOT NULL DEFAULT 0 CHECK (cansancio BETWEEN 0 AND 10),
    estado_animo               INTEGER NOT NULL DEFAULT 0 CHECK (estado_animo BETWEEN 0 AND 10),
    comentario_personal        TEXT    NOT NULL DEFAULT '',
    clima                      INTEGER NOT NULL DEFAULT 0 CHECK (clima BETWEEN 0 AND 12),
    dia                        INTEGER NOT NULL DEFAULT 0 CHECK (dia BETWEEN 0 AND 6),
    precio_centavos            INTEGER NOT NULL DEFAULT 0 CHECK (precio_centavos >= 0),
    tipo_partido               INTEGER NOT NULL DEFAULT 1 CHECK (tipo_partido IN (1,2,3)),
    rival_nombre               TEXT    NOT NULL DEFAULT '',
    tipo_rival                 TEXT    NOT NULL DEFAULT '',
    posicion_jugada            TEXT    NOT NULL DEFAULT '',
    minutos_jugados            INTEGER NOT NULL DEFAULT 0 CHECK (minutos_jugados BETWEEN 0 AND 180),
    intensidad                 INTEGER NOT NULL DEFAULT 0 CHECK (intensidad BETWEEN 0 AND 10),
    condicion_cancha           TEXT    NOT NULL DEFAULT '',
    arbitraje                  TEXT    NOT NULL DEFAULT '',
    eventos_clave              TEXT    NOT NULL DEFAULT '',
    rating_tecnico             INTEGER NOT NULL DEFAULT 0 CHECK (rating_tecnico BETWEEN 0 AND 10),
    rating_fisico              INTEGER NOT NULL DEFAULT 0 CHECK (rating_fisico BETWEEN 0 AND 10),
    rating_mental              INTEGER NOT NULL DEFAULT 0 CHECK (rating_mental BETWEEN 0 AND 10),
    estado_cancha              INTEGER NOT NULL DEFAULT 0 CHECK (estado_cancha BETWEEN 0 AND 5),
    goles_equipo               INTEGER NOT NULL DEFAULT -1,
    goles_rival                INTEGER NOT NULL DEFAULT -1,
    formato_partido            TEXT    NOT NULL DEFAULT '',    -- F5/F7/F8/F11
    tarjeta                    INTEGER NOT NULL DEFAULT 1 CHECK (tarjeta IN (1,2,3)),
    goles_en_contra            INTEGER NOT NULL DEFAULT 0 CHECK (goles_en_contra >= 0),
    dolor_fisico               INTEGER NOT NULL DEFAULT 0 CHECK (dolor_fisico BETWEEN 0 AND 3),
    temperatura_c              REAL,
    arbitraje_score            INTEGER NOT NULL DEFAULT 0 CHECK (arbitraje_score BETWEEN 0 AND 5),
    lo_mejor                   TEXT    NOT NULL DEFAULT '',
    que_mejorar                TEXT    NOT NULL DEFAULT '',
    tags                       TEXT    NOT NULL DEFAULT '',
    goles_detalle              TEXT    NOT NULL DEFAULT '',
    asistencias_detalle        TEXT    NOT NULL DEFAULT '',
    atajaste_todo_el_partido   INTEGER NOT NULL DEFAULT 1 CHECK (atajaste_todo_el_partido IN (0,1,2)),
    diferencia_gol             INTEGER NOT NULL DEFAULT 0,
    precip_mm                  REAL,
    wind_kmh                   REAL,
    weather_code               INTEGER,
    clima_json                 TEXT    NOT NULL DEFAULT '',
    apparent_temp_c            REAL,
    created_at                 TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at                 TEXT    NOT NULL DEFAULT ''
);

-- Metadatos de partido: favoritos y etiquetas normalizadas.
-- El legacy las crea on-demand en partido.c; aquí forman parte del esquema.
CREATE TABLE IF NOT EXISTS partido_meta (
    partido_id INTEGER PRIMARY KEY REFERENCES partido(id) ON DELETE CASCADE,
    favorito   INTEGER NOT NULL DEFAULT 0 CHECK (favorito IN (0,1))
);

CREATE TABLE IF NOT EXISTS partido_tag (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    partido_id INTEGER NOT NULL REFERENCES partido(id) ON DELETE CASCADE,
    tag        TEXT    NOT NULL,
    UNIQUE (partido_id, tag)
);

-- ---------- Lesiones ----------
CREATE TABLE IF NOT EXISTS lesion (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    jugador     TEXT    NOT NULL,
    tipo        TEXT    NOT NULL,
    descripcion TEXT    NOT NULL,
    fecha       TEXT    NOT NULL,                         -- ISO
    camiseta_id INTEGER NOT NULL REFERENCES camiseta(id) ON DELETE RESTRICT,
    estado      TEXT    NOT NULL DEFAULT 'ACTIVA'
                CHECK (estado IN ('ACTIVA','EN TRATAMIENTO','MEJORANDO','RECUPERADO','RECAIDA')),
    partido_id  INTEGER REFERENCES partido(id) ON DELETE SET NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at  TEXT    NOT NULL DEFAULT ''
);

-- ---------- Finanzas ----------
CREATE TABLE IF NOT EXISTS financiamiento (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha          TEXT    NOT NULL,                      -- YYYY-MM-DD
    -- 0=ingreso, 1=gasto
    tipo           INTEGER NOT NULL CHECK (tipo IN (0,1)),
    -- 0=transporte,1=equipamiento,2=cuotas,3=torneos,4=arbitraje,5=canchas,6=medicina,7=otros
    categoria      INTEGER NOT NULL CHECK (categoria BETWEEN 0 AND 7),
    descripcion    TEXT    NOT NULL,
    monto_centavos INTEGER NOT NULL CHECK (monto_centavos >= 0),
    item_especifico TEXT   NOT NULL DEFAULT '',
    partido_id     INTEGER REFERENCES partido(id) ON DELETE SET NULL,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS presupuesto_mensual (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    mes_anio                  TEXT    NOT NULL UNIQUE,      -- YYYY-MM
    presupuesto_total_centavos INTEGER NOT NULL DEFAULT 0 CHECK (presupuesto_total_centavos >= 0),
    limite_gasto_centavos     INTEGER NOT NULL DEFAULT 0 CHECK (limite_gasto_centavos >= 0),
    alertas_habilitadas       INTEGER NOT NULL DEFAULT 1 CHECK (alertas_habilitadas IN (0,1)),
    fecha_creacion            TEXT    NOT NULL,
    fecha_modificacion        TEXT    NOT NULL
);

-- ---------- Carrera ----------
CREATE TABLE IF NOT EXISTS carrera_identidad (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    nombre          TEXT    NOT NULL DEFAULT '',
    edad            INTEGER NOT NULL DEFAULT 0 CHECK (edad BETWEEN 0 AND 120),
    pie_habil       TEXT    NOT NULL DEFAULT '',
    posiciones      TEXT    NOT NULL DEFAULT '',
    altura_cm       REAL,
    peso_kg         REAL,
    estilo          TEXT    NOT NULL DEFAULT '',
    dorsal_favorito INTEGER NOT NULL DEFAULT 0,
    objetivos       TEXT    NOT NULL DEFAULT '',
    historia        TEXT    NOT NULL DEFAULT '',
    updated_at      TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS carrera_partido_hito (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    partido_id INTEGER NOT NULL UNIQUE REFERENCES partido(id) ON DELETE CASCADE,
    tipo_hito  TEXT    NOT NULL,
    nota       TEXT    NOT NULL DEFAULT '',
    created_at TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS carrera_resumen_narrativo (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha          TEXT    NOT NULL,
    periodo_inicio TEXT    NOT NULL DEFAULT '',
    periodo_fin    TEXT    NOT NULL DEFAULT '',
    perfil_dinamico TEXT   NOT NULL DEFAULT '',
    resumen        TEXT    NOT NULL
);

-- Entrenamiento y progresión (opción 11 de Carrera)
CREATE TABLE IF NOT EXISTS entrenamiento_plan (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre              TEXT    NOT NULL,
    descripcion         TEXT    NOT NULL,
    duracion_semanas    INTEGER NOT NULL DEFAULT 4 CHECK (duracion_semanas > 0),
    sesiones_por_semana INTEGER NOT NULL DEFAULT 3 CHECK (sesiones_por_semana > 0)
);

CREATE TABLE IF NOT EXISTS progresion_jugador (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    jugador_id  INTEGER NOT NULL REFERENCES jugador(id) ON DELETE CASCADE,
    plan_id     INTEGER REFERENCES entrenamiento_plan(id) ON DELETE SET NULL,
    semana      INTEGER NOT NULL CHECK (semana > 0),
    ataque      INTEGER NOT NULL DEFAULT 0 CHECK (ataque BETWEEN 0 AND 100),
    defensa     INTEGER NOT NULL DEFAULT 0 CHECK (defensa BETWEEN 0 AND 100),
    resistencia INTEGER NOT NULL DEFAULT 0 CHECK (resistencia BETWEEN 0 AND 100),
    velocidad   INTEGER NOT NULL DEFAULT 0 CHECK (velocidad BETWEEN 0 AND 100),
    tecnica     INTEGER NOT NULL DEFAULT 0 CHECK (tecnica BETWEEN 0 AND 100),
    UNIQUE (jugador_id, plan_id, semana)
);

-- ---------- Tablas de apoyo (Torneos/Temporadas) ----------
-- No hay UI de administración en esta versión, pero Carrera y Resumen las leen.
CREATE TABLE IF NOT EXISTS torneo (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre            TEXT    NOT NULL,
    tiene_equipo_fijo INTEGER NOT NULL DEFAULT 0 CHECK (tiene_equipo_fijo IN (0,1)),
    equipo_fijo_id    INTEGER REFERENCES equipo(id) ON DELETE SET NULL,
    cantidad_equipos  INTEGER NOT NULL DEFAULT 0,
    tipo_torneo       INTEGER NOT NULL DEFAULT 0,
    formato_torneo    INTEGER NOT NULL DEFAULT 0,
    fase_actual       TEXT    NOT NULL DEFAULT 'Fase de Grupos'
);

CREATE TABLE IF NOT EXISTS partido_torneo (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    torneo_id    INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    equipo1_id   INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    equipo2_id   INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    fecha        TEXT,
    goles_equipo1 INTEGER NOT NULL DEFAULT 0,
    goles_equipo2 INTEGER NOT NULL DEFAULT 0,
    estado       TEXT,
    fase         TEXT NOT NULL DEFAULT 'Fase de Grupos'
);

CREATE TABLE IF NOT EXISTS equipo_historial (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    equipo_id             INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    torneo_id             INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    posicion_final        INTEGER,
    partidos_jugados      INTEGER NOT NULL DEFAULT 0,
    partidos_ganados      INTEGER NOT NULL DEFAULT 0,
    partidos_empatados    INTEGER NOT NULL DEFAULT 0,
    partidos_perdidos     INTEGER NOT NULL DEFAULT 0,
    goles_favor           INTEGER NOT NULL DEFAULT 0,
    goles_contra          INTEGER NOT NULL DEFAULT 0,
    mejor_goleador        TEXT,
    goles_mejor_goleador  INTEGER NOT NULL DEFAULT 0,
    fecha_inicio          TEXT,
    fecha_fin             TEXT
);

CREATE TABLE IF NOT EXISTS temporada (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre      TEXT    NOT NULL,
    anio        INTEGER NOT NULL,
    fecha_inicio TEXT   NOT NULL,
    fecha_fin    TEXT   NOT NULL,
    estado      TEXT    NOT NULL DEFAULT 'Planificada',
    descripcion TEXT
);
