//! The upstream `data.json` shape, mirroring `apps/host/src/types/{Data,Song,Sheet}.ts`.
//!
//! Lives in the library rather than in a binary because `catalog_sync` and its
//! tests both need it — a `src/bin/*.rs` file is its own crate root and cannot
//! be imported.

use serde::Deserialize;
use sqlx::types::chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeMap;

// ── upstream data.json shape (raw fields; mirrors types/{Data,Song,Sheet}.ts) ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawData {
    #[serde(default)]
    pub songs: Vec<RawSong>,
    #[serde(default)]
    pub categories: Vec<RawCategory>,
    #[serde(default)]
    pub versions: Vec<RawVersion>,
    #[serde(default)]
    pub types: Vec<RawType>,
    #[serde(default)]
    pub difficulties: Vec<RawDifficulty>,
    #[serde(default)]
    pub regions: Vec<RawRegion>,
    pub update_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawCategory {
    pub category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawVersion {
    pub version: String,
    pub abbr: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawType {
    pub r#type: String,
    pub name: String,
    pub abbr: Option<String>,
    pub icon_url: Option<String>,
    pub icon_height: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawDifficulty {
    pub difficulty: String,
    pub name: String,
    pub color: Option<String>,
    pub icon_url: Option<String>,
    pub icon_height: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct RawRegion {
    pub region: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSong {
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
    #[serde(default)]
    pub sheets: Vec<RawSheet>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSheet {
    pub r#type: Option<String>,
    pub difficulty: Option<String>,
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub internal_level: Option<String>,
    pub internal_level_value: Option<f64>,
    pub note_designer: Option<String>,
    pub note_counts: Option<BTreeMap<String, Option<i64>>>,
    pub regions: Option<BTreeMap<String, bool>>,
    pub region_overrides: Option<BTreeMap<String, RawOverride>>,
    pub is_special: Option<bool>,
}

// `PartialEq` because `catalog_sync` compares an incoming override against the
// stored one to decide whether a sheet's sub-tables actually changed.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawOverride {
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub internal_level: Option<String>,
    pub internal_level_value: Option<f64>,
    pub note_designer: Option<String>,
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// `versions[].releaseDate` and `songs[].releaseDate`: bare calendar dates.
pub fn parse_date(s: &Option<String>) -> Option<NaiveDate> {
    s.as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// `Data.updateTime`, which is **not** a bare date: the live feed sends an
/// RFC 3339 instant (`2026-09-11T01:24:07.945Z`). Parsing it with
/// [`parse_date`] alone silently yielded `None`, and `catalog_sync` then fell
/// back to `now()` on every single run — which changed the `/catalog` ETag
/// daily and made every client refetch the whole 4.7MB payload.
///
/// Kept separate from [`parse_date`] so the date-only callers above keep
/// rejecting anything that is not `YYYY-MM-DD`. A vendored or hand-written
/// snapshot may still carry the bare date, so both shapes are accepted here.
pub fn parse_update_time(s: &Option<String>) -> Option<DateTime<Utc>> {
    let s = s.as_deref()?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)
        .map(|ndt| ndt.and_utc())
}

// sheet_expr = songId|type|difficulty (mirrors utils/sheet.ts:computeSheetExpr,
// where a null/undefined part stringifies to its JS literal).
pub fn sheet_expr(song_id: &Option<String>, ty: &Option<String>, diff: &Option<String>) -> String {
    format!(
        "{}|{}|{}",
        song_id.as_deref().unwrap_or("null"),
        ty.as_deref().unwrap_or("undefined"),
        diff.as_deref().unwrap_or("undefined"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimal_payload() {
        let json = r#"{
            "updateTime": "2026-09-11",
            "songs": [{
                "songId": "Example",
                "title": "Example",
                "sheets": [{ "type": "dx", "difficulty": "master", "level": "14+" }]
            }],
            "categories": [{ "category": "pops" }]
        }"#;

        let data: RawData = serde_json::from_str(json).unwrap();

        assert_eq!(data.songs.len(), 1);
        assert_eq!(data.songs[0].song_id.as_deref(), Some("Example"));
        assert_eq!(data.songs[0].sheets.len(), 1);
        assert_eq!(data.categories.len(), 1);
        // Absent arrays default to empty rather than failing to parse.
        assert!(data.regions.is_empty());
    }

    // The exact string the live feed sends, verified against
    // https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json on 2026-09-13.
    // Fixtures that used a bare date hid the fallback-to-now() bug entirely.
    #[test]
    fn update_time_parses_the_live_rfc_3339_shape() {
        let parsed = parse_update_time(&Some("2026-09-11T01:24:07.945Z".into()))
            .expect("the live updateTime must parse");
        assert_eq!(parsed.to_rfc3339(), "2026-09-11T01:24:07.945+00:00");
    }

    #[test]
    fn update_time_still_accepts_a_bare_date() {
        let parsed = parse_update_time(&Some("2026-09-11".into()))
            .expect("a vendored snapshot may be terse");
        assert_eq!(parsed.to_rfc3339(), "2026-09-11T00:00:00+00:00");
    }

    #[test]
    fn update_time_rejects_garbage_rather_than_guessing() {
        assert!(parse_update_time(&None).is_none());
        assert!(parse_update_time(&Some("not a date".into())).is_none());
    }

    // parse_date's callers (versions[].releaseDate) must keep rejecting
    // instants — widening it there would write a wrong DATE silently.
    #[test]
    fn parse_date_remains_date_only() {
        assert!(parse_date(&Some("2026-09-11".into())).is_some());
        assert!(parse_date(&Some("2026-09-11T01:24:07.945Z".into())).is_none());
    }

    #[test]
    fn sheet_expr_uses_js_literals_for_missing_parts() {
        assert_eq!(
            sheet_expr(
                &Some("Example".into()),
                &Some("dx".into()),
                &Some("master".into())
            ),
            "Example|dx|master"
        );
        assert_eq!(sheet_expr(&None, &None, &None), "null|undefined|undefined");
    }
}
