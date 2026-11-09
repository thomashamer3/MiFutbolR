-- 009_ola2_recordatorios.sql — Recordatorios y agenda (IMP-18).

CREATE TABLE IF NOT EXISTS recordatorio (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    titulo       TEXT    NOT NULL,
    nota         TEXT    NOT NULL DEFAULT '',
    fecha        TEXT    NOT NULL,
    hora         TEXT    NOT NULL DEFAULT '',
    periodicidad INTEGER NOT NULL DEFAULT 0 CHECK (periodicidad BETWEEN 0 AND 3),
    tematica     TEXT    NOT NULL DEFAULT '',
    completado   INTEGER NOT NULL DEFAULT 0 CHECK (completado IN (0,1)),
    ultima_gen   TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE INDEX IF NOT EXISTS idx_recordatorio_fecha
    ON recordatorio(completado, fecha, hora);
