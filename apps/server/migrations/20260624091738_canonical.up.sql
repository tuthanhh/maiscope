-- Canonical catalog schema. Single game (maimai) — see app/game.ts (multi-game
-- routing/registry removed). Mirrors the frontend `Data` shape exactly
-- (apps/host/src/types/{Data,Song,Sheet}.ts) so the client rehydrates its frozen,
-- prototype-linked object graph (utils/data.ts:preprocessData) unchanged.
-- Charts / assets / contributions live in later migrations.
-- Cross-tier key: sheet_expr = songId|type|difficulty.

-- ── catalog_meta ─────────────────────────────────────────────────────────────
-- Singleton (one row). Holds Data.updateTime — catalog freshness stamp / ETag
-- source (contract §1). CHECK pins it to a single row.
CREATE TABLE catalog_meta (
    id          BOOLEAN     PRIMARY KEY DEFAULT true,
    update_time TIMESTAMPTZ NOT NULL,
    CONSTRAINT catalog_meta_single_row CHECK (id)
);

-- ── ordered lookup tables ────────────────────────────────────────────────────
-- Data.categories/versions/types/difficulties/regions. Array order is
-- significant (stores/data.ts builds index maps), so each carries `ordinal`.

-- Data.categories: { category }[]
CREATE TABLE categories (
    category TEXT    PRIMARY KEY,
    ordinal  INTEGER NOT NULL
);

-- Data.versions: { version, abbr?, releaseDate? }[]
CREATE TABLE versions (
    version      TEXT    PRIMARY KEY,
    abbr         TEXT,
    release_date DATE,
    ordinal      INTEGER NOT NULL
);

-- Data.types: { type, name, abbr?, iconUrl?, iconHeight? }[]
CREATE TABLE types (
    type        TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    abbr        TEXT,
    icon_url    TEXT,                      -- raw; client resolves to absolute URL
    icon_height INTEGER,
    ordinal     INTEGER NOT NULL
);

-- Data.difficulties: { difficulty, name, color?, iconUrl?, iconHeight? }[]
CREATE TABLE difficulties (
    difficulty  TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    color       TEXT,
    icon_url    TEXT,
    icon_height INTEGER,
    ordinal     INTEGER NOT NULL
);

-- Data.regions: { region, name }[]
CREATE TABLE regions (
    region  TEXT    PRIMARY KEY,
    name    TEXT    NOT NULL,
    ordinal INTEGER NOT NULL
);

-- ── songs ────────────────────────────────────────────────────────────────────
-- Mirrors types/Song.ts raw fields. song_no is a derived field stored as a
-- convenience (client recomputes in preprocessData).
CREATE TABLE songs (
    id           BIGSERIAL   PRIMARY KEY,
    song_id      TEXT        UNIQUE,        -- Song.songId (string | null)
    song_no      INTEGER,                   -- derived display order
    category     TEXT,
    title        TEXT,
    artist       TEXT,
    bpm          DOUBLE PRECISION,
    image_name   TEXT,
    version      TEXT,
    release_date DATE,
    is_new       BOOLEAN,
    is_locked    BOOLEAN,
    comment      TEXT,
    source_index INTEGER     NOT NULL,      -- upstream data.json order (reproducible ingest)
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX songs_source_idx ON songs (source_index);

-- ── sheets ───────────────────────────────────────────────────────────────────
-- Mirrors the sheet-only additions in types/Sheet.ts. Catalog metadata about a
-- chart slot — NOT the chart notes (those go in the future `charts` table).
CREATE TABLE sheets (
    id                   BIGSERIAL   PRIMARY KEY,
    song_id_fk           BIGINT      NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    sheet_expr           TEXT        NOT NULL UNIQUE,   -- songId|type|difficulty
    type                 TEXT,
    difficulty           TEXT,
    level                TEXT,
    level_value          DOUBLE PRECISION,
    internal_level       TEXT,
    internal_level_value DOUBLE PRECISION,
    note_designer        TEXT,
    is_special           BOOLEAN,
    source_index         INTEGER     NOT NULL,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sheets_song_idx ON sheets (song_id_fk);

-- ── sheet sub-tables ─────────────────────────────────────────────────────────
-- Normalized out of the Sheet object's Record<...> maps.

-- Sheet.noteCounts: Record<string, number | null>. value nullable per the type.
CREATE TABLE sheet_note_counts (
    sheet_id BIGINT NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
    key      TEXT   NOT NULL,                  -- 'total' | 'tap' | 'hold' | ...
    value    INTEGER,
    PRIMARY KEY (sheet_id, key)
);

-- Sheet.regions: Record<string, boolean>.
CREATE TABLE sheet_regions (
    sheet_id  BIGINT  NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
    region    TEXT    NOT NULL,
    available BOOLEAN NOT NULL,
    PRIMARY KEY (sheet_id, region)
);

-- Sheet.regionOverrides: Record<string, Sheet>. Row existence = override applies;
-- non-null columns override the canonical sheet value, null columns inherit.
CREATE TABLE sheet_region_overrides (
    sheet_id             BIGINT NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
    region               TEXT   NOT NULL,
    level                TEXT,
    level_value          DOUBLE PRECISION,
    internal_level       TEXT,
    internal_level_value DOUBLE PRECISION,
    note_designer        TEXT,
    PRIMARY KEY (sheet_id, region)
);
