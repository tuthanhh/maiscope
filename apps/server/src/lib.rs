// Shared between the `server` binary (phase 2's contribution-approve handler)
// and `src/bin/*.rs` binaries (seed_songs) — a `src/bin/*.rs` file is its own
// separate crate root and can't see main.rs's module tree, so anything both
// need to call has to live here instead.
pub mod catalog_sync;
pub mod chart_revision;
pub mod upstream;
