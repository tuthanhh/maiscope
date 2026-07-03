// Kept as the typed Rust mirror of the API contract. Handlers currently emit the
// shapes directly from Postgres JSON views (see migrations/*_json_views), so
// these structs aren't wired in yet — allow dead_code until they are.
#![allow(dead_code)]

// Response types mirroring apps/host/src/types/{Song,Sheet}.ts.
//
// Server emits RAW fields only. Derived fields the client computes in
// utils/data.ts:preprocessData (songNo, imageUrl, imageUrlM, sheetExpr,
// notePercents, $canonicalSheet) are intentionally absent here.

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
    // Record<string, Sheet> — partial per-region overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_overrides: Option<BTreeMap<String, Sheet>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_special: Option<bool>,
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
