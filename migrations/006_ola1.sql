-- 006_ola1.sql — Ola 1: torneos.
-- Las tablas de torneo y partido_torneo ya existen desde 001.

CREATE TABLE IF NOT EXISTS equipo_torneo (
    torneo_id INTEGER NOT NULL REFERENCES torneo(id) ON DELETE CASCADE,
    equipo_id INTEGER NOT NULL REFERENCES equipo(id) ON DELETE CASCADE,
    PRIMARY KEY (torneo_id, equipo_id)
);

CREATE INDEX IF NOT EXISTS idx_equipo_torneo_equipo
    ON equipo_torneo(equipo_id);
CREATE INDEX IF NOT EXISTS idx_partido_torneo_torneo
    ON partido_torneo(torneo_id, estado);
