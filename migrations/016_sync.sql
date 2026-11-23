-- 016_sync.sql — Sincronización con PocketBase (port de sync_queue.c / sync_entidades.c).
-- Local-first: SQLite es la fuente de trabajo; PocketBase recibe/envía partidos,
-- camisetas y botines. La cola `sync_dirty` la alimentan triggers sobre `partido`.
-- Diferencias con C: los uuid y `updated_at` faltantes se completan al sincronizar
-- (con el deshacer suspendido), así los triggers nunca reescriben la fila y el
-- historial de Ctrl+Z no se ensucia.

ALTER TABLE partido ADD COLUMN uuid TEXT;
ALTER TABLE camiseta ADD COLUMN uuid TEXT;
ALTER TABLE botin ADD COLUMN uuid TEXT;
-- Columnas de botín que C sincroniza (botines_sync); la TUI no las edita, pero
-- se conservan para no pisar en el servidor lo cargado desde otro equipo.
ALTER TABLE botin ADD COLUMN marca TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN modelo TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN talla TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN superficie TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN estado_fisico TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN fecha_compra TEXT NOT NULL DEFAULT '';
ALTER TABLE botin ADD COLUMN costo_centavos INTEGER NOT NULL DEFAULT 0;
ALTER TABLE botin ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_partido_uuid ON partido(uuid);
CREATE UNIQUE INDEX IF NOT EXISTS idx_camiseta_uuid ON camiseta(uuid);
CREATE UNIQUE INDEX IF NOT EXISTS idx_botin_uuid ON botin(uuid);

-- Configuración (URL local/remota, correo, token, user_id, cursor, servidor
-- activo, última sync, auto_sync). La contraseña nunca se guarda.
CREATE TABLE IF NOT EXISTS sync_config (
    clave          TEXT PRIMARY KEY,
    valor          TEXT,
    actualizado_en TEXT
);

CREATE TABLE IF NOT EXISTS sync_dirty (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    partido_id   INTEGER,
    uuid         TEXT,
    operacion    TEXT NOT NULL DEFAULT 'upsert',
    estado       TEXT NOT NULL DEFAULT 'pendiente',
    ultimo_error TEXT,
    creado_en    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sync_dirty_estado ON sync_dirty(estado, id);
CREATE INDEX IF NOT EXISTS idx_sync_dirty_uuid ON sync_dirty(uuid);

CREATE TABLE IF NOT EXISTS sync_conflicto (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid        TEXT NOT NULL UNIQUE,
    local_json  TEXT,
    remoto_json TEXT,
    descripcion TEXT,
    creado_en   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_tombstones (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    entidad   TEXT NOT NULL,
    uuid      TEXT NOT NULL,
    creado_en TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_tombstone_uuid ON sync_tombstones(entidad, uuid);

-- Partidos: solo se encolan los que ya tienen uuid (los nuevos lo reciben al
-- sincronizar y ese UPDATE los encola).
CREATE TRIGGER IF NOT EXISTS trg_partido_sync_insert AFTER INSERT ON partido
WHEN new.uuid IS NOT NULL AND TRIM(new.uuid) <> ''
BEGIN
  INSERT INTO sync_dirty (partido_id, uuid, operacion, estado, creado_en)
  VALUES (new.id, new.uuid, 'upsert', 'pendiente', datetime('now','localtime'));
END;

CREATE TRIGGER IF NOT EXISTS trg_partido_sync_update AFTER UPDATE ON partido
WHEN new.id = old.id AND new.uuid IS NOT NULL AND TRIM(new.uuid) <> ''
BEGIN
  INSERT INTO sync_dirty (partido_id, uuid, operacion, estado, creado_en)
  VALUES (new.id, new.uuid, 'upsert', 'pendiente', datetime('now','localtime'));
END;

CREATE TRIGGER IF NOT EXISTS trg_partido_sync_delete AFTER DELETE ON partido
WHEN old.uuid IS NOT NULL AND TRIM(old.uuid) <> ''
BEGIN
  INSERT INTO sync_dirty (partido_id, uuid, operacion, estado, creado_en)
  VALUES (old.id, old.uuid, 'eliminar', 'pendiente', datetime('now','localtime'));
END;

-- Camisetas y botines: los borrados dejan una lápida para el servidor.
CREATE TRIGGER IF NOT EXISTS trg_camiseta_sync_delete AFTER DELETE ON camiseta
WHEN old.uuid IS NOT NULL AND TRIM(old.uuid) <> ''
BEGIN
  INSERT INTO sync_tombstones (entidad, uuid, creado_en)
  VALUES ('camiseta', old.uuid, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
  ON CONFLICT(entidad, uuid) DO UPDATE SET creado_en = excluded.creado_en;
END;

CREATE TRIGGER IF NOT EXISTS trg_botin_sync_delete AFTER DELETE ON botin
WHEN old.uuid IS NOT NULL AND TRIM(old.uuid) <> ''
BEGIN
  INSERT INTO sync_tombstones (entidad, uuid, creado_en)
  VALUES ('botin', old.uuid, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
  ON CONFLICT(entidad, uuid) DO UPDATE SET creado_en = excluded.creado_en;
END;
