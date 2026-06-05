//! One-line capture parser: turns free text like
//!   "did DSA for 45min"
//!   "skipped workout, too tired"
//!   "1 hour of reading and felt sharp"
//! into structured intent (habit + action + optional duration).
//!
//! No LLM dependency — heuristic matching against the user's existing habit
//! names. Always testable without a DB or network.

use crate::habits::LogStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCapture {
    /// Name of the matched habit (exact match against user's habits), if any.
    pub habit_match: Option<String>,
    /// Done by default; flips to Skipped if the text suggests a skip.
    pub status: LogStatus,
    /// Duration in minutes, parsed from things like "45min", "1 hour", "30m".
    pub duration_minutes: Option<i64>,
    /// The original input, preserved for journal logging.
    pub raw: String,
}

/// Parse a capture string against the user's existing habit names.
/// `habit_names` is intentionally `&[&str]` so the caller controls allocation.
pub fn parse(text: &str, habit_names: &[&str]) -> ParsedCapture {
    let lower = text.to_lowercase();

    let status = if has_skip_intent(&lower) {
        LogStatus::Skipped
    } else {
        LogStatus::Done
    };

    let habit_match = habit_names
        .iter()
        .filter(|name| lower.contains(&name.to_lowercase()))
        .max_by_key(|name| name.len())
        .map(|s| s.to_string());

    let duration_minutes = extract_duration(&lower);

    ParsedCapture {
        habit_match,
        status,
        duration_minutes,
        raw: text.to_string(),
    }
}

fn has_skip_intent(lower: &str) -> bool {
    const SKIP_WORDS: [&str; 4] = ["skip", "skipped", "didn't", "couldn't"];
    SKIP_WORDS.iter().any(|w| lower.contains(w))
}

/// Find the first "<number><optional space><unit>" pair in `text` and return
/// the duration as minutes. Recognizes: min, mins, minute(s), m, hour(s), hr,
/// hrs, h.
fn extract_duration(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let num_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let num: i64 = match text[num_start..i].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Optional whitespace between number and unit.
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        let rest = &text[i..];

        if let Some(m) = match_minute_unit(rest) {
            return Some(num.saturating_mul(m));
        }
    }
    None
}

/// Returns the multiplier to convert this unit to minutes, or None if `rest`
/// doesn't start with a recognized duration unit followed by a word boundary.
fn match_minute_unit(rest: &str) -> Option<i64> {
    // Multi-letter units first so "minute" doesn't match as "m".
    for (prefix, mul) in [
        ("minutes", 1),
        ("minute", 1),
        ("mins", 1),
        ("min", 1),
        ("hours", 60),
        ("hour", 60),
        ("hrs", 60),
        ("hr", 60),
    ] {
        if rest.starts_with(prefix) && ends_word(rest, prefix.len()) {
            return Some(mul);
        }
    }
    // Single-letter units: only valid if followed by non-letter (or EOS).
    let mut chars = rest.chars();
    let first = chars.next()?;
    let second_is_letter = chars.next().map(|c| c.is_alphabetic()).unwrap_or(false);
    if second_is_letter {
        return None;
    }
    match first {
        'm' => Some(1),
        'h' => Some(60),
        _ => None,
    }
}

fn ends_word(s: &str, at: usize) -> bool {
    s.as_bytes()
        .get(at)
        .map(|b| !b.is_ascii_alphabetic())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names<'a>(v: &'a [&'a str]) -> Vec<&'a str> {
        v.to_vec()
    }

    #[test]
    fn matches_habit_by_name() {
        let p = parse("did DSA today", &names(&["DSA", "Workout"]));
        assert_eq!(p.habit_match.as_deref(), Some("DSA"));
        assert_eq!(p.status, LogStatus::Done);
        assert_eq!(p.duration_minutes, None);
    }

    #[test]
    fn case_insensitive_match() {
        let p = parse("crushed some workout", &names(&["Workout"]));
        assert_eq!(p.habit_match.as_deref(), Some("Workout"));
    }

    #[test]
    fn longest_match_wins() {
        let p = parse("did DSA practice", &names(&["DSA", "DSA practice"]));
        assert_eq!(p.habit_match.as_deref(), Some("DSA practice"));
    }

    #[test]
    fn no_match_when_habit_missing() {
        let p = parse("went for a walk", &names(&["Read", "Meditate"]));
        assert_eq!(p.habit_match, None);
    }

    #[test]
    fn detects_skip_intent() {
        let p = parse("skipped workout", &names(&["Workout"]));
        assert_eq!(p.status, LogStatus::Skipped);
    }

    #[test]
    fn detects_skip_via_contraction() {
        let p = parse("didn't read today", &names(&["Read"]));
        assert_eq!(p.status, LogStatus::Skipped);
    }

    #[test]
    fn extracts_45min() {
        let p = parse("did DSA for 45min", &names(&["DSA"]));
        assert_eq!(p.duration_minutes, Some(45));
    }

    #[test]
    fn extracts_45_min_with_space() {
        let p = parse("DSA 45 min", &names(&["DSA"]));
        assert_eq!(p.duration_minutes, Some(45));
    }

    #[test]
    fn extracts_1_hour() {
        let p = parse("1 hour of reading", &names(&["Read"]));
        assert_eq!(p.duration_minutes, Some(60));
    }

    #[test]
    fn extracts_2hrs() {
        let p = parse("Workout 2hrs", &names(&["Workout"]));
        assert_eq!(p.duration_minutes, Some(120));
    }

    #[test]
    fn extracts_30m_short() {
        let p = parse("Read 30m today", &names(&["Read"]));
        assert_eq!(p.duration_minutes, Some(30));
    }

    #[test]
    fn does_not_treat_word_starting_with_m_as_minutes() {
        // "30 minute" still matches, but "30 meals" should not be 30m.
        let p = parse("30 meals eaten", &names(&[]));
        assert_eq!(p.duration_minutes, None);
    }

    #[test]
    fn empty_input_is_safe() {
        let p = parse("", &names(&["DSA"]));
        assert_eq!(p.habit_match, None);
        assert_eq!(p.duration_minutes, None);
        assert_eq!(p.status, LogStatus::Done);
        assert_eq!(p.raw, "");
    }
}
