-- 007_ola1_temporadas.sql — Ola 1: temporadas y fases (IMP-12).

CREATE TABLE IF NOT EXISTS temporada_fase (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id  INTEGER NOT NULL REFERENCES temporada(id) ON DELETE CASCADE,
    nombre        TEXT    NOT NULL,
    tipo          TEXT    NOT NULL DEFAULT 'regular',
    fecha_inicio  TEXT    NOT NULL DEFAULT '',
    fecha_fin     TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS torneo_temporada (
    temporada_id INTEGER NOT NULL REFERENCES temporada(id) ON DELETE CASCADE,
    torneo_id    INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    PRIMARY KEY (temporada_id, torneo_id)
);

CREATE INDEX IF NOT EXISTS idx_temporada_fase_temporada
    ON temporada_fase(temporada_id, id);
CREATE INDEX IF NOT EXISTS idx_torneo_temporada_torneo
    ON torneo_temporada(torneo_id);
