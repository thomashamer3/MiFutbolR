-- 002_indexes.sql
CREATE INDEX IF NOT EXISTS idx_camiseta_nombre      ON camiseta(nombre);
CREATE INDEX IF NOT EXISTS idx_camiseta_activa      ON camiseta(activa);
CREATE INDEX IF NOT EXISTS idx_cancha_nombre        ON cancha(nombre);
CREATE INDEX IF NOT EXISTS idx_cancha_activa        ON cancha(activa);
CREATE INDEX IF NOT EXISTS idx_equipo_nombre        ON equipo(nombre);
CREATE INDEX IF NOT EXISTS idx_equipo_activa        ON equipo(activa);
CREATE INDEX IF NOT EXISTS idx_jugador_equipo       ON jugador(equipo_id);

CREATE INDEX IF NOT EXISTS idx_partido_fecha_hora   ON partido(fecha_hora);
CREATE INDEX IF NOT EXISTS idx_partido_mes_anio     ON partido(mes_anio);
CREATE INDEX IF NOT EXISTS idx_partido_cancha_fecha ON partido(cancha_id, fecha_hora);
CREATE INDEX IF NOT EXISTS idx_partido_camiseta_fecha ON partido(camiseta_id, fecha_hora);
CREATE INDEX IF NOT EXISTS idx_partido_resultado    ON partido(resultado);
CREATE INDEX IF NOT EXISTS idx_partido_clima        ON partido(clima);
CREATE INDEX IF NOT EXISTS idx_partido_tipo         ON partido(tipo_partido);
CREATE INDEX IF NOT EXISTS idx_partido_botin        ON partido(botin_id);
CREATE INDEX IF NOT EXISTS idx_partido_meta_favorito ON partido_meta(favorito);
CREATE INDEX IF NOT EXISTS idx_partido_tag_tag      ON partido_tag(tag);

CREATE INDEX IF NOT EXISTS idx_lesion_camiseta_fecha ON lesion(camiseta_id, fecha);
CREATE INDEX IF NOT EXISTS idx_lesion_partido       ON lesion(partido_id);
CREATE INDEX IF NOT EXISTS idx_lesion_estado        ON lesion(estado);

CREATE INDEX IF NOT EXISTS idx_financiamiento_fecha ON financiamiento(fecha);
CREATE INDEX IF NOT EXISTS idx_financiamiento_tipo  ON financiamiento(tipo);
CREATE INDEX IF NOT EXISTS idx_financiamiento_categoria ON financiamiento(categoria);
CREATE INDEX IF NOT EXISTS idx_financiamiento_partido ON financiamiento(partido_id);

CREATE INDEX IF NOT EXISTS idx_carrera_hito_partido ON carrera_partido_hito(partido_id);
CREATE INDEX IF NOT EXISTS idx_carrera_hito_tipo    ON carrera_partido_hito(tipo_hito);
