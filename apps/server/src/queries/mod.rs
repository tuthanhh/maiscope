// Split by resource, mirroring routes/. get_chart's and sync's queries stay
// inline in their routes/*.rs handlers — they were never separate functions
// before this split, and extracting them now would mean rewriting control
// flow, not moving it (ticket 04's own constraint).
mod catalog;
mod sheets;
mod songs;

pub use catalog::{
    fetch_categories, fetch_difficulties, fetch_regions, fetch_types, fetch_update_time,
    fetch_versions,
};
pub use sheets::{
    SheetSearchParams, fetch_all_sheets, fetch_sheet_by_expr, fetch_sheets_for_song, search_sheets,
};
pub use songs::{fetch_all_songs, fetch_song_by_song_id};
