-- B9 Historial de equipamiento (`equipamiento_historial.c`): lo que ya
-- compraste y usaste. Como C, salvo el precio en centavos (convención de MiFutbolR)
-- y la fecha de compra en ISO AAAA-MM-DD.
CREATE TABLE IF NOT EXISTS equipamiento_historial (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo             INTEGER NOT NULL DEFAULT 1 CHECK (tipo BETWEEN 1 AND 6),
    marca            TEXT    NOT NULL DEFAULT '',
    modelo           TEXT    NOT NULL DEFAULT '',
    fecha_compra     TEXT    NOT NULL DEFAULT '',
    precio_centavos  INTEGER NOT NULL DEFAULT 0 CHECK (precio_centavos >= 0),
    partidos_usados  INTEGER NOT NULL DEFAULT 0 CHECK (partidos_usados >= 0),
    estado_fisico    INTEGER NOT NULL DEFAULT 1 CHECK (estado_fisico BETWEEN 1 AND 5),
    rating           INTEGER NOT NULL DEFAULT 5 CHECK (rating BETWEEN 1 AND 10),
    notas            TEXT    NOT NULL DEFAULT '',
    activo           INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_equipamiento_tipo ON equipamiento_historial(tipo);
