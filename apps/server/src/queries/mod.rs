// Split by resource, mirroring routes/. get_chart's and sync's queries stay
// inline in their routes/*.rs handlers — they were never separate functions
// before this split, and extracting them now would mean rewriting control
// flow, not moving it (ticket 04's own constraint).
mod catalog;
mod official;
mod reference;
mod sheets;
mod songs;
mod users;

pub use catalog::{
    fetch_categories, fetch_difficulties, fetch_regions, fetch_types, fetch_update_time,
    fetch_versions,
};
pub use official::{fetch_official_sheets, fetch_official_songs};
pub use reference::fetch_reference;
pub use sheets::{
    SheetSearchParams, fetch_all_sheets, fetch_sheet_by_expr, fetch_sheets_for_song, search_sheets,
};
pub use songs::{fetch_all_songs, fetch_song_by_song_id};
pub use users::{
    fetch_active_user, rotate_session, start_session, tombstone_user, upsert_github_user,
};
