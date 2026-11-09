-- 008_ola2.sql — Ola 2: tiendas, multimedia y amigos.

CREATE TABLE IF NOT EXISTS tienda (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre       TEXT    NOT NULL,
    tipo         TEXT    NOT NULL DEFAULT '',
    url          TEXT    NOT NULL DEFAULT '',
    telefono     TEXT    NOT NULL DEFAULT '',
    instagram    TEXT    NOT NULL DEFAULT '',
    productos    TEXT    NOT NULL DEFAULT '',
    rango_precio TEXT    NOT NULL DEFAULT '',
    tiene_envio  INTEGER NOT NULL DEFAULT 0 CHECK (tiene_envio IN (0,1)),
    favorito     INTEGER NOT NULL DEFAULT 0 CHECK (favorito IN (0,1)),
    notas        TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS media (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    partido_id  INTEGER REFERENCES partido(id) ON DELETE SET NULL,
    tipo        TEXT    NOT NULL DEFAULT 'video',
    titulo      TEXT    NOT NULL,
    url         TEXT    NOT NULL DEFAULT '',
    descripcion TEXT    NOT NULL DEFAULT '',
    fecha       TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS amigo (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre            TEXT    NOT NULL,
    posicion          TEXT    NOT NULL DEFAULT '',
    goles             INTEGER NOT NULL DEFAULT 0,
    asistencias       INTEGER NOT NULL DEFAULT 0,
    partidos          INTEGER NOT NULL DEFAULT 0,
    mejor_rendimiento INTEGER NOT NULL DEFAULT 0,
    notas             TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_tienda_nombre ON tienda(nombre);
CREATE INDEX IF NOT EXISTS idx_media_tipo ON media(tipo);
CREATE INDEX IF NOT EXISTS idx_amigo_goles ON amigo(goles DESC);
