package gramvalue

import (
	"encoding/hex"
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
)

// KnownTags are the ten tags whose content has a standard form (ADR-004).
var KnownTags = []string{"date", "time", "localtime", "datetime", "localdatetime", "duration", "point", "uuid", "week", "weekDate"}

// Date is a calendar date in the proleptic Gregorian calendar.
type Date struct {
	Year       int64
	Month, Day int
}

// Time is a time of day.
type Time struct {
	Hour, Minute, Second int
	Nanosecond           int
	// OffsetMinutes is the offset east of UTC in minutes, when written.
	OffsetMinutes *int
}

// DateTime is a date and time, with an optional zone name.
type DateTime struct {
	Date Date
	Time Time
	// Zone is the zone name in square brackets, when written.
	Zone *string
}

// Seconds are the seconds of a duration, with their own sign.
type Seconds struct {
	Negative bool
	Whole    uint64
	Nanos    int
}

// Duration is an ISO 8601 duration with per-number signs; a nil component was not written.
type Duration struct {
	Years, Months, Days, Hours, Minutes *int64
	Seconds                             *Seconds
}

// Point is a point of two or three coordinates and an optional SRID.
type Point struct {
	SRID *uint64
	X, Y float64
	Z    *float64
}

// UUID is the 16 bytes of a UUID.
type UUID [16]byte

// Week is an ISO week of a year; Day is set for a weekDate.
type Week struct {
	Year int
	Week int
	Day  *int
}

// Interpretation is a successful interpretation of a known tag.
type Interpretation struct {
	// Tag is the tag that was interpreted.
	Tag string
	// Typed is a Date, Time, DateTime, Duration, Point, UUID or Week.
	Typed any
	// NormalContent is the normal content string (spec/tags.md section 4).
	NormalContent string
}

// ToValue is the tagged value with the normal content string.
func (i Interpretation) ToValue() Value {
	v, err := NewTagged(i.Tag, i.NormalContent)
	if err != nil {
		panic("a known tag is a symbol")
	}
	return v
}

type fault struct{ code, message string }

func form(m string) fault            { return fault{"invalid-form", m} }
func rng(m string) fault             { return fault{"out-of-range", m} }
func throw(f fault)                  { panic(f) }
func formf(m string, a ...any) fault { return form(fmt.Sprintf(m, a...)) }

// Interpret interprets a tagged value. It returns (nil, nil) for a value that is not tagged or whose tag is unknown.
func Interpret(v Value) (*Interpretation, error) {
	t, ok := v.(TaggedValue)
	if !ok {
		return nil, nil
	}
	return InterpretTag(t.tag, t.content)
}

// InterpretTag interprets a tag and content. It returns (nil, nil) for an unknown tag.
func InterpretTag(tag, content string) (result *Interpretation, err error) {
	known := false
	for _, k := range KnownTags {
		known = known || k == tag
	}
	if !known {
		return nil, nil
	}
	defer func() {
		if r := recover(); r != nil {
			f, ok := r.(fault)
			if !ok {
				panic(r)
			}
			result, err = nil, diag(f.code, tag+": "+f.message)
		}
	}()
	return run(tag, content), nil
}

func run(tag, content string) *Interpretation {
	switch tag {
	case "date":
		d, used := parseDatePrefix(content)
		if used != len(content) {
			throw(form("unexpected text after the date"))
		}
		return &Interpretation{tag, d, content}
	case "time", "localtime":
		t := parseTime(content, tag == "time")
		return &Interpretation{tag, t, timeNormal(t)}
	case "datetime", "localdatetime":
		dt, normal := parseDateTime(content, tag == "datetime")
		return &Interpretation{tag, dt, normal}
	case "duration":
		return &Interpretation{tag, parseDuration(content), content}
	case "point":
		p, normal := parsePoint(content)
		return &Interpretation{tag, p, normal}
	case "uuid":
		return &Interpretation{tag, parseUUID(content), strings.ToLower(content)}
	case "week":
		return &Interpretation{tag, parseWeek(content, false), content}
	}
	return &Interpretation{tag, parseWeek(content, true), content}
}

func twoDigits(s, what string) int {
	if len(s) != 2 || s[0] < '0' || s[0] > '9' || s[1] < '0' || s[1] > '9' {
		throw(formf("%s must be two digits", what))
	}
	return int(s[0]-'0')*10 + int(s[1]-'0')
}

func isLeap(y int64) bool { return (y%4 == 0 && y%100 != 0) || y%400 == 0 }

func daysInMonth(year int64, month int) int {
	switch month {
	case 1, 3, 5, 7, 8, 10, 12:
		return 31
	case 4, 6, 9, 11:
		return 30
	}
	if isLeap(year) {
		return 29
	}
	return 28
}

var (
	datePrefixRE = regexp.MustCompile(`^(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})`)
	yearRE       = regexp.MustCompile(`^-?[0-9]{4,}`)
)

func parseDatePrefix(s string) (Date, int) {
	m := datePrefixRE.FindStringSubmatch(s)
	if m == nil {
		if !yearRE.MatchString(s) {
			throw(form("the year needs at least four digits"))
		}
		throw(form("expected YYYY-MM-DD"))
	}
	year, err := strconv.ParseInt(m[1], 10, 64)
	if err != nil {
		throw(rng("the year is too large"))
	}
	month, _ := strconv.Atoi(m[2])
	day, _ := strconv.Atoi(m[3])
	if month < 1 || month > 12 {
		throw(rng(fmt.Sprintf("month %d does not exist", month)))
	}
	if day == 0 || day > daysInMonth(year, month) {
		throw(rng(fmt.Sprintf("day %d does not exist in month %d of %d", day, month, year)))
	}
	return Date{year, month, day}, len(m[0])
}

var offsetRE = regexp.MustCompile(`^([+-])([0-9]{2}):([0-9]{2})$`)

func parseOffset(s string) int {
	if s == "Z" {
		return 0
	}
	m := offsetRE.FindStringSubmatch(s)
	if m == nil {
		throw(form("an offset is Z, +hh:mm or -hh:mm"))
	}
	h, _ := strconv.Atoi(m[2])
	mi, _ := strconv.Atoi(m[3])
	if h > 23 || mi > 59 {
		throw(rng("the offset is out of range"))
	}
	if m[1] == "-" {
		return -(h*60 + mi)
	}
	return h*60 + mi
}

func allDigits(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}

func parseTime(s string, offsetAllowed bool) Time {
	clock, off := s, ""
	if i := strings.IndexAny(s, "Z+-"); i >= 0 {
		clock, off = s[:i], s[i:]
	}
	var offset *int
	if off != "" {
		if !offsetAllowed {
			throw(form("this tag does not allow an offset"))
		}
		o := parseOffset(off)
		offset = &o
	}
	if len(clock) < 5 || clock[2] != ':' {
		throw(form("expected hh:mm"))
	}
	hour := twoDigits(clock[0:2], "the hour")
	minute := twoDigits(clock[3:5], "the minute")
	second, nano := 0, 0
	if len(clock) > 5 {
		if clock[5] != ':' || len(clock) < 8 {
			throw(form("expected hh:mm or hh:mm:ss"))
		}
		second = twoDigits(clock[6:8], "the second")
		if len(clock) > 8 {
			if clock[8] != '.' {
				throw(form("expected a fraction after the seconds"))
			}
			frac := clock[9:]
			if len(frac) < 1 || len(frac) > 9 || !allDigits(frac) {
				throw(form("a fraction has 1 to 9 digits"))
			}
			nano, _ = strconv.Atoi(frac + strings.Repeat("0", 9-len(frac)))
		}
	}
	if hour > 23 || minute > 59 || second > 59 {
		throw(rng("the time is out of range"))
	}
	return Time{hour, minute, second, nano, offset}
}

func offsetText(minutes int) string {
	if minutes == 0 {
		return "Z"
	}
	a, sign := minutes, "+"
	if a < 0 {
		a, sign = -a, "-"
	}
	return fmt.Sprintf("%s%02d:%02d", sign, a/60, a%60)
}

func timeNormal(t Time) string {
	s := fmt.Sprintf("%02d:%02d:%02d", t.Hour, t.Minute, t.Second)
	if t.Nanosecond != 0 {
		s += "." + strings.TrimRight(fmt.Sprintf("%09d", t.Nanosecond), "0")
	}
	if t.OffsetMinutes != nil {
		s += offsetText(*t.OffsetMinutes)
	}
	return s
}

var zoneRE = regexp.MustCompile(`^[A-Za-z0-9/_+-]+$`)

func parseDateTime(s string, offsetAllowed bool) (DateTime, string) {
	date, used := parseDatePrefix(s)
	if used >= len(s) || s[used] != 'T' {
		throw(form("expected T after the date"))
	}
	rest := s[used+1:]
	clock := rest
	var zone *string
	if i := strings.Index(rest, "["); i >= 0 {
		if !strings.HasSuffix(rest, "]") {
			throw(form("a zone name is written [name]"))
		}
		z := rest[i+1 : len(rest)-1]
		clock, zone = rest[:i], &z
	}
	t := parseTime(clock, offsetAllowed)
	normal := s[:used] + "T" + timeNormal(t)
	if zone != nil {
		if t.OffsetMinutes == nil {
			throw(form("a zone name needs an offset"))
		}
		if !zoneRE.MatchString(*zone) {
			throw(form("a zone name is letters, digits and / _ + -"))
		}
		normal += "[" + *zone + "]"
	}
	return DateTime{date, t, zone}, normal
}

var (
	signedRE  = regexp.MustCompile(`^-?[0-9]+$`)
	secondsRE = regexp.MustCompile(`^(-?)([0-9]+)(?:\.([0-9]{1,9}))?$`)
)

func parseSigned(num string) *int64 {
	if !signedRE.MatchString(num) {
		throw(form("a duration component is a number"))
	}
	n, err := strconv.ParseInt(num, 10, 64)
	if err != nil {
		throw(rng("a duration component is too large"))
	}
	return &n
}

func parseDuration(s string) Duration {
	if !strings.HasPrefix(s, "P") {
		throw(form("a duration starts with P"))
	}
	rest := s[1:]
	var d Duration
	inTime, anyComponent := false, false
	orderDate, orderTime, timeComponents := 0, 0, 0
	for rest != "" {
		if rest[0] == 'T' {
			if inTime {
				throw(form("T appears once"))
			}
			inTime = true
			rest = rest[1:]
			if rest == "" {
				throw(form("T needs at least one component"))
			}
			continue
		}
		end := strings.IndexFunc(rest, func(r rune) bool { return r >= 'A' && r <= 'Z' })
		if end < 0 {
			throw(form("a component needs a designator"))
		}
		num, designator := rest[:end], rest[end]
		rest = rest[end+1:]
		anyComponent = true
		if inTime {
			timeComponents++
		}
		switch {
		case !inTime && designator == 'Y' && orderDate < 1:
			d.Years, orderDate = parseSigned(num), 1
		case !inTime && designator == 'M' && orderDate < 2:
			d.Months, orderDate = parseSigned(num), 2
		case !inTime && designator == 'D' && orderDate < 3:
			d.Days, orderDate = parseSigned(num), 3
		case inTime && designator == 'H' && orderTime < 1:
			d.Hours, orderTime = parseSigned(num), 1
		case inTime && designator == 'M' && orderTime < 2:
			d.Minutes, orderTime = parseSigned(num), 2
		case inTime && designator == 'S' && orderTime < 3:
			m := secondsRE.FindStringSubmatch(num)
			if m == nil {
				throw(form("seconds are digits with an optional fraction of 1 to 9 digits"))
			}
			whole, err := strconv.ParseUint(m[2], 10, 64)
			if err != nil {
				throw(rng("the seconds are too large"))
			}
			nanos := 0
			if m[3] != "" {
				nanos, _ = strconv.Atoi(m[3] + strings.Repeat("0", 9-len(m[3])))
			}
			d.Seconds, orderTime = &Seconds{m[1] == "-", whole, nanos}, 3
		default:
			throw(form("components are in the order Y M D, then T, H M S"))
		}
	}
	if !anyComponent {
		throw(form("a duration has at least one component"))
	}
	if inTime && timeComponents == 0 {
		throw(form("T needs at least one component"))
	}
	return d
}

var numberRE = regexp.MustCompile(`^-?[0-9]+(\.[0-9]+)?$`)

func numberText(f float64) string {
	if f == 0 {
		return "0"
	}
	return strings.TrimSuffix(FloatText(f), ".0")
}

func parsePoint(s string) (Point, string) {
	var srid *uint64
	coords := s
	if rest, ok := strings.CutPrefix(s, "SRID="); ok {
		n, c, found := strings.Cut(rest, ";")
		if !found {
			throw(form("SRID=n; needs the semicolon"))
		}
		v, err := strconv.ParseUint(n, 10, 64)
		if !allDigits(n) || err != nil || v == 0 {
			throw(form("the SRID is a positive integer"))
		}
		srid, coords = &v, c
	}
	parts := strings.Split(coords, " ")
	if len(parts) != 2 && len(parts) != 3 {
		throw(form("a point is two or three numbers separated by single spaces"))
	}
	nums := make([]float64, len(parts))
	for i, p := range parts {
		if !numberRE.MatchString(p) {
			throw(form("a point is two or three numbers separated by single spaces"))
		}
		f, err := strconv.ParseFloat(p, 64)
		if err != nil || math.IsInf(f, 0) {
			throw(rng("a coordinate is out of range"))
		}
		nums[i] = f
	}
	normal := ""
	if srid != nil {
		normal = fmt.Sprintf("SRID=%d;", *srid)
	}
	texts := make([]string, len(nums))
	for i, n := range nums {
		texts[i] = numberText(n)
	}
	normal += strings.Join(texts, " ")
	p := Point{SRID: srid, X: nums[0], Y: nums[1]}
	if len(nums) == 3 {
		p.Z = &nums[2]
	}
	return p, normal
}

var uuidRE = regexp.MustCompile(`^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$`)

func parseUUID(s string) UUID {
	if !uuidRE.MatchString(s) {
		throw(form("a UUID is 8-4-4-4-12 hexadecimal digits"))
	}
	var u UUID
	if _, err := hex.Decode(u[:], []byte(strings.ReplaceAll(s, "-", ""))); err != nil {
		throw(form("a UUID is 8-4-4-4-12 hexadecimal digits"))
	}
	return u
}

// jan1Weekday is the day of the week of 1 January, 0 = Monday (proleptic Gregorian).
func jan1Weekday(year int64) int64 {
	y := year - 1
	d := y*365 + y/4 - y/100 + y/400
	return ((d % 7) + 7) % 7
}

func has53Weeks(year int64) bool {
	w := jan1Weekday(year)
	return w == 3 || (isLeap(year) && w == 2)
}

var (
	weekRE     = regexp.MustCompile(`^([0-9]{4})W([0-9]{2})$`)
	weekDateRE = regexp.MustCompile(`^([0-9]{4})W([0-9]{2})-([0-9])$`)
)

func parseWeek(s string, withDay bool) Week {
	re, want := weekRE, "expected YYYYWww"
	if withDay {
		re, want = weekDateRE, "expected YYYYWww-D"
	}
	m := re.FindStringSubmatch(s)
	if m == nil {
		throw(form(want))
	}
	year, _ := strconv.Atoi(m[1])
	week, _ := strconv.Atoi(m[2])
	maxWeek := 52
	if has53Weeks(int64(year)) {
		maxWeek = 53
	}
	if week == 0 || week > maxWeek {
		throw(rng(fmt.Sprintf("%d has %d ISO weeks, not week %d", year, maxWeek, week)))
	}
	w := Week{Year: year, Week: week}
	if withDay {
		day, _ := strconv.Atoi(m[3])
		if day < 1 || day > 7 {
			throw(rng("the weekday is 1 to 7"))
		}
		w.Day = &day
	}
	return w
}
