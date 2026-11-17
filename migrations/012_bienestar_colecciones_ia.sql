-- 012_bienestar_colecciones_ia.sql — paridad con MiFutbolC: Bienestar,
-- Colecciones e inventario, Entrenador IA. Mismos nombres de tabla y columnas
-- que MiFutbolC para que sus datos se puedan importar sin traducción.

-- Bienestar -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS bienestar_habito (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha          TEXT    NOT NULL,
    dormi_bien     INTEGER DEFAULT 0,
    hidratacion    INTEGER DEFAULT 0,
    alcohol        INTEGER DEFAULT 0,
    estado_animico TEXT    DEFAULT '',
    nervios        INTEGER DEFAULT 0,
    confianza      INTEGER DEFAULT 0,
    motivacion     INTEGER DEFAULT 0,
    notas          TEXT    DEFAULT '',
    tipo_diario    TEXT    DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_entrenamiento (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha        TEXT    NOT NULL,
    tipo         TEXT    NOT NULL,
    duracion_min INTEGER NOT NULL,
    intensidad   INTEGER NOT NULL,
    omitido      INTEGER DEFAULT 0,
    notas        TEXT    DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_comida (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha       TEXT NOT NULL,
    tipo        TEXT NOT NULL,
    calidad     TEXT NOT NULL,
    descripcion TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_sesion_mental (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha              TEXT    NOT NULL,
    tipo               TEXT    NOT NULL,
    momento            TEXT    NOT NULL DEFAULT 'N/A',
    partido_id         INTEGER DEFAULT NULL REFERENCES partido(id) ON DELETE SET NULL,
    confianza          INTEGER DEFAULT 0,
    estres             INTEGER DEFAULT 0,
    motivacion         INTEGER DEFAULT 0,
    miedos             TEXT    DEFAULT '',
    presion            INTEGER DEFAULT 0,
    concentracion      INTEGER DEFAULT 0,
    pensamientos_clave TEXT    DEFAULT '',
    texto_libre        TEXT    DEFAULT ''
);

CREATE TABLE IF NOT EXISTS bienestar_salud (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    altura_cm       REAL DEFAULT NULL,
    peso_kg         REAL DEFAULT NULL,
    tipo_sangre     TEXT DEFAULT '',
    ultima_revision TEXT DEFAULT '',
    medidas         TEXT DEFAULT '',
    notas           TEXT DEFAULT ''
);
INSERT OR IGNORE INTO bienestar_salud (id) VALUES (1);

CREATE TABLE IF NOT EXISTS bienestar_recomendacion (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha             TEXT    NOT NULL,
    score_preparacion INTEGER DEFAULT 0,
    riesgo_lesion     INTEGER DEFAULT 0,
    resumen           TEXT    DEFAULT '',
    rutina            TEXT    DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_bienestar_habito_fecha ON bienestar_habito(fecha);
CREATE INDEX IF NOT EXISTS idx_bienestar_entrenamiento_fecha ON bienestar_entrenamiento(fecha);
CREATE INDEX IF NOT EXISTS idx_bienestar_comida_fecha ON bienestar_comida(fecha);
CREATE INDEX IF NOT EXISTS idx_bienestar_mental_fecha ON bienestar_sesion_mental(fecha);

-- Colecciones e inventario --------------------------------------------------
CREATE TABLE IF NOT EXISTS coleccion (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre         TEXT NOT NULL UNIQUE,
    descripcion    TEXT DEFAULT '',
    fecha_creacion TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS inventario_item (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo         INTEGER NOT NULL,
    nombre       TEXT    DEFAULT '',
    estado       INTEGER NOT NULL DEFAULT 0,
    valor        REAL    DEFAULT 0,
    fecha_compra TEXT    DEFAULT '',
    camiseta_id  INTEGER UNIQUE,
    CHECK (tipo IN (1, 2, 3)),
    CHECK (estado IN (0, 1)),
    FOREIGN KEY (camiseta_id) REFERENCES camiseta(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS coleccion_inventario (
    coleccion_id     INTEGER NOT NULL,
    inventario_id    INTEGER NOT NULL,
    fecha_asociacion TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (coleccion_id, inventario_id),
    FOREIGN KEY (coleccion_id) REFERENCES coleccion(id) ON DELETE CASCADE,
    FOREIGN KEY (inventario_id) REFERENCES inventario_item(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_inventario_tipo ON inventario_item(tipo);
CREATE INDEX IF NOT EXISTS idx_coleccion_inventario_item ON coleccion_inventario(inventario_id);

INSERT OR IGNORE INTO coleccion (nombre, descripcion) VALUES
    ('Equipamiento actual', 'Items que usas actualmente.'),
    ('Historico', 'Coleccion de items de temporadas anteriores.'),
    ('Vendidos', 'Items vendidos o fuera del inventario activo.');

-- Entrenador IA ----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS consejos_historial (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    fecha   INTEGER NOT NULL,                 -- unix time, como MiFutbolC
    consejo TEXT    NOT NULL,
    seguido INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ia_config (
    id                 INTEGER PRIMARY KEY,
    nivel_intervencion INTEGER NOT NULL DEFAULT 2,
    ultima_alerta      INTEGER NOT NULL DEFAULT 0,
    ultimo_riesgo      REAL    NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO ia_config (id, nivel_intervencion, ultima_alerta, ultimo_riesgo)
VALUES (1, 2, 0, 0);
