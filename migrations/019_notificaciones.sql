-- B9 Notificaciones: cumpleaños de jugadores (`cumpleanos_jugadores` de C,
-- que consultaba una columna inexistente). Fecha ISO AAAA-MM-DD o vacía.
ALTER TABLE jugador ADD COLUMN fecha_nacimiento TEXT NOT NULL DEFAULT '';

-- El undo del jugador también guarda la fecha de nacimiento.
DROP TRIGGER IF EXISTS undo_jugador_ai;
CREATE TRIGGER IF NOT EXISTS undo_jugador_ai AFTER INSERT ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', NEW.id, 'crear', 'jugador #' || NEW.id, NULL, json_object('id', NEW.id, 'equipo_id', NEW.equipo_id, 'nombre', NEW.nombre, 'numero', NEW.numero, 'posicion', NEW.posicion, 'es_capitan', NEW.es_capitan, 'fecha_nacimiento', NEW.fecha_nacimiento)); -- NOSONAR: literal repetido por diseño en trigger generado
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

DROP TRIGGER IF EXISTS undo_jugador_au;
CREATE TRIGGER IF NOT EXISTS undo_jugador_au AFTER UPDATE ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', NEW.id, 'editar', 'jugador #' || NEW.id, json_object('id', OLD.id, 'equipo_id', OLD.equipo_id, 'nombre', OLD.nombre, 'numero', OLD.numero, 'posicion', OLD.posicion, 'es_capitan', OLD.es_capitan, 'fecha_nacimiento', OLD.fecha_nacimiento), json_object('id', NEW.id, 'equipo_id', NEW.equipo_id, 'nombre', NEW.nombre, 'numero', NEW.numero, 'posicion', NEW.posicion, 'es_capitan', NEW.es_capitan, 'fecha_nacimiento', NEW.fecha_nacimiento));
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;

DROP TRIGGER IF EXISTS undo_jugador_ad;
CREATE TRIGGER IF NOT EXISTS undo_jugador_ad AFTER DELETE ON jugador WHEN (SELECT IFNULL(suspendido, 0) FROM undo_control WHERE id = 1) = 0
BEGIN
  INSERT INTO undo_historial (tabla, registro_id, operacion, descripcion, antes, despues)
  VALUES ('jugador', OLD.id, 'borrar', 'jugador #' || OLD.id, json_object('id', OLD.id, 'equipo_id', OLD.equipo_id, 'nombre', OLD.nombre, 'numero', OLD.numero, 'posicion', OLD.posicion, 'es_capitan', OLD.es_capitan, 'fecha_nacimiento', OLD.fecha_nacimiento), NULL);
  DELETE FROM undo_historial WHERE id NOT IN (SELECT id FROM undo_historial ORDER BY id DESC LIMIT 50);
END;
