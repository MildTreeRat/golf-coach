//! An instant, with the offset it was written in — a pydantic `datetime` field. [M36 P4]
//!
//! **No Python module to mirror**, the second such module after [`crate::capability`]: Python
//! reaches for `datetime` in every contract that holds a time, and this is what each of those
//! fields is on this side. It replaced a lexeme newtype that carried the string as it arrived,
//! whose own doc said the day a phase needed to *order* two shots it would become a parsed type
//! with vectors of its own. M36 is that day: the shot store lists shots newest first by instant,
//! where `08:00-05:00` leads `12:00Z` and a lexical order says the reverse, and the corpus, the
//! bag profile and the session sort all compare instants too.
//!
//! The gate is `spec/vectors/format/timestamp.json`, recorded from pydantic (and named in its
//! provenance beside the interpreter), and run by `tests/time.rs`.
//!
//! # What it reads, and the divergence it names
//!
//! RFC 3339 as pydantic writes it, and the spellings of that it re-spells:
//! `YYYY-MM-DDTHH:MM:SS`, an optional fraction of one to six digits, then `Z` or `±HH:MM`. Every
//! timestamp on disk is in that grammar, because pydantic wrote every one of them.
//!
//! **pydantic reads more than that, and this refuses the rest** (the M36 plan's call 9): a naive
//! value with no offset, a space or a lowercase `t` for the `T`, a lowercase `z`, `+0530`, a time
//! with no seconds, a fraction past six digits (which pydantic truncates), a comma for the point, a
//! Unix number. Nothing here writes any of them, and a naive one is the dangerous kind: frozen
//! Python reads it, and then cannot compare it with an aware one at all (`TypeError`). So a
//! contract that holds one of these is refused at deserialization, where Python would have taken
//! it. `tests/time.rs` pins each, with pydantic's own answer beside it.
//!
//! # What it writes
//!
//! pydantic's spelling, with the offset it was read with: `Z` for a zero offset (including
//! `+00:00` and `-00:00` as read), `±HH:MM` otherwise; the fraction as six digits whenever it is
//! non-zero, trailing zeros kept (`.120000`), and dropped when it is zero. So every canonical
//! lexeme round-trips byte for byte, which is what kept the engine and screen families unchanged
//! when this replaced the lexeme type, and a non-canonical one comes back the way frozen Python
//! would have written it.
//!
//! [`Timestamp::isoformat`] is the other spelling, CPython's `datetime.isoformat()`, which
//! `storage/corpus.py::_arrival` sorts on **as a string**; [`Timestamp::date_ymd`] is
//! `f"{dt:%Y-%m-%d}"` in the value's own offset, for the bag profile's dates.
//!
//! # Equality and order are by instant, as Python's are
//!
//! Python compares two aware datetimes by the instant alone: `12:00Z` and `17:30+05:30` are `==`
//! and hash alike, though each writes back its own offset. [`PartialEq`], [`Ord`] and [`Hash`] here
//! are the same, so a sort or a strict `<` ports as written. A caller that needs the spelling to
//! match compares the strings.
//!
//! **Hand-rolled, not `chrono`** (the M36 plan's decision 6): `chrono`'s serializer writes `.300`
//! for `.300000`, so it would need a custom one to match pydantic anyway, and it would be the first
//! dependency this crate takes beyond `serde`. What is here is a parser, two writers and Howard
//! Hinnant's civil-calendar arithmetic, which is exact over the whole proleptic Gregorian range.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ContractError, Validate};

const MICROS_PER_SECOND: i64 = 1_000_000;
const MICROS_PER_MINUTE: i64 = 60 * MICROS_PER_SECOND;
const MICROS_PER_DAY: i64 = 86_400 * MICROS_PER_SECOND;

/// The days from 0000-03-01 to 1970-01-01 in the proleptic Gregorian calendar, which shifts
/// Hinnant's March-based era arithmetic onto the Unix epoch.
const EPOCH_SHIFT_DAYS: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;

/// An instant and the offset it is spelled in.
///
/// Microseconds, because that is a Python `datetime`'s resolution and so every value on disk has at
/// most six fraction digits. `i64` microseconds reach about 292,000 years either side of 1970, far
/// past the 0001–9999 a `datetime` can hold.
#[derive(Clone, Copy)]
pub struct Timestamp {
    /// Microseconds since 1970-01-01T00:00:00Z.
    utc_micros: i64,
    /// Minutes east of UTC, within ±23:59 — what pydantic accepts, and `timezone`'s own bound.
    offset_minutes: i16,
}

/// A string [`Timestamp::parse`] refuses, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTimestampError {
    pub text: String,
    pub problem: &'static str,
}

impl fmt::Display for ParseTimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a timestamp as pydantic writes one: {}",
            self.text, self.problem
        )
    }
}

impl std::error::Error for ParseTimestampError {}

impl Timestamp {
    /// Read pydantic's grammar (the module doc), refusing everything outside it.
    pub fn parse(text: &str) -> Result<Self, ParseTimestampError> {
        let refuse = |problem| ParseTimestampError {
            text: text.to_string(),
            problem,
        };
        // Bytes, not chars: every accepted character is ASCII, so a multi-byte one (a full-width
        // digit, say) fails the digit test where it stands rather than shifting the fields.
        let bytes = text.as_bytes();
        if bytes.len() < 20 {
            return Err(refuse("too short for YYYY-MM-DDTHH:MM:SS and an offset"));
        }
        let field = |at: usize, width: usize| digits(&bytes[at..at + width]);
        let punctuation = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
        if punctuation.iter().any(|&(at, want)| bytes[at] != want) {
            return Err(refuse("not laid out as YYYY-MM-DDTHH:MM:SS"));
        }
        let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
            field(0, 4),
            field(5, 2),
            field(8, 2),
            field(11, 2),
            field(14, 2),
            field(17, 2),
        ) else {
            return Err(refuse("a date or time field is not all ASCII digits"));
        };
        if !(1..=9999).contains(&year) {
            return Err(refuse("the year is outside 0001-9999"));
        }
        if !(1..=12).contains(&month) {
            return Err(refuse("the month is outside 01-12"));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(refuse("the day is not in that month"));
        }
        if hour > 23 || minute > 59 || second > 59 {
            // pydantic refuses `24:00:00` and the leap second `:60` alike, since a `datetime`
            // can hold neither.
            return Err(refuse("the time of day is out of range"));
        }

        let mut rest = &bytes[19..];
        let mut micros = 0i64;
        if let Some(fraction) = rest.strip_prefix(b".") {
            let width = fraction.iter().take_while(|b| b.is_ascii_digit()).count();
            if width == 0 {
                return Err(refuse("a decimal point with no digits after it"));
            }
            if width > 6 {
                // pydantic truncates a seventh digit onwards; nothing here writes one.
                return Err(refuse("more than six fraction digits"));
            }
            let value = digits(&fraction[..width]).expect("counted as ASCII digits");
            micros = value * 10i64.pow(6 - width as u32);
            rest = &fraction[width..];
        }

        let offset_minutes = match rest {
            b"Z" => 0,
            [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
                let (Some(hours), Some(mins)) = (digits(&[*h1, *h2]), digits(&[*m1, *m2])) else {
                    return Err(refuse("the offset is not ±HH:MM in ASCII digits"));
                };
                if hours > 23 || mins > 59 {
                    return Err(refuse("the offset is outside ±23:59"));
                }
                let magnitude = (hours * 60 + mins) as i16;
                if *sign == b'-' {
                    -magnitude
                } else {
                    magnitude
                }
            }
            _ => return Err(refuse("no offset, or not `Z` or ±HH:MM")),
        };

        let local_micros = days_from_civil(year, month, day) * MICROS_PER_DAY
            + ((hour * 60 + minute) * 60 + second) * MICROS_PER_SECOND
            + micros;
        Ok(Self {
            utc_micros: local_micros - i64::from(offset_minutes) * MICROS_PER_MINUTE,
            offset_minutes,
        })
    }

    /// Now, in UTC, at a `datetime`'s microsecond resolution.
    ///
    /// The stores take their clock as an argument (the M36 plan's call 4), and this is what the
    /// verbs pass. The sub-microsecond part is dropped, which is CPython's `datetime.now` too: it
    /// floors, and for an instant after 1970 a floor is a truncation.
    pub fn now_utc() -> Self {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock reads after 1970");
        Self {
            utc_micros: i64::try_from(since.as_micros()).expect("a clock before the year 294,000"),
            offset_minutes: 0,
        }
    }

    /// The instant, in microseconds since 1970-01-01T00:00:00Z — the whole of the order.
    pub fn utc_micros(&self) -> i64 {
        self.utc_micros
    }

    /// The offset it was read in, in minutes east of UTC.
    pub fn offset_minutes(&self) -> i16 {
        self.offset_minutes
    }

    /// CPython's `datetime.isoformat()`: pydantic's spelling with `+00:00` where pydantic writes
    /// `Z`.
    ///
    /// Its own function because `_arrival` sorts on this **string**, not on the instant, so two
    /// manifests at one instant in different offsets order by how their offsets spell, and a whole
    /// second sorts before `.500000` because `+` is below `.`. The M36 plan's P1 finding 5 has the
    /// cases, and the storage family records them.
    pub fn isoformat(&self) -> String {
        let mut out = String::with_capacity(32);
        self.write(&mut out, "+00:00")
            .expect("writing to a String cannot fail");
        out
    }

    /// `f"{dt:%Y-%m-%d}"` — the date in the value's **own** offset, not in UTC.
    ///
    /// So `2026-08-20T23:30:00-05:00` is the 20th, where UTC says the 21st, which is the date the
    /// bag profile prints. The year is four digits, zero-padded, which is what CPython writes on
    /// this platform; below year 1000 `%Y` is the C library's to pad, so the table records no date
    /// there, and no instant on disk is that old.
    pub fn date_ymd(&self) -> String {
        let (days, _) = self.local_day_and_micros();
        let (year, month, day) = civil_from_days(days);
        format!("{year:04}-{month:02}-{day:02}")
    }

    /// The local date as days since the epoch, and the microseconds into that day.
    fn local_day_and_micros(&self) -> (i64, i64) {
        let local = self.utc_micros + i64::from(self.offset_minutes) * MICROS_PER_MINUTE;
        (
            local.div_euclid(MICROS_PER_DAY),
            local.rem_euclid(MICROS_PER_DAY),
        )
    }

    /// The shared writer: everything but how a zero offset is spelled.
    fn write(&self, out: &mut impl fmt::Write, zero_offset: &str) -> fmt::Result {
        let (days, in_day) = self.local_day_and_micros();
        let (year, month, day) = civil_from_days(days);
        let seconds = in_day / MICROS_PER_SECOND;
        let micros = in_day % MICROS_PER_SECOND;
        write!(
            out,
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )?;
        if micros != 0 {
            write!(out, ".{micros:06}")?;
        }
        if self.offset_minutes == 0 {
            return out.write_str(zero_offset);
        }
        let sign = if self.offset_minutes < 0 { '-' } else { '+' };
        let magnitude = self.offset_minutes.unsigned_abs();
        write!(out, "{sign}{:02}:{:02}", magnitude / 60, magnitude % 60)
    }
}

/// pydantic's spelling — what `model_dump_json` writes.
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, "Z")
    }
}

/// As the lexeme, because that is what a test failure needs to show.
impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timestamp(\"{self}\")")
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.utc_micros == other.utc_micros
    }
}

impl Eq for Timestamp {}

impl Hash for Timestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.utc_micros.hash(state);
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.utc_micros.cmp(&other.utc_micros)
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// No bounds beyond the grammar, which the parse has already held it to.
impl Validate for Timestamp {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// A run of ASCII digits as a number, or `None` for anything else — `str::parse` would take a
/// leading `+`, which pydantic does not (`2026-08-+4`).
fn digits(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0i64, |acc, b| acc * 10 + i64::from(b - b'0')),
    )
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date — Hinnant's `days_from_civil`.
///
/// The year is counted from March, so a leap day is the last day of its year and every era of
/// 400 years is the same 146,097 days; that is what makes this exact without a table.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - EPOCH_SHIFT_DAYS
}

/// The inverse — Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + EPOCH_SHIFT_DAYS;
    let era = days.div_euclid(DAYS_PER_ERA);
    let day_of_era = days - era * DAYS_PER_ERA;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The calendar arithmetic against itself over every day a `datetime` can hold, so a slip in
    /// one constant shows up as the day it breaks rather than as a wrong spelling in the table.
    #[test]
    fn the_civil_calendar_round_trips_every_day_from_year_1_to_9999() {
        let first = days_from_civil(1, 1, 1);
        let last = days_from_civil(9999, 12, 31);
        let mut expected = (1, 1, 1);
        for days in first..=last {
            assert_eq!(civil_from_days(days), expected, "day {days}");
            assert_eq!(
                days_from_civil(expected.0, expected.1, expected.2),
                days,
                "{expected:?}"
            );
            let (year, month, day) = expected;
            expected = if day < days_in_month(year, month) {
                (year, month, day + 1)
            } else if month < 12 {
                (year, month + 1, 1)
            } else {
                (year + 1, 1, 1)
            };
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }
}
