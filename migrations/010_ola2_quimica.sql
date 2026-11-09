-- 010_ola2_quimica.sql — Química entre jugadores (IMP-24).

CREATE TABLE IF NOT EXISTS quimica_jugador_estadistica (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    jugador1_id   INTEGER NOT NULL REFERENCES jugador(id) ON DELETE CASCADE,
    jugador2_id   INTEGER NOT NULL REFERENCES jugador(id) ON DELETE CASCADE,
    partidos      INTEGER NOT NULL DEFAULT 0 CHECK (partidos >= 0),
    victorias     INTEGER NOT NULL DEFAULT 0 CHECK (victorias >= 0),
    goles         INTEGER NOT NULL DEFAULT 0 CHECK (goles >= 0),
    asistencias   INTEGER NOT NULL DEFAULT 0 CHECK (asistencias >= 0),
    comentario    TEXT    NOT NULL DEFAULT '',
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now','localtime')),
    CHECK (jugador1_id <> jugador2_id)
);

CREATE INDEX IF NOT EXISTS idx_quimica_jugador1
    ON quimica_jugador_estadistica(jugador1_id);
CREATE INDEX IF NOT EXISTS idx_quimica_jugador2
    ON quimica_jugador_estadistica(jugador2_id);
