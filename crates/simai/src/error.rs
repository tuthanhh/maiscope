//! The parser's error type.
//!
//! Replaces the `std::io::Error` [`parse_chart`](super::parse_chart) used to
//! return, which was only ever a placeholder — nothing here touches IO, and
//! `ErrorKind::InvalidData` said nothing about *which* part of the chart was
//! invalid.

use std::fmt;

/// A chart that could not be parsed, and where.
///
/// Carries the three things needed to diagnose a failure from a single log
/// line: which comma it came from, what that comma said, and why it failed.
/// The token index is 0-based and counts every comma, including the empty ones
/// that carry rests — so it lines up with the chart text as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 0-based index of the comma-separated token that failed.
    pub token_index: usize,
    /// The token as written, before markers were stripped.
    pub token: String,
    /// What went wrong, from whichever sub-parser rejected it.
    pub cause: String,
}

impl ParseError {
    pub(super) fn new(token_index: usize, token: &str, cause: impl Into<String>) -> Self {
        Self {
            token_index,
            token: token.to_string(),
            cause: cause.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "token {} ('{}'): {}",
            self.token_index, self.token, self.cause
        )
    }
}

impl std::error::Error for ParseError {}
