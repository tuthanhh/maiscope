//! Shared test helpers for the parser modules. Compiled only under `cfg(test)`.
//!
//! These exist so a test reads as one line of intent — the simai string in,
//! the expected [`Note`] out — with the unwrapping and arity checks factored
//! out. Constructing expected values is boilerplate; deciding *what* the
//! expected value is, is the test.
//!
//! `dead_code` is allowed module-wide: a helper is unused until the stub that
//! needs it is written, and deleting it would just mean writing it again.
#![allow(dead_code)]

use super::note::parse_note;
use crate::systems::component::{Duration, Note, NoteKind, SlideSegment, SlideShape};

/// Parse, panicking with the parser's own message on failure.
pub(super) fn parse(s: &str) -> Vec<Note> {
    parse_note(s).unwrap_or_else(|e| panic!("parse_note({s:?}) failed: {e}"))
}

/// Parse and assert exactly one note came back.
pub(super) fn parse_one(s: &str) -> Note {
    let notes = parse(s);
    assert_eq!(notes.len(), 1, "expected 1 note from {s:?}, got {notes:#?}");
    notes.into_iter().next().unwrap()
}

/// Assert the parse failed, and hand back the message so the test can check it.
pub(super) fn parse_err(s: &str) -> String {
    match parse_note(s) {
        Err(e) => e,
        Ok(notes) => panic!("expected {s:?} to fail, but it parsed as {notes:#?}"),
    }
}

/// A note with no modifiers and no sub-comma offset — the common case.
pub(super) fn plain(kind: NoteKind) -> Note {
    Note {
        is_break: false,
        is_firework: false,
        is_ex: false,
        offset_ms: 0,
        kind,
    }
}

/// A note delayed by a pseudo-EACH backtick: `` 1`2 `` puts the second at 1.
pub(super) fn delayed(offset_ms: u32, kind: NoteKind) -> Note {
    Note {
        offset_ms,
        ..plain(kind)
    }
}

/// `Duration::Simple`, the only duration a tap/touch hold produces.
pub(super) fn simple(divider: usize, count: usize) -> Duration {
    Duration::Simple { divider, count }
}

/// One slide segment carrying the given break flag.
pub(super) fn seg(shape: SlideShape, duration: Duration, is_break: bool) -> SlideSegment {
    SlideSegment {
        shape,
        duration,
        is_break,
    }
}
