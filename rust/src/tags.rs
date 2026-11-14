//! Interpretation of the ten known tags (spec/tags.md).
//!
//! Interpretation checks a tagged string's content form and returns a typed value and a normal
//! content string. It never changes the tagged value.

use crate::diagnostic::Diagnostic;
use crate::value::Value;

/// A calendar date in the proleptic Gregorian calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Date {
    /// The year, possibly negative.
    pub year: i64,
    /// 1 to 12.
    pub month: u8,
    /// 1 to the month's length.
    pub day: u8,
}

/// An offset from UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offset {
    /// Minutes east of UTC, -1439 to 1439.
    pub minutes: i16,
}

/// A time of day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Time {
    /// 0 to 23.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
    /// 0 to 59; 0 when absent in the text.
    pub second: u8,
    /// 0 to 999,999,999.
    pub nanosecond: u32,
    /// The offset, when written.
    pub offset: Option<Offset>,
}

/// A date and time, with an optional zone name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTime {
    /// The date part.
    pub date: Date,
    /// The time part.
    pub time: Time,
    /// The zone name in square brackets, when written.
    pub zone: Option<String>,
}

/// Seconds with a sign, as a duration writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seconds {
    /// True when written with a leading `-`.
    pub negative: bool,
    /// Whole seconds.
    pub whole: u64,
    /// Nanoseconds of the fraction.
    pub nanos: u32,
}

/// An ISO 8601 duration with per-number signs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Duration {
    /// Years, when written.
    pub years: Option<i64>,
    /// Months, when written.
    pub months: Option<i64>,
    /// Days, when written.
    pub days: Option<i64>,
    /// Hours, when written.
    pub hours: Option<i64>,
    /// Minutes, when written.
    pub minutes: Option<i64>,
    /// Seconds, when written.
    pub seconds: Option<Seconds>,
}

/// A point of two or three coordinates and an optional SRID.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// The SRID, when written.
    pub srid: Option<u64>,
    /// The first coordinate.
    pub x: f64,
    /// The second coordinate.
    pub y: f64,
    /// The third coordinate, when written.
    pub z: Option<f64>,
}

/// An ISO week of a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Week {
    /// The four-digit year.
    pub year: u16,
    /// 1 to 52, or 53 in a year that has 53 weeks.
    pub week: u8,
    /// 1 (Monday) to 7, for `weekDate`.
    pub day: Option<u8>,
}

/// What a known tag's content means.
#[derive(Debug, Clone, PartialEq)]
pub enum Typed {
    /// `date`.
    Date(Date),
    /// `time` (offset optional) or `localtime` (no offset).
    Time(Time),
    /// `datetime` (offset optional) or `localdatetime` (no offset).
    DateTime(DateTime),
    /// `duration`.
    Duration(Duration),
    /// `point`.
    Point(Point),
    /// `uuid`: the 16 bytes.
    Uuid([u8; 16]),
    /// `week` and `weekDate`.
    Week(Week),
}

/// A successful interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct Interpretation {
    /// The tag that was interpreted.
    pub tag: &'static str,
    /// The typed value.
    pub typed: Typed,
    /// The normal content string (spec/tags.md section 4).
    pub normal_content: String,
}

/// The ten known tags.
pub const KNOWN_TAGS: [&str; 10] = [
    "date",
    "time",
    "localtime",
    "datetime",
    "localdatetime",
    "duration",
    "point",
    "uuid",
    "week",
    "weekDate",
];

/// Interpret a tagged value. Returns `Ok(None)` for a value that is not tagged or whose tag is unknown.
pub fn interpret(value: &Value) -> Result<Option<Interpretation>, Diagnostic> {
    match value {
        Value::Tagged(t) => interpret_tag(t.tag(), t.content()),
        _ => Ok(None),
    }
}

/// Interpret a tag and content. Returns `Ok(None)` for an unknown tag.
pub fn interpret_tag(tag: &str, content: &str) -> Result<Option<Interpretation>, Diagnostic> {
    let known = KNOWN_TAGS.iter().find(|t| **t == tag).copied();
    let Some(name) = known else { return Ok(None) };
    let fail = |e: Fault| Diagnostic::new(e.code, format!("{name}: {}", e.message));
    let (typed, normal) = match name {
        "date" => {
            let d = parse_date_full(content).map_err(fail)?;
            (Typed::Date(d), content.to_string())
        }
        "time" => {
            let t = parse_time(content, true).map_err(fail)?;
            let n = time_normal(&t);
            (Typed::Time(t), n)
        }
        "localtime" => {
            let t = parse_time(content, false).map_err(fail)?;
            let n = time_normal(&t);
            (Typed::Time(t), n)
        }
        "datetime" => {
            let (dt, n) = parse_datetime(content, true).map_err(fail)?;
            (Typed::DateTime(dt), n)
        }
        "localdatetime" => {
            let (dt, n) = parse_datetime(content, false).map_err(fail)?;
            (Typed::DateTime(dt), n)
        }
        "duration" => (
            Typed::Duration(parse_duration(content).map_err(fail)?),
            content.to_string(),
        ),
        "point" => {
            let (p, n) = parse_point(content).map_err(fail)?;
            (Typed::Point(p), n)
        }
        "uuid" => {
            let bytes = parse_uuid(content).map_err(fail)?;
            (Typed::Uuid(bytes), content.to_ascii_lowercase())
        }
        "week" => (
            Typed::Week(parse_week(content, false).map_err(fail)?),
            content.to_string(),
        ),
        _ => (
            Typed::Week(parse_week(content, true).map_err(fail)?),
            content.to_string(),
        ),
    };
    Ok(Some(Interpretation {
        tag: name,
        typed,
        normal_content: normal,
    }))
}

struct Fault {
    code: &'static str,
    message: String,
}

fn form(message: impl Into<String>) -> Fault {
    Fault {
        code: "invalid-form",
        message: message.into(),
    }
}
fn range(message: impl Into<String>) -> Fault {
    Fault {
        code: "out-of-range",
        message: message.into(),
    }
}

fn two_digits(s: &str, what: &str) -> Result<u8, Fault> {
    let b = s.as_bytes();
    if b.len() == 2 && b.iter().all(u8::is_ascii_digit) {
        Ok((b[0] - b'0') * 10 + (b[1] - b'0'))
    } else {
        Err(form(format!("{what} must be two digits")))
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

/// Parse `-?YYYY+-MM-DD` from the start of `s`; returns the date and the length consumed.
fn parse_date_prefix(s: &str) -> Result<(Date, usize), Fault> {
    let digits_start = usize::from(s.starts_with('-'));
    let rest = &s[digits_start..];
    let year_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if year_len < 4 {
        return Err(form("the year needs at least four digits"));
    }
    let year_text = &s[..digits_start + year_len];
    let year: i64 = year_text
        .parse()
        .map_err(|_| range("the year is too large"))?;
    let tail = &s[digits_start + year_len..];
    let b = tail.as_bytes();
    if b.len() < 6 || b[0] != b'-' || b[3] != b'-' {
        return Err(form("expected YYYY-MM-DD"));
    }
    let month = two_digits(&tail[1..3], "the month")?;
    let day = two_digits(&tail[4..6], "the day")?;
    if !(1..=12).contains(&month) {
        return Err(range(format!("month {month} does not exist")));
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(range(format!(
            "day {day} does not exist in month {month} of {year}"
        )));
    }
    Ok((Date { year, month, day }, digits_start + year_len + 6))
}

fn parse_date_full(s: &str) -> Result<Date, Fault> {
    let (d, n) = parse_date_prefix(s)?;
    if n != s.len() {
        return Err(form("unexpected text after the date"));
    }
    Ok(d)
}

fn parse_offset(s: &str) -> Result<Offset, Fault> {
    if s == "Z" {
        return Ok(Offset { minutes: 0 });
    }
    let b = s.as_bytes();
    if b.len() != 6 || !(b[0] == b'+' || b[0] == b'-') || b[3] != b':' {
        return Err(form("an offset is Z, +hh:mm or -hh:mm"));
    }
    let h = two_digits(&s[1..3], "the offset hours")?;
    let m = two_digits(&s[4..6], "the offset minutes")?;
    if h > 23 || m > 59 {
        return Err(range("the offset is out of range"));
    }
    let total = i16::from(h) * 60 + i16::from(m);
    Ok(Offset {
        minutes: if b[0] == b'-' { -total } else { total },
    })
}

fn parse_time(s: &str, offset_allowed: bool) -> Result<Time, Fault> {
    // The time proper ends at an offset: Z, + or -.
    let split = s.find(['Z', '+', '-']).unwrap_or(s.len());
    let (clock, off) = s.split_at(split);
    let offset = if off.is_empty() {
        None
    } else if offset_allowed {
        Some(parse_offset(off)?)
    } else {
        return Err(form("this tag does not allow an offset"));
    };
    let b = clock.as_bytes();
    if b.len() < 5 || b[2] != b':' {
        return Err(form("expected hh:mm"));
    }
    let hour = two_digits(&clock[0..2], "the hour")?;
    let minute = two_digits(&clock[3..5], "the minute")?;
    let (mut second, mut nanosecond) = (0u8, 0u32);
    match clock.len() {
        5 => {}
        _ if b[5] == b':' && clock.len() >= 8 => {
            second = two_digits(&clock[6..8], "the second")?;
            if clock.len() > 8 {
                let frac = clock[8..]
                    .strip_prefix('.')
                    .ok_or_else(|| form("expected a fraction after the seconds"))?;
                if frac.is_empty() || frac.len() > 9 || !frac.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(form("a fraction has 1 to 9 digits"));
                }
                nanosecond = format!("{frac:0<9}").parse().expect("nine digits");
            }
        }
        _ => return Err(form("expected hh:mm or hh:mm:ss")),
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(range("the time is out of range"));
    }
    Ok(Time {
        hour,
        minute,
        second,
        nanosecond,
        offset,
    })
}

fn offset_text(o: Offset) -> String {
    if o.minutes == 0 {
        "Z".to_string()
    } else {
        let a = o.minutes.abs();
        format!(
            "{}{:02}:{:02}",
            if o.minutes < 0 { '-' } else { '+' },
            a / 60,
            a % 60
        )
    }
}

fn time_normal(t: &Time) -> String {
    let mut s = format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second);
    if t.nanosecond != 0 {
        let frac = format!("{:09}", t.nanosecond);
        s.push('.');
        s.push_str(frac.trim_end_matches('0'));
    }
    if let Some(o) = t.offset {
        s.push_str(&offset_text(o));
    }
    s
}

fn parse_datetime(s: &str, offset_allowed: bool) -> Result<(DateTime, String), Fault> {
    let (date, used) = parse_date_prefix(s)?;
    let rest = s[used..]
        .strip_prefix('T')
        .ok_or_else(|| form("expected T after the date"))?;
    let (clock, zone) = match rest.find('[') {
        Some(i) => {
            let z = rest[i..]
                .strip_prefix('[')
                .and_then(|z| z.strip_suffix(']'))
                .ok_or_else(|| form("a zone name is written [name]"))?;
            (&rest[..i], Some(z))
        }
        None => (rest, None),
    };
    let time = parse_time(clock, offset_allowed)?;
    if let Some(z) = zone {
        if time.offset.is_none() {
            return Err(form("a zone name needs an offset"));
        }
        if z.is_empty()
            || !z
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-'))
        {
            return Err(form("a zone name is letters, digits and / _ + -"));
        }
    }
    let normal = format!(
        "{}T{}{}",
        &s[..used],
        time_normal(&time),
        zone.map(|z| format!("[{z}]")).unwrap_or_default()
    );
    Ok((
        DateTime {
            date,
            time,
            zone: zone.map(str::to_string),
        },
        normal,
    ))
}

fn parse_signed(num: &str) -> Result<i64, Fault> {
    let digits = num.strip_prefix('-').unwrap_or(num);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err(form("a duration component is a number"));
    }
    num.parse()
        .map_err(|_| range("a duration component is too large"))
}

fn parse_duration(s: &str) -> Result<Duration, Fault> {
    let mut rest = s
        .strip_prefix('P')
        .ok_or_else(|| form("a duration starts with P"))?;
    let mut d = Duration::default();
    let mut in_time = false;
    let mut order_date = 0; // Y=1 M=2 D=3
    let mut order_time = 0; // H=1 M=2 S=3
    let mut any = false;
    let mut time_components = 0;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('T') {
            if in_time {
                return Err(form("T appears once"));
            }
            in_time = true;
            rest = r;
            if rest.is_empty() {
                return Err(form("T needs at least one component"));
            }
            continue;
        }
        let end = rest
            .find(|c: char| c.is_ascii_uppercase())
            .ok_or_else(|| form("a component needs a designator"))?;
        let (num, tail) = rest.split_at(end);
        let designator = tail.as_bytes()[0];
        rest = &tail[1..];
        any = true;
        if in_time {
            time_components += 1;
        }
        match (in_time, designator) {
            (false, b'Y') if order_date < 1 => {
                d.years = Some(parse_signed(num)?);
                order_date = 1;
            }
            (false, b'M') if order_date < 2 => {
                d.months = Some(parse_signed(num)?);
                order_date = 2;
            }
            (false, b'D') if order_date < 3 => {
                d.days = Some(parse_signed(num)?);
                order_date = 3;
            }
            (true, b'H') if order_time < 1 => {
                d.hours = Some(parse_signed(num)?);
                order_time = 1;
            }
            (true, b'M') if order_time < 2 => {
                d.minutes = Some(parse_signed(num)?);
                order_time = 2;
            }
            (true, b'S') if order_time < 3 => {
                let negative = num.starts_with('-');
                let body = num.strip_prefix('-').unwrap_or(num);
                let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
                if whole.is_empty()
                    || !whole.bytes().all(|c| c.is_ascii_digit())
                    || (body.contains('.') && (frac.is_empty() || frac.len() > 9))
                    || !frac.bytes().all(|c| c.is_ascii_digit())
                {
                    return Err(form(
                        "seconds are digits with an optional fraction of 1 to 9 digits",
                    ));
                }
                d.seconds = Some(Seconds {
                    negative,
                    whole: whole
                        .parse()
                        .map_err(|_| range("the seconds are too large"))?,
                    nanos: if frac.is_empty() {
                        0
                    } else {
                        format!("{frac:0<9}").parse().expect("nine digits")
                    },
                });
                order_time = 3;
            }
            _ => return Err(form("components are in the order Y M D, then T, H M S")),
        }
    }
    if !any {
        return Err(form("a duration has at least one component"));
    }
    if in_time && time_components == 0 {
        return Err(form("T needs at least one component"));
    }
    Ok(d)
}

fn is_number(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or(s);
    let (int, frac) = body.split_once('.').unwrap_or((body, "0"));
    !int.is_empty()
        && int.bytes().all(|c| c.is_ascii_digit())
        && !frac.is_empty()
        && frac.bytes().all(|c| c.is_ascii_digit())
        && !(body.ends_with('.'))
}

fn number_text(f: f64) -> String {
    if f == 0.0 {
        "0".to_string()
    } else {
        format!("{f}")
    }
}

fn parse_point(s: &str) -> Result<(Point, String), Fault> {
    let (srid, coords) = match s.strip_prefix("SRID=") {
        Some(rest) => {
            let (n, c) = rest
                .split_once(';')
                .ok_or_else(|| form("SRID=n; needs the semicolon"))?;
            if n.is_empty() || !n.bytes().all(|c| c.is_ascii_digit()) {
                return Err(form("the SRID is a positive integer"));
            }
            let n: u64 = n.parse().map_err(|_| range("the SRID is too large"))?;
            if n == 0 {
                return Err(form("the SRID is a positive integer"));
            }
            (Some(n), c)
        }
        None => (None, s),
    };
    let parts: Vec<&str> = coords.split(' ').collect();
    if !(parts.len() == 2 || parts.len() == 3) || !parts.iter().all(|p| is_number(p)) {
        return Err(form(
            "a point is two or three numbers separated by single spaces",
        ));
    }
    let nums: Vec<f64> = parts
        .iter()
        .map(|p| p.parse().expect("checked number"))
        .collect();
    if nums.iter().any(|n| !n.is_finite()) {
        return Err(range("a coordinate is out of range"));
    }
    let mut normal = String::new();
    if let Some(n) = srid {
        normal.push_str(&format!("SRID={n};"));
    }
    normal.push_str(
        &nums
            .iter()
            .map(|n| number_text(*n))
            .collect::<Vec<_>>()
            .join(" "),
    );
    Ok((
        Point {
            srid,
            x: nums[0],
            y: nums[1],
            z: nums.get(2).copied(),
        },
        normal,
    ))
}

fn parse_uuid(s: &str) -> Result<[u8; 16], Fault> {
    let b = s.as_bytes();
    if b.len() != 36 || [8, 13, 18, 23].iter().any(|&i| b[i] != b'-') {
        return Err(form("a UUID is 8-4-4-4-12 hexadecimal digits"));
    }
    let hex: Vec<u8> = b.iter().copied().filter(|&c| c != b'-').collect();
    if hex.len() != 32 || !hex.iter().all(u8::is_ascii_hexdigit) {
        return Err(form("a UUID is 8-4-4-4-12 hexadecimal digits"));
    }
    let nibble = |c: u8| (c as char).to_digit(16).expect("hex digit") as u8;
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = nibble(hex[2 * i]) << 4 | nibble(hex[2 * i + 1]);
    }
    Ok(out)
}

/// Day of the week of 1 January, 0 = Monday.
fn jan1_weekday(year: i64) -> i64 {
    // Days from a fixed Monday (0001-01-01 is a Monday in the proleptic Gregorian calendar).
    let y = year - 1;
    (y * 365 + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)).rem_euclid(7)
}

fn has_53_weeks(year: i64) -> bool {
    let w = jan1_weekday(year);
    w == 3 || (is_leap(year) && w == 2)
}

fn parse_week(s: &str, with_day: bool) -> Result<Week, Fault> {
    let b = s.as_bytes();
    let expected = if with_day { 9 } else { 7 };
    if b.len() != expected
        || !b[..4].iter().all(u8::is_ascii_digit)
        || b[4] != b'W'
        || (with_day && b[7] != b'-')
    {
        return Err(form(if with_day {
            "expected YYYYWww-D"
        } else {
            "expected YYYYWww"
        }));
    }
    let year: u16 = s[..4].parse().expect("four digits");
    let week = two_digits(&s[5..7], "the week")?;
    let day = if with_day {
        if !b[8].is_ascii_digit() {
            return Err(form("the weekday is one digit"));
        }
        Some(b[8] - b'0')
    } else {
        None
    };
    let max = if has_53_weeks(i64::from(year)) {
        53
    } else {
        52
    };
    if week == 0 || week > max {
        return Err(range(format!(
            "{year} has {max} ISO weeks, not week {week}"
        )));
    }
    if let Some(d) = day {
        if !(1..=7).contains(&d) {
            return Err(range("the weekday is 1 to 7"));
        }
    }
    Ok(Week { year, week, day })
}
