-- 004_ola0.sql — Ola 0: logros, notificaciones y onboarding.
-- Las migraciones son inmutables: los cambios futuros van en 005+.

CREATE TABLE IF NOT EXISTS logro_desbloqueado (
    logro_id TEXT PRIMARY KEY,
    fecha    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS notificacion (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo       TEXT    NOT NULL,
    severidad  INTEGER NOT NULL DEFAULT 0 CHECK (severidad BETWEEN 0 AND 2),
    titulo     TEXT    NOT NULL,
    mensaje    TEXT    NOT NULL,
    leida      INTEGER NOT NULL DEFAULT 0 CHECK (leida IN (0,1)),
    entidad    TEXT    NOT NULL DEFAULT '',
    entidad_id INTEGER,
    fecha      TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE INDEX IF NOT EXISTS idx_notificacion_leida
    ON notificacion(leida, fecha);
CREATE INDEX IF NOT EXISTS idx_notificacion_entidad
    ON notificacion(entidad, entidad_id);
CREATE INDEX IF NOT EXISTS idx_logro_fecha
    ON logro_desbloqueado(fecha);

ALTER TABLE settings ADD COLUMN tutorial_completado INTEGER NOT NULL DEFAULT 0;
