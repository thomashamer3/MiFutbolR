-- 015_fk_indexes.sql — Índices para claves foráneas sin índice propio.
-- Sin ellos, borrar un partido/equipo/torneo recorre la tabla hija entera
-- (ON DELETE SET NULL/CASCADE) y los JOIN por estas columnas no usan índice.
-- Los filtros por mes usan los índices de fecha ya existentes
-- (idx_partido_fecha_hora, idx_financiamiento_fecha, idx_recordatorio_fecha).

CREATE INDEX IF NOT EXISTS idx_equipo_partido ON equipo(partido_id);
CREATE INDEX IF NOT EXISTS idx_media_partido ON media(partido_id);
CREATE INDEX IF NOT EXISTS idx_bienestar_mental_partido ON bienestar_sesion_mental(partido_id);
CREATE INDEX IF NOT EXISTS idx_partido_torneo_equipo1 ON partido_torneo(equipo1_id);
CREATE INDEX IF NOT EXISTS idx_partido_torneo_equipo2 ON partido_torneo(equipo2_id);
CREATE INDEX IF NOT EXISTS idx_equipo_historial_equipo ON equipo_historial(equipo_id);
CREATE INDEX IF NOT EXISTS idx_equipo_historial_torneo ON equipo_historial(torneo_id);
CREATE INDEX IF NOT EXISTS idx_progresion_jugador_plan ON progresion_jugador(plan_id);
CREATE INDEX IF NOT EXISTS idx_torneo_equipo_fijo ON torneo(equipo_fijo_id);
