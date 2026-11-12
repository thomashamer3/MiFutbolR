-- 011_usuario.sql — cuentas locales y bienvenida.

CREATE TABLE IF NOT EXISTS cuenta (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre        TEXT    NOT NULL COLLATE NOCASE UNIQUE,
    password_hash TEXT    NOT NULL DEFAULT '',
    con_password  INTEGER NOT NULL DEFAULT 0 CHECK (con_password IN (0,1)),
    creado_en     TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    ultimo_acceso TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS cuenta_sesion (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    usuario_id      INTEGER REFERENCES cuenta(id) ON DELETE SET NULL,
    bienvenida_vista INTEGER NOT NULL DEFAULT 0 CHECK (bienvenida_vista IN (0,1))
);

INSERT OR IGNORE INTO cuenta_sesion (id, usuario_id, bienvenida_vista) VALUES (1, NULL, 0);
