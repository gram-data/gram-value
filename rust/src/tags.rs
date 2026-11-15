//! Interpretation of the eleven known tags (spec/tags.md).
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

/// The floating-point element encoding of an [`Embedding`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddingDtype {
    /// IEEE 754 binary16 (2 bytes per element).
    F16,
    /// Brain floating-point 16-bit (2 bytes per element).
    BF16,
    /// IEEE 754 binary32 (4 bytes per element).
    F32,
    /// IEEE 754 binary64 (8 bytes per element).
    F64,
}

impl EmbeddingDtype {
    /// Element width in bytes.
    pub const fn width(self) -> usize {
        match self {
            EmbeddingDtype::F16 | EmbeddingDtype::BF16 => 2,
            EmbeddingDtype::F32 => 4,
            EmbeddingDtype::F64 => 8,
        }
    }

    /// Exact SafeTensors dtype string.
    pub const fn as_str(self) -> &'static str {
        match self {
            EmbeddingDtype::F16 => "F16",
            EmbeddingDtype::BF16 => "BF16",
            EmbeddingDtype::F32 => "F32",
            EmbeddingDtype::F64 => "F64",
        }
    }
}

/// Maximum embedding dimension (`2^20`, ADR-008 2.4).
pub const MAX_EMBEDDING_DIMENSION: usize = 1_048_576;

/// A rank-one dense floating-point tensor carried by an `embedding` tag (ADR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embedding {
    dtype: EmbeddingDtype,
    dimension: usize,
    bytes: Vec<u8>,
}

impl Embedding {
    /// Construct an embedding from a dtype, positive dimension (`<= 1,048,576`), and exact little-endian payload bytes.
    pub fn new(
        dtype: EmbeddingDtype,
        dimension: usize,
        bytes: Vec<u8>,
    ) -> Result<Self, Diagnostic> {
        if dimension == 0 || dimension > MAX_EMBEDDING_DIMENSION {
            return Err(Diagnostic::new(
                "out-of-range",
                format!(
                    "embedding: dimension {dimension} must be between 1 and {MAX_EMBEDDING_DIMENSION}"
                ),
            ));
        }
        let expected = dimension * dtype.width();
        if bytes.len() != expected {
            return Err(Diagnostic::new(
                "invalid-form",
                format!(
                    "embedding: expected {expected} payload bytes for {dimension} {:?} elements, got {}",
                    dtype,
                    bytes.len()
                ),
            ));
        }
        Ok(Embedding {
            dtype,
            dimension,
            bytes,
        })
    }

    /// The element encoding.
    pub fn dtype(&self) -> EmbeddingDtype {
        self.dtype
    }

    /// The number of elements (`1 ..= 1,048,576`).
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// The raw little-endian payload bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the embedding and return its raw payload bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// The canonical Base64-encoded SafeTensors content string (ADR-008 2.2).
    pub fn to_normal_content(&self) -> String {
        encode_embedding_normal(self.dtype, self.dimension, &self.bytes)
    }

    /// Convert to a `Value::Tagged("embedding", ...)` using the canonical normal content string.
    pub fn to_value(&self) -> Value {
        Value::tagged("embedding", self.to_normal_content()).expect("embedding is a valid symbol")
    }
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
    /// `embedding`.
    Embedding(Embedding),
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

/// The eleven known tags.
pub const KNOWN_TAGS: [&str; 11] = [
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
    "embedding",
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
        "weekDate" => (
            Typed::Week(parse_week(content, true).map_err(fail)?),
            content.to_string(),
        ),
        _ => {
            let (emb, n) = parse_embedding(content).map_err(fail)?;
            (Typed::Embedding(emb), n)
        }
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

// ---- embedding (ADR-008) --------------------------------------------------------------------

const MAX_BASE64_CHARS: usize = 11_190_284;
const MAX_DECODED_BYTES: usize = 8_392_712;
const MAX_HEADER_BYTES: usize = 4_096;

fn b64_val(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_strict_base64(content: &str) -> Result<Vec<u8>, Fault> {
    let raw = content.as_bytes();
    // 1. Base64 syntax (invalid-form)
    if raw.is_empty() {
        return Err(form("empty Base64 content"));
    }
    if raw.len() % 4 != 0 {
        return Err(form("Base64 length must be a multiple of 4"));
    }
    let pad_count = if raw.ends_with(b"==") {
        2
    } else if raw.ends_with(b"=") {
        1
    } else {
        0
    };
    let data_end = raw.len() - pad_count;
    for (i, &b) in raw[..data_end].iter().enumerate() {
        if b64_val(b).is_none() {
            return Err(form(format!("invalid Base64 byte at index {i}")));
        }
    }
    if pad_count == 1 {
        let v3 = b64_val(raw[raw.len() - 2]).expect("checked above");
        if (v3 & 0x03) != 0 {
            return Err(form("non-canonical Base64 pad bits"));
        }
    } else if pad_count == 2 {
        let v2 = b64_val(raw[raw.len() - 3]).expect("checked above");
        if (v2 & 0x0F) != 0 {
            return Err(form("non-canonical Base64 pad bits"));
        }
    }

    // 2. Decoded-size bound (out-of-range before allocation)
    let decoded_len = (raw.len() / 4) * 3 - pad_count;
    if raw.len() > MAX_BASE64_CHARS || decoded_len > MAX_DECODED_BYTES {
        return Err(range(format!(
            "embedding exceeds maximum decoded size of {MAX_DECODED_BYTES} bytes"
        )));
    }

    let mut out = Vec::with_capacity(decoded_len);
    let mut i = 0;
    while i < raw.len() {
        let a = b64_val(raw[i]).expect("checked");
        let b = b64_val(raw[i + 1]).expect("checked");
        out.push((a << 2) | (b >> 4));
        if raw[i + 2] != b'=' {
            let c = b64_val(raw[i + 2]).expect("checked");
            out.push((b << 4) | (c >> 2));
            if raw[i + 3] != b'=' {
                let d = b64_val(raw[i + 3]).expect("checked");
                out.push((c << 6) | d);
            }
        }
        i += 4;
    }
    Ok(out)
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn encode_embedding_normal(dtype: EmbeddingDtype, dimension: usize, payload: &[u8]) -> String {
    let json = format!(
        "{{\"embedding\":{{\"dtype\":\"{}\",\"shape\":[{dimension}],\"data_offsets\":[0,{}]}}}}",
        dtype.as_str(),
        payload.len()
    );
    let pad = (8 - (json.len() % 8)) % 8;
    let header_len = json.len() + pad;
    let mut buf = Vec::with_capacity(8 + header_len + payload.len());
    buf.extend_from_slice(&(header_len as u64).to_le_bytes());
    buf.extend_from_slice(json.as_bytes());
    buf.resize(8 + header_len, b' ');
    buf.extend_from_slice(payload);
    encode_base64(&buf)
}

enum ShapeDim {
    Valid(u64),
    OverflowU64,
}

struct HeaderParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> HeaderParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.pos += 1;
        }
    }

    fn expect_byte(&mut self, want: u8) -> Result<(), Fault> {
        self.skip_ws();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == want {
            self.pos += 1;
            Ok(())
        } else {
            Err(form(format!("expected '{}' in JSON header", want as char)))
        }
    }

    fn parse_string(&mut self) -> Result<String, Fault> {
        self.skip_ws();
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'"' {
            return Err(form("expected JSON string"));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b < 0x20 {
                return Err(form("unescaped control character in JSON string"));
            }
            if b == b'\\' {
                self.pos += 1;
                if self.pos >= self.bytes.len() {
                    return Err(form("unterminated JSON escape"));
                }
                let esc = self.bytes[self.pos];
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let code = self.parse_hex4()?;
                        if (0xD800..=0xDBFF).contains(&code) {
                            if self.pos + 2 > self.bytes.len()
                                || self.bytes[self.pos] != b'\\'
                                || self.bytes[self.pos + 1] != b'u'
                            {
                                return Err(form("unpaired UTF-16 high surrogate in JSON string"));
                            }
                            self.pos += 2;
                            let low = self.parse_hex4()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(form("invalid UTF-16 low surrogate in JSON string"));
                            }
                            let cp = 0x10000
                                + (((u32::from(code) - 0xD800) << 10) | (u32::from(low) - 0xDC00));
                            let ch =
                                char::from_u32(cp).ok_or_else(|| form("invalid Unicode scalar"))?;
                            out.push(ch);
                        } else if (0xDC00..=0xDFFF).contains(&code) {
                            return Err(form("unpaired UTF-16 low surrogate in JSON string"));
                        } else {
                            let ch = char::from_u32(u32::from(code))
                                .ok_or_else(|| form("invalid Unicode scalar"))?;
                            out.push(ch);
                        }
                    }
                    _ => return Err(form("invalid JSON escape sequence")),
                }
            } else {
                let s = std::str::from_utf8(&self.bytes[self.pos..])
                    .map_err(|_| form("invalid UTF-8 in JSON string"))?;
                let ch = s
                    .chars()
                    .next()
                    .ok_or_else(|| form("unexpected end of JSON string"))?;
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        Err(form("unterminated JSON string"))
    }

    fn parse_hex4(&mut self) -> Result<u16, Fault> {
        if self.pos + 4 > self.bytes.len() {
            return Err(form("short \\uXXXX escape in JSON string"));
        }
        let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| form("invalid \\uXXXX escape"))?;
        let val =
            u16::from_str_radix(hex, 16).map_err(|_| form("invalid hex in \\uXXXX escape"))?;
        self.pos += 4;
        Ok(val)
    }

    fn parse_plain_uint_token(&mut self) -> Result<&'a str, Fault> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(form("expected plain base-10 integer"));
        }
        let len = self.pos - start;
        if len > 1 && self.bytes[start] == b'0' {
            return Err(form("leading zero in JSON integer"));
        }
        if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'.' | b'e' | b'E') {
            return Err(form("non-integer JSON number"));
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.pos]).expect("ASCII digits"))
    }

    fn parse_shape(&mut self) -> Result<ShapeDim, Fault> {
        self.expect_byte(b'[')?;
        let tok = self.parse_plain_uint_token()?;
        self.expect_byte(b']')?;
        Ok(match tok.parse::<u64>() {
            Ok(n) => ShapeDim::Valid(n),
            Err(_) => ShapeDim::OverflowU64,
        })
    }

    fn parse_data_offsets(&mut self) -> Result<(u64, u64), Fault> {
        self.expect_byte(b'[')?;
        let t0 = self.parse_plain_uint_token()?;
        self.expect_byte(b',')?;
        let t1 = self.parse_plain_uint_token()?;
        self.expect_byte(b']')?;
        let a = t0
            .parse::<u64>()
            .map_err(|_| form("data_offsets exceeds 64 unsigned bits"))?;
        let b = t1
            .parse::<u64>()
            .map_err(|_| form("data_offsets exceeds 64 unsigned bits"))?;
        Ok((a, b))
    }

    fn parse_header(&mut self) -> Result<(String, ShapeDim, (u64, u64)), Fault> {
        self.expect_byte(b'{')?;
        let top_key = self.parse_string()?;
        if top_key != "embedding" {
            return Err(form(format!("unexpected top-level key {top_key:?}")));
        }
        self.expect_byte(b':')?;
        self.expect_byte(b'{')?;

        let mut dtype: Option<String> = None;
        let mut shape: Option<ShapeDim> = None;
        let mut offsets: Option<(u64, u64)> = None;

        for idx in 0..3 {
            if idx > 0 {
                self.expect_byte(b',')?;
            }
            let key = self.parse_string()?;
            self.expect_byte(b':')?;
            match key.as_str() {
                "dtype" => {
                    if dtype.is_some() {
                        return Err(form("duplicate key \"dtype\""));
                    }
                    dtype = Some(self.parse_string()?);
                }
                "shape" => {
                    if shape.is_some() {
                        return Err(form("duplicate key \"shape\""));
                    }
                    shape = Some(self.parse_shape()?);
                }
                "data_offsets" => {
                    if offsets.is_some() {
                        return Err(form("duplicate key \"data_offsets\""));
                    }
                    offsets = Some(self.parse_data_offsets()?);
                }
                _ => return Err(form(format!("unexpected tensor key {key:?}"))),
            }
        }
        self.expect_byte(b'}')?;
        self.expect_byte(b'}')?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return Err(form("trailing bytes in SafeTensors JSON header"));
        }
        Ok((
            dtype.expect("3 distinct keys checked"),
            shape.expect("3 distinct keys checked"),
            offsets.expect("3 distinct keys checked"),
        ))
    }
}

fn parse_embedding(content: &str) -> Result<(Embedding, String), Fault> {
    // Steps 1 & 2: Base64 syntax + decoded-size bound before allocation.
    let decoded = decode_strict_base64(content)?;

    // Step 3: SafeTensors container & JSON profile (invalid-form).
    if decoded.len() < 8 {
        return Err(form("SafeTensors buffer shorter than 8 bytes"));
    }
    let header_len_u64 = u64::from_le_bytes(decoded[0..8].try_into().expect("8 bytes"));
    let remaining = (decoded.len() - 8) as u64;
    if header_len_u64 > MAX_HEADER_BYTES as u64 || header_len_u64 > remaining {
        return Err(form("SafeTensors header length exceeds 4,096 or buffer"));
    }
    let header_len = header_len_u64 as usize;
    let header_str = std::str::from_utf8(&decoded[8..8 + header_len])
        .map_err(|_| form("SafeTensors header is not valid UTF-8"))?;
    let (dtype_str, shape_dim, (off0, off1)) = HeaderParser::new(header_str).parse_header()?;

    // Step 4: dtype (unsupported).
    let dtype = match dtype_str.as_str() {
        "F16" => EmbeddingDtype::F16,
        "BF16" => EmbeddingDtype::BF16,
        "F32" => EmbeddingDtype::F32,
        "F64" => EmbeddingDtype::F64,
        _ => {
            return Err(Fault {
                code: "unsupported",
                message: format!("unsupported embedding dtype {dtype_str:?}"),
            })
        }
    };

    // Step 5: dimension range (out-of-range).
    let dim_u64 = match shape_dim {
        ShapeDim::OverflowU64 => return Err(range("embedding dimension exceeds 64 unsigned bits")),
        ShapeDim::Valid(n) => n,
    };
    if dim_u64 == 0 || dim_u64 > MAX_EMBEDDING_DIMENSION as u64 {
        return Err(range(format!(
            "embedding dimension {dim_u64} must be between 1 and {MAX_EMBEDDING_DIMENSION}"
        )));
    }
    let dimension = dim_u64 as usize;

    // Step 6: stated and actual byte lengths (invalid-form).
    let expected_bytes = dimension * dtype.width();
    if off0 != 0 || off1 != expected_bytes as u64 {
        return Err(form(format!(
            "data_offsets [{off0}, {off1}] does not match expected [0, {expected_bytes}]"
        )));
    }
    let payload_start = 8 + header_len;
    if decoded.len() != payload_start + expected_bytes {
        return Err(form(format!(
            "payload length {} does not match expected {expected_bytes}",
            decoded.len() - payload_start
        )));
    }

    let payload = decoded[payload_start..].to_vec();
    let normal = encode_embedding_normal(dtype, dimension, &payload);
    let emb = Embedding {
        dtype,
        dimension,
        bytes: payload,
    };
    Ok((emb, normal))
}
