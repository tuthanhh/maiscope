//! `maidata.txt`: one song's header plus up to one chart per difficulty slot.
//!
//! The file is a flat list of `&key=value` pairs. A value runs from its
//! `&key=` line to the next line that starts with `&`, so an `&inote_N=`
//! body spans many lines. Slots pair by number: `&lv_4=`, `&des_4=` and
//! `&inote_4=` describe one difficulty.

/// A parsed `maidata.txt`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Maidata {
    /// `&title=`, trimmed. Present but possibly empty.
    pub title: String,
    pub artist: Option<String>,
    /// `&wholebpm=` as written: the format does not guarantee a number.
    pub wholebpm: Option<String>,
    pub genre: Option<String>,
    pub version: Option<String>,
    /// One entry per slot the file mentions, in slot order.
    pub difficulties: Vec<Difficulty>,
    /// Every other `&key=value`, in file order and untrimmed: keys this crate
    /// does not read (`&first=`, `&des=`, `&cabinet=`, ...) are kept, not
    /// dropped, so a caller storing the file loses nothing.
    pub other: Vec<(String, String)>,
}

/// One difficulty slot: `&lv_N=`, `&des_N=` and `&inote_N=` for one `N`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Difficulty {
    /// maidata's slot number: 1 EASY, 2 BASIC to 6 Re:MASTER, 7 UTAGE.
    pub slot: u8,
    /// `&lv_N=`, trimmed. For slot 7 this is the UTAGE tag (`協`).
    pub lv: Option<String>,
    /// `&des_N=`, trimmed: the note designer.
    pub des: Option<String>,
    /// `&inote_N=` exactly as written. `None` when the key is absent or its
    /// body is blank: an empty chart is no chart.
    pub inote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaidataError {
    /// No `&title=` key at all. Every song has one.
    MissingTitle,
}

impl std::fmt::Display for MaidataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaidataError::MissingTitle => f.write_str("no &title= in maidata"),
        }
    }
}

impl std::error::Error for MaidataError {}

/// The difficulty code a standard slot stores as (`sheets.difficulty` in the
/// catalog). Slot 7 is UTAGE, whose code is the tag in `&lv_7=`, and slot 1
/// (EASY) has no catalog sheet; both are `None`.
pub fn standard_difficulty(slot: u8) -> Option<&'static str> {
    match slot {
        2 => Some("basic"),
        3 => Some("advanced"),
        4 => Some("expert"),
        5 => Some("master"),
        6 => Some("remaster"),
        _ => None,
    }
}

/// Parse a whole `maidata.txt`. A leading byte-order mark is ignored, and
/// line endings may be CRLF or LF, mixed. A key that appears twice keeps its
/// last value. Chart bodies are not parsed here; hand each `inote` to
/// [`crate::parse_chart`].
pub fn parse(text: &str) -> Result<Maidata, MaidataError> {
    let mut title: Option<String> = None;
    let mut m = Maidata::default();
    let mut slots: std::collections::BTreeMap<u8, Difficulty> = Default::default();

    for (key, value) in pairs(text) {
        let slot_key = key
            .split_once('_')
            .and_then(|(name, n)| Some((name, n.parse::<u8>().ok()?)));
        match (key.as_str(), slot_key) {
            ("title", _) => title = Some(value.trim().to_string()),
            ("artist", _) => m.artist = Some(value.trim().to_string()),
            ("wholebpm", _) => m.wholebpm = Some(value.trim().to_string()),
            ("genre", _) => m.genre = Some(value.trim().to_string()),
            ("version", _) => m.version = Some(value.trim().to_string()),
            (_, Some((field @ ("lv" | "des" | "inote"), slot))) => {
                let d = slots.entry(slot).or_insert_with(|| Difficulty {
                    slot,
                    ..Difficulty::default()
                });
                match field {
                    "lv" => d.lv = Some(value.trim().to_string()),
                    "des" => d.des = Some(value.trim().to_string()),
                    _ => d.inote = (!value.trim().is_empty()).then_some(value),
                }
            }
            _ => m.other.push((key, value)),
        }
    }

    m.title = title.ok_or(MaidataError::MissingTitle)?;
    m.difficulties = slots.into_values().collect();
    Ok(m)
}

/// `&key=value` pairs in file order. A value runs from its `&key=` line to
/// the next line that starts with `&` and has an `=`, its lines joined by
/// `\n` (so CRLF does not survive into a value).
fn pairs(text: &str) -> Vec<(String, String)> {
    // A byte-order mark would sit in front of the first `&` and hide that key.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('&')
            && let Some((key, first)) = rest.split_once('=')
        {
            if let Some((k, lines)) = current.take() {
                out.push((k, lines.join("\n")));
            }
            current = Some((key.to_string(), vec![first]));
        } else if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    if let Some((k, lines)) = current {
        out.push((k, lines.join("\n")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a real file from the local sample: a byte-order mark,
    /// CRLF on the header but LF inside a chart body, keys this crate does
    /// not read, and slots 2 to 5. The chart bodies are hand-written; real
    /// chart text is never committed (`tests/fixtures/README.md`).
    const MULTI: &str = "\u{feff}&title=Example Song\r\n&artist=Someone feat. Another\r\n&wholebpm=180\r\n&first=0.05\r\n&des=Designer\r\n&lv_2=3\r\n&lv_3=6\r\n&lv_4=9\r\n&lv_5=12+\r\n&des_5=Mapper\r\n&inote_2=(180){4}1,2,\nE\r\n&inote_3=(180){4}1,2,3,4,\nE\r\n&inote_4=(180){8}1h[4:1],,5,\nE\r\n&inote_5=(180){8}1-5[8:1],\n{4}2,\nE\r\n";

    #[test]
    fn a_multi_difficulty_file_pairs_each_slot() {
        let m = parse(MULTI).unwrap();

        assert_eq!(m.title, "Example Song");
        assert_eq!(m.artist.as_deref(), Some("Someone feat. Another"));
        assert_eq!(m.wholebpm.as_deref(), Some("180"));
        assert_eq!(m.genre, None);
        assert_eq!(
            m.difficulties.iter().map(|d| d.slot).collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        let master = &m.difficulties[3];
        assert_eq!(master.lv.as_deref(), Some("12+"));
        assert_eq!(master.des.as_deref(), Some("Mapper"));
        // A body keeps its own line breaks; only the line the next key sits
        // on ends it.
        assert_eq!(master.inote.as_deref(), Some("(180){8}1-5[8:1],\n{4}2,\nE"));
        assert_eq!(m.difficulties[0].des, None);
    }

    #[test]
    fn keys_this_crate_does_not_read_are_kept_in_order() {
        let m = parse(MULTI).unwrap();
        assert_eq!(
            m.other,
            [
                ("first".to_string(), "0.05".to_string()),
                ("des".to_string(), "Designer".to_string()),
            ]
        );
    }

    #[test]
    fn gaps_in_the_slot_numbering_stay_gaps() {
        let m = parse("&title=Gap\n&lv_3=7\n&inote_3=1,\n&lv_6=13\n&inote_6=2,\n").unwrap();
        assert_eq!(
            m.difficulties.iter().map(|d| d.slot).collect::<Vec<_>>(),
            [3, 6]
        );
    }

    #[test]
    fn crlf_line_endings_leave_no_carriage_returns_behind() {
        let m = parse("&title=Crlf\r\n&artist=A\r\n&lv_5=13\r\n&inote_5=1,\r\n2,\r\n").unwrap();
        assert_eq!(m.title, "Crlf");
        assert_eq!(m.artist.as_deref(), Some("A"));
        assert_eq!(m.difficulties[0].lv.as_deref(), Some("13"));
        assert_eq!(m.difficulties[0].inote.as_deref(), Some("1,\n2,"));
    }

    #[test]
    fn a_missing_title_is_an_error() {
        assert_eq!(parse("&artist=Someone\n&lv_5=12\n"), Err(MaidataError::MissingTitle));
    }

    #[test]
    fn a_blank_chart_body_is_no_chart() {
        let m = parse("&title=Blank\n&lv_5=12\n&inote_5=\n  \n&lv_7=協\n&inote_7=1,\n").unwrap();
        assert_eq!(m.difficulties[0].lv.as_deref(), Some("12"));
        assert_eq!(m.difficulties[0].inote, None);
        assert_eq!(m.difficulties[1].lv.as_deref(), Some("協"));
        assert_eq!(m.difficulties[1].inote.as_deref(), Some("1,"));
    }

    #[test]
    fn standard_slots_map_to_catalog_codes() {
        assert_eq!(standard_difficulty(5), Some("master"));
        assert_eq!(standard_difficulty(1), None);
        assert_eq!(standard_difficulty(7), None);
    }
}
