// Response types mirroring apps/host/src/types/{Song,Sheet}.ts.
//
// Server emits RAW fields only. Derived fields the client computes in
// utils/data.ts:preprocessData (songNo, imageUrl, imageUrlM, sheetExpr,
// notePercents, $canonicalSheet) are intentionally absent here.
//
// This is the single source of truth for the response shape: main.rs's
// handlers build these types via src/queries.rs's sqlx::query_as! queries,
// then serde serializes them directly. There is no second, hand-maintained
// description of the shape (the old v_song/v_sheet/... Postgres views that
// used to play that role were dropped in migration 20260907080000).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Song-level fields shared by Song and (via prototype) Sheet.
/// Mirrors types/Song.ts minus `sheets` and the derived fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongMeta {
    // `string | null` in TS — always present, may be null.
    pub song_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_new: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_locked: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Sheet-only fields. Mirrors the additions in types/Sheet.ts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_level_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_designer: Option<String>,
    // Record<string, number | null>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_counts: Option<BTreeMap<String, Option<i64>>>,

    // Record<string, boolean>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<BTreeMap<String, bool>>,
    // Partial per-region overrides — only the 5 fields a region can override,
    // never a full Sheet (region_overrides never carried type/difficulty/
    // isSpecial/hasChart on the wire; the old view only ever built these 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_overrides: Option<BTreeMap<String, RegionOverride>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_special: Option<bool>,

    // Always present — true if a `charts` row with inline content exists for
    // this sheet_expr. Server truth, not derived, so no skip_serializing_if.
    pub has_chart: bool,
}

/// A region's partial override of a Sheet's level/designer fields.
/// Row existence in `sheet_region_overrides` = override applies; NULL columns
/// inherit the canonical sheet value (contract §1.1 regionOverrides).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_level_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_designer: Option<String>,
}

/// Nested sheet as stored under a Song (sheet-only fields; song fields come
/// from the parent via prototype on the client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedSheet {
    #[serde(flatten)]
    pub sheet: SheetMeta,
}

/// `GET /songs/{id}` response. Mirrors types/Song.ts (raw).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    #[serde(flatten)]
    pub meta: SongMeta,
    pub sheets: Vec<NestedSheet>,
}

/// `GET /sheets/{sheetExpr}` response. Standalone sheet = song fields + sheet
/// fields flattened, matching types/Sheet.ts (`Omit<Song,'sheets'> & {...}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    #[serde(flatten)]
    pub song: SongMeta,
    #[serde(flatten)]
    pub sheet: SheetMeta,
}

/// `Data.categories[]` entry (contract §1, `GET /catalog`).
#[derive(Debug, Serialize)]
pub struct CategoryEntry {
    pub category: String,
}

/// `Data.versions[]` entry.
#[derive(Debug, Serialize)]
pub struct VersionEntry {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
    #[serde(rename = "releaseDate", skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

/// `Data.types[]` entry.
#[derive(Debug, Serialize)]
pub struct TypeEntry {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(rename = "iconHeight", skip_serializing_if = "Option::is_none")]
    pub icon_height: Option<i32>,
}

/// `Data.difficulties[]` entry.
#[derive(Debug, Serialize)]
pub struct DifficultyEntry {
    pub difficulty: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(rename = "iconHeight", skip_serializing_if = "Option::is_none")]
    pub icon_height: Option<i32>,
}

/// `Data.regions[]` entry.
#[derive(Debug, Serialize)]
pub struct RegionEntry {
    pub region: String,
    pub name: String,
}

/// `GET /catalog` response (contract §1). Byte-shape-compatible with the old
/// `data.json` so `preprocessData` is unchanged.
#[derive(Debug, Serialize)]
pub struct Catalog {
    pub songs: Vec<Song>,
    pub categories: Vec<CategoryEntry>,
    pub versions: Vec<VersionEntry>,
    pub types: Vec<TypeEntry>,
    pub difficulties: Vec<DifficultyEntry>,
    pub regions: Vec<RegionEntry>,
    #[serde(rename = "updateTime")]
    pub update_time: String,
}

/// Raw row from the `songs` table (plus `to_char`-formatted `release_date`).
/// Field order MUST match the SELECT list in queries.rs::fetch_song_row /
/// fetch_all_songs — sqlx::query_as! maps positionally.
#[derive(sqlx::FromRow)]
pub struct SongRow {
    pub id: i64,
    pub song_id: Option<String>,
    pub category: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bpm: Option<f64>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub is_new: Option<bool>,
    pub is_locked: Option<bool>,
    pub comment: Option<String>,
}

impl SongRow {
    pub fn into_meta(self) -> SongMeta {
        SongMeta {
            song_id: self.song_id,
            category: self.category,
            title: self.title,
            artist: self.artist,
            bpm: self.bpm,
            image_name: self.image_name,
            version: self.version,
            release_date: self.release_date,
            is_new: self.is_new,
            is_locked: self.is_locked,
            comment: self.comment,
        }
    }
}

/// Raw row from the `sheets` table joined with its has_chart/note_counts/
/// regions/region_overrides aggregates. Field order MUST match the SELECT
/// list in queries.rs — sqlx::query_as! maps positionally.
pub struct SheetRow {
    pub song_id_fk: i64,
    pub r#type: Option<String>,
    pub difficulty: Option<String>,
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub internal_level: Option<String>,
    pub internal_level_value: Option<f64>,
    pub note_designer: Option<String>,
    pub is_special: Option<bool>,
    pub has_chart: bool,
    pub note_counts: Option<sqlx::types::Json<BTreeMap<String, Option<i64>>>>,
    pub regions: Option<sqlx::types::Json<BTreeMap<String, bool>>>,
    pub region_overrides: Option<sqlx::types::Json<BTreeMap<String, RegionOverride>>>,
}

impl SheetRow {
    pub fn into_meta(self) -> SheetMeta {
        SheetMeta {
            r#type: self.r#type,
            difficulty: self.difficulty,
            level: self.level,
            level_value: self.level_value,
            internal_level: self.internal_level,
            internal_level_value: self.internal_level_value,
            note_designer: self.note_designer,
            note_counts: self.note_counts.map(|j| j.0),
            regions: self.regions.map(|j| j.0),
            region_overrides: self.region_overrides.map(|j| j.0),
            is_special: self.is_special,
            has_chart: self.has_chart,
        }
    }
}
