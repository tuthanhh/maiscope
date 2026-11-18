// The server's wire and row types: the users and sessions of contract §4, and
// the read API's shapes below. Each carries only the fields a screen reads
// (ADR-0026, docs/reference/screen-data-requirements.md); the client derives
// the rest (`imageUrl`, `sheet.song`, `sheet.key`).
//
// This is the single source of truth for the response shape: the handlers
// build these types from `src/queries/`'s `sqlx::query_as!` queries, then
// serde serializes them directly.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A region's partial override of a Sheet's level/designer fields.
/// Row existence in `sheet_region_overrides` = override applies; NULL columns
/// inherit the canonical sheet value (contract §0, `regionOverrides`).
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

/// Raw row from `users`, as login returns it. `login` is non-null here: only a
/// tombstone has none, and login is what ends a tombstone.
#[derive(sqlx::FromRow)]
pub struct UserRow {
    pub id: sqlx::types::Uuid,
    pub login: String,
    pub avatar_url: Option<String>,
    pub role: String,
}

/// A user as contract §4 shows one. `id` is the UUID as a string.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub login: String,
    pub avatar_url: Option<String>,
    pub role: crate::auth::Role,
}

/// `GET /auth/github/callback` response: a session for this device.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: User,
}

/// `POST /auth/refresh` response: the rotated pair for this device.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPair {
    pub token: String,
    pub refresh_token: String,
}

// ── read API (api-rewrite) ─────────────────────────────────────────────────

/// `GET /api/reference`, and the same five arrays inside `GET /api/catalog`
/// (ADR-0021). Each list is in its table's `ordinal` order.
#[derive(Debug, Serialize)]
pub struct Reference {
    pub categories: Vec<ReferenceCategory>,
    pub versions: Vec<ReferenceVersion>,
    pub types: Vec<ReferenceType>,
    pub difficulties: Vec<ReferenceDifficulty>,
    pub regions: Vec<ReferenceRegion>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceCategory {
    pub category: String,
}

#[derive(Debug, Serialize)]
pub struct ReferenceVersion {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceType {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
}

/// Keyed by `code`, the form a sheet's `difficulty` and its URL carry
/// (`master`, `協`); `name` is the display form (`MASTER`, `【協】`).
#[derive(Debug, Serialize)]
pub struct ReferenceDifficulty {
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceRegion {
    pub region: String,
    pub name: String,
}

/// A row of `songs` with the fields the v2 payload carries. `query_as!` maps
/// columns to fields by name.
pub struct OfficialSongRow {
    pub id: sqlx::types::Uuid,
    pub category: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bpm: Option<f64>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub is_locked: Option<bool>,
    pub comment: Option<String>,
}

/// A row of `sheets` with its aggregates, `difficulty` already the code.
pub struct OfficialSheetRow {
    pub song_id: sqlx::types::Uuid,
    pub r#type: Option<String>,
    pub difficulty: Option<String>,
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub internal_level: Option<String>,
    pub internal_level_value: Option<f64>,
    pub note_designer: Option<String>,
    pub has_chart: bool,
    pub note_counts: Option<sqlx::types::Json<BTreeMap<String, Option<i64>>>>,
    pub regions: Option<sqlx::types::Json<BTreeMap<String, bool>>>,
    pub region_overrides: Option<sqlx::types::Json<BTreeMap<String, RegionOverride>>>,
}

/// A song's own fields, as every v2 payload carries them. `id` is the UUID,
/// hyphenated. Absent fields are omitted rather than `null`, as in v1: the
/// Catalog is 3MB and the client keys optional rows on presence.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialSongFields {
    pub id: String,
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
    pub is_locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// A sheet's own fields. `difficulty` is `difficulties.code` (ADR-0026).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialSheetFields {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_counts: Option<BTreeMap<String, Option<i64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regions: Option<BTreeMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_overrides: Option<BTreeMap<String, RegionOverride>>,
    /// Always present: whether the chart endpoint has text for this sheet.
    pub has_chart: bool,
}

/// A song with its sheets: an element of the Catalog's `songs`, and the whole
/// of `GET /api/songs/{id}`.
#[derive(Debug, Serialize)]
pub struct OfficialSong {
    #[serde(flatten)]
    pub fields: OfficialSongFields,
    pub sheets: Vec<OfficialSheetFields>,
}

/// `GET /api/catalog`: the songs, and the reference tables beside them.
#[derive(Debug, Serialize)]
pub struct OfficialCatalog {
    pub songs: Vec<OfficialSong>,
    #[serde(flatten)]
    pub reference: Reference,
}

impl From<OfficialSongRow> for OfficialSongFields {
    fn from(row: OfficialSongRow) -> Self {
        OfficialSongFields {
            id: row.id.to_string(),
            category: row.category,
            title: row.title,
            artist: row.artist,
            bpm: row.bpm,
            image_name: row.image_name,
            version: row.version,
            release_date: row.release_date,
            is_locked: row.is_locked,
            comment: row.comment,
        }
    }
}

impl From<OfficialSheetRow> for OfficialSheetFields {
    fn from(row: OfficialSheetRow) -> Self {
        OfficialSheetFields {
            r#type: row.r#type,
            difficulty: row.difficulty,
            level: row.level,
            level_value: row.level_value,
            internal_level: row.internal_level,
            internal_level_value: row.internal_level_value,
            note_designer: row.note_designer,
            note_counts: row.note_counts.map(|j| j.0),
            regions: row.regions.map(|j| j.0),
            region_overrides: row.region_overrides.map(|j| j.0),
            has_chart: row.has_chart,
        }
    }
}

/// `GET /api/songs/{id}/sheets/{type}/{difficulty}`: the sheet's own fields
/// and an explicit `song`, the same reference the client builds for the
/// Catalog's sheets — not v1's flattening of song fields into the sheet.
#[derive(Debug, Serialize)]
pub struct OfficialSheet {
    #[serde(flatten)]
    pub fields: OfficialSheetFields,
    pub song: OfficialSongFields,
}
