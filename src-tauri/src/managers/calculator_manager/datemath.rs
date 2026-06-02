//! Date-arithmetic helpers (UTIL.1.C).
//!
//! Extracted from `calculator_manager.rs` (REF.9.B) as a pure structural move;
//! behavior unchanged.

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

pub(super) fn contains_date_literal(s: &str) -> bool {
    // Match YYYY-MM-DD or YYYY/MM/DD; loose check, full parse happens later.
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        let chunk = &s[i..i + 10];
        if chunk.chars().enumerate().all(|(idx, c)| match (idx, c) {
            (0..=3, c) => c.is_ascii_digit(),
            (4, '-') | (4, '/') => true,
            (5..=6, c) => c.is_ascii_digit(),
            (7, '-') | (7, '/') => true,
            (8..=9, c) => c.is_ascii_digit(),
            _ => false,
        }) {
            return true;
        }
    }
    false
}

fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    let normalized = s.replace('/', "-");
    NaiveDate::parse_from_str(&normalized, "%Y-%m-%d").ok()
}

pub(super) fn parse_date_anchor(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let trimmed = s.trim();
    match trimmed {
        "today" => Some(today),
        "tomorrow" => today.checked_add_days(Days::new(1)),
        "yesterday" => today.checked_sub_days(Days::new(1)),
        other => parse_iso_date(other),
    }
}

pub(super) fn parse_weekday(s: &str) -> Result<Weekday, String> {
    match s.trim() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        other => Err(format!("unknown weekday '{other}'")),
    }
}

pub(super) fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = from.weekday().num_days_from_monday();
    let tgt = target.num_days_from_monday();
    let diff = if tgt > cur {
        tgt - cur
    } else {
        7 - (cur - tgt)
    };
    from.checked_add_days(Days::new(diff as u64))
        .unwrap_or(from)
}

pub(super) fn previous_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = from.weekday().num_days_from_monday();
    let tgt = target.num_days_from_monday();
    let diff = if cur > tgt {
        cur - tgt
    } else {
        7 - (tgt - cur)
    };
    from.checked_sub_days(Days::new(diff as u64))
        .unwrap_or(from)
}

pub(super) fn parse_n_unit(s: &str) -> Result<(u32, String), String> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("expected '<N> <unit>', got '{s}'"));
    }
    let n: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid number '{}'", parts[0]))?;
    let unit = parts[1].trim_end_matches('s').to_string(); // strip plural
    Ok((n, unit))
}

pub(super) fn add_to(date: NaiveDate, n: u32, unit: &str) -> Result<NaiveDate, String> {
    match unit {
        "day" => date
            .checked_add_days(Days::new(n as u64))
            .ok_or_else(|| "date overflow".into()),
        "week" => date
            .checked_add_days(Days::new(n as u64 * 7))
            .ok_or_else(|| "date overflow".into()),
        "month" => date
            .checked_add_months(Months::new(n))
            .ok_or_else(|| "date overflow".into()),
        "year" => date
            .checked_add_months(Months::new(n * 12))
            .ok_or_else(|| "date overflow".into()),
        other => Err(format!("unknown date unit '{other}'")),
    }
}

pub(super) fn subtract_from(date: NaiveDate, n: u32, unit: &str) -> Result<NaiveDate, String> {
    match unit {
        "day" => date
            .checked_sub_days(Days::new(n as u64))
            .ok_or_else(|| "date underflow".into()),
        "week" => date
            .checked_sub_days(Days::new(n as u64 * 7))
            .ok_or_else(|| "date underflow".into()),
        "month" => date
            .checked_sub_months(Months::new(n))
            .ok_or_else(|| "date underflow".into()),
        "year" => date
            .checked_sub_months(Months::new(n * 12))
            .ok_or_else(|| "date underflow".into()),
        other => Err(format!("unknown date unit '{other}'")),
    }
}

/// Split on the rightmost top-level ` + ` or ` - ` (not inside numeric literal).
/// Returns `(lhs, op, rhs)` or `None` if no operator found.
pub(super) fn split_top_level_arith(s: &str) -> Option<(&str, char, &str)> {
    // Find rightmost " + " or " - " surrounded by spaces.
    let mut idx_op: Option<(usize, char)> = None;
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if i + 2 >= bytes.len() {
            break;
        }
        if bytes[i] == b' '
            && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-')
            && bytes[i + 2] == b' '
        {
            idx_op = Some((i, bytes[i + 1] as char));
        }
    }
    idx_op.map(|(i, op)| (&s[..i], op, &s[i + 3..]))
}
