package gramvalue

import (
	"bytes"
	"encoding/base64"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
	"unicode/utf16"
	"unicode/utf8"
)

// KnownTags are the eleven tags whose content has a standard form (ADR-004, ADR-008).
var KnownTags = []string{"date", "time", "localtime", "datetime", "localdatetime", "duration", "point", "uuid", "week", "weekDate", "embedding"}

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

// EmbeddingDtype is the floating-point element encoding of an Embedding.
type EmbeddingDtype string

const (
	DtypeF16  EmbeddingDtype = "F16"
	DtypeBF16 EmbeddingDtype = "BF16"
	DtypeF32  EmbeddingDtype = "F32"
	DtypeF64  EmbeddingDtype = "F64"
)

// Width returns the element byte width.
func (d EmbeddingDtype) Width() int {
	switch d {
	case DtypeF16, DtypeBF16:
		return 2
	case DtypeF32:
		return 4
	case DtypeF64:
		return 8
	default:
		return 0
	}
}

// MaxEmbeddingDimension is the maximum vector dimension (2^20, ADR-008 2.4).
const MaxEmbeddingDimension = 1_048_576

// Embedding is a rank-one dense floating-point tensor carried by an `embedding` tag (ADR-008).
type Embedding struct {
	dtype     EmbeddingDtype
	dimension int
	payload   []byte
}

// NewEmbedding constructs an Embedding after validating dtype, dimension, and payload byte length.
func NewEmbedding(dtype EmbeddingDtype, dimension int, payload []byte) (Embedding, error) {
	if dtype.Width() == 0 {
		return Embedding{}, diag("unsupported", fmt.Sprintf("embedding: unsupported dtype %q", dtype))
	}
	if dimension <= 0 || dimension > MaxEmbeddingDimension {
		return Embedding{}, diag("out-of-range", fmt.Sprintf("embedding: dimension %d must be between 1 and %d", dimension, MaxEmbeddingDimension))
	}
	expected := dimension * dtype.Width()
	if len(payload) != expected {
		return Embedding{}, diag("invalid-form", fmt.Sprintf("embedding: expected %d payload bytes, got %d", expected, len(payload)))
	}
	cp := make([]byte, len(payload))
	copy(cp, payload)
	return Embedding{dtype: dtype, dimension: dimension, payload: cp}, nil
}

// Dtype returns the element encoding.
func (e Embedding) Dtype() EmbeddingDtype { return e.dtype }

// Dimension returns the vector dimension.
func (e Embedding) Dimension() int { return e.dimension }

// Bytes returns a copy of the raw little-endian payload bytes.
func (e Embedding) Bytes() []byte {
	cp := make([]byte, len(e.payload))
	copy(cp, e.payload)
	return cp
}

// Equals compares two embeddings by exact dtype, dimension, and payload bytes.
func (e Embedding) Equals(other Embedding) bool {
	return e.dtype == other.dtype && e.dimension == other.dimension && bytes.Equal(e.payload, other.payload)
}

// ToNormalContent returns the canonical Base64-encoded SafeTensors content string.
func (e Embedding) ToNormalContent() string {
	return encodeEmbeddingNormal(e.dtype, e.dimension, e.payload)
}

// ToValue returns a TaggedValue("embedding", ...) with the canonical normal content string.
func (e Embedding) ToValue() Value {
	v, err := NewTagged("embedding", e.ToNormalContent())
	if err != nil {
		panic("embedding is a valid symbol")
	}
	return v
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
func unsup(m string) fault           { return fault{"unsupported", m} }
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
	case "embedding":
		emb, normal := parseEmbedding(content)
		return &Interpretation{tag, emb, normal}
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

// ---- embedding (ADR-008) --------------------------------------------------------------------

const (
	maxBase64Chars  = 11_190_284
	maxDecodedBytes = 8_392_712
	maxHeaderBytes  = 4_096
)

func b64Val(b byte) int {
	switch {
	case b >= 'A' && b <= 'Z':
		return int(b - 'A')
	case b >= 'a' && b <= 'z':
		return int(b-'a') + 26
	case b >= '0' && b <= '9':
		return int(b-'0') + 52
	case b == '+':
		return 62
	case b == '/':
		return 63
	default:
		return -1
	}
}

func decodeStrictBase64(content string) []byte {
	n := len(content)
	if n == 0 {
		throw(form("empty Base64 content"))
	}
	if n%4 != 0 {
		throw(form("Base64 length must be a multiple of 4"))
	}
	padCount := 0
	if strings.HasSuffix(content, "==") {
		padCount = 2
	} else if strings.HasSuffix(content, "=") {
		padCount = 1
	}
	dataEnd := n - padCount
	for i := 0; i < dataEnd; i++ {
		if b64Val(content[i]) < 0 {
			throw(formf("invalid Base64 byte at index %d", i))
		}
	}
	if padCount == 1 {
		if (b64Val(content[n-2]) & 0x03) != 0 {
			throw(form("non-canonical Base64 pad bits"))
		}
	} else if padCount == 2 {
		if (b64Val(content[n-3]) & 0x0f) != 0 {
			throw(form("non-canonical Base64 pad bits"))
		}
	}

	decodedLen := (n/4)*3 - padCount
	if n > maxBase64Chars || decodedLen > maxDecodedBytes {
		throw(rng(fmt.Sprintf("embedding exceeds maximum decoded size of %d bytes", maxDecodedBytes)))
	}
	decoded, err := base64.StdEncoding.Strict().DecodeString(content)
	if err != nil {
		throw(form("malformed Base64 content"))
	}
	return decoded
}

func encodeEmbeddingNormal(dtype EmbeddingDtype, dimension int, payload []byte) string {
	jsonStr := fmt.Sprintf(`{"embedding":{"dtype":"%s","shape":[%d],"data_offsets":[0,%d]}}`, dtype, dimension, len(payload))
	pad := (8 - (len(jsonStr) % 8)) % 8
	headerLen := len(jsonStr) + pad
	buf := make([]byte, 8+headerLen+len(payload))
	binary.LittleEndian.PutUint64(buf[:8], uint64(headerLen))
	copy(buf[8:], jsonStr)
	for i := 8 + len(jsonStr); i < 8+headerLen; i++ {
		buf[i] = ' '
	}
	copy(buf[8+headerLen:], payload)
	return base64.StdEncoding.EncodeToString(buf)
}

type headerParser struct {
	s   string
	pos int
}

func (p *headerParser) skipWS() {
	for p.pos < len(p.s) {
		switch p.s[p.pos] {
		case ' ', '\t', '\n', '\r':
			p.pos++
		default:
			return
		}
	}
}

func (p *headerParser) expect(ch byte) {
	p.skipWS()
	if p.pos >= len(p.s) || p.s[p.pos] != ch {
		throw(formf("expected '%c' in JSON header", ch))
	}
	p.pos++
}

func (p *headerParser) parseHex4() rune {
	if p.pos+4 > len(p.s) {
		throw(form("short \\uXXXX escape in JSON string"))
	}
	v, err := strconv.ParseUint(p.s[p.pos:p.pos+4], 16, 16)
	if err != nil {
		throw(form("invalid \\uXXXX escape in JSON string"))
	}
	p.pos += 4
	return rune(v)
}

func (p *headerParser) parseString() string {
	p.skipWS()
	if p.pos >= len(p.s) || p.s[p.pos] != '"' {
		throw(form("expected JSON string"))
	}
	p.pos++
	var out strings.Builder
	for p.pos < len(p.s) {
		b := p.s[p.pos]
		if b == '"' {
			p.pos++
			return out.String()
		}
		if b < 0x20 {
			throw(form("unescaped control character in JSON string"))
		}
		if b == '\\' {
			p.pos++
			if p.pos >= len(p.s) {
				throw(form("unterminated JSON escape"))
			}
			esc := p.s[p.pos]
			p.pos++
			switch esc {
			case '"', '\\', '/':
				out.WriteByte(esc)
			case 'b':
				out.WriteByte('\b')
			case 'f':
				out.WriteByte('\f')
			case 'n':
				out.WriteByte('\n')
			case 'r':
				out.WriteByte('\r')
			case 't':
				out.WriteByte('\t')
			case 'u':
				r := p.parseHex4()
				if r >= 0xD800 && r <= 0xDBFF {
					if p.pos+2 > len(p.s) || p.s[p.pos] != '\\' || p.s[p.pos+1] != 'u' {
						throw(form("unpaired UTF-16 high surrogate in JSON string"))
					}
					p.pos += 2
					low := p.parseHex4()
					if low < 0xDC00 || low > 0xDFFF {
						throw(form("invalid UTF-16 low surrogate in JSON string"))
					}
					out.WriteRune(utf16.DecodeRune(r, low))
				} else if r >= 0xDC00 && r <= 0xDFFF {
					throw(form("unpaired UTF-16 low surrogate in JSON string"))
				} else {
					out.WriteRune(r)
				}
			default:
				throw(form("invalid JSON escape sequence"))
			}
		} else {
			r, size := utf8.DecodeRuneInString(p.s[p.pos:])
			if r == utf8.RuneError && size == 1 {
				throw(form("invalid UTF-8 in JSON string"))
			}
			out.WriteRune(r)
			p.pos += size
		}
	}
	throw(form("unterminated JSON string"))
	return ""
}

func (p *headerParser) parsePlainUintToken() string {
	p.skipWS()
	start := p.pos
	for p.pos < len(p.s) && p.s[p.pos] >= '0' && p.s[p.pos] <= '9' {
		p.pos++
	}
	if p.pos == start {
		throw(form("expected plain base-10 integer"))
	}
	if p.pos-start > 1 && p.s[start] == '0' {
		throw(form("leading zero in JSON integer"))
	}
	if p.pos < len(p.s) && (p.s[p.pos] == '.' || p.s[p.pos] == 'e' || p.s[p.pos] == 'E') {
		throw(form("non-integer JSON number"))
	}
	return p.s[start:p.pos]
}

func (p *headerParser) parseShape() (uint64, bool) {
	p.expect('[')
	tok := p.parsePlainUintToken()
	p.expect(']')
	n, err := strconv.ParseUint(tok, 10, 64)
	if err != nil {
		return 0, true
	}
	return n, false
}

func (p *headerParser) parseDataOffsets() (uint64, uint64) {
	p.expect('[')
	t0 := p.parsePlainUintToken()
	p.expect(',')
	t1 := p.parsePlainUintToken()
	p.expect(']')
	a, err0 := strconv.ParseUint(t0, 10, 64)
	b, err1 := strconv.ParseUint(t1, 10, 64)
	if err0 != nil || err1 != nil {
		throw(form("data_offsets exceeds 64 unsigned bits"))
	}
	return a, b
}

func (p *headerParser) parseHeader() (string, uint64, bool, uint64, uint64) {
	p.expect('{')
	topKey := p.parseString()
	if topKey != "embedding" {
		throw(formf("unexpected top-level key %q", topKey))
	}
	p.expect(':')
	p.expect('{')

	var dtype string
	var hasDtype, hasShape, hasOffsets bool
	var shapeDim uint64
	var shapeOverflow bool
	var off0, off1 uint64

	for idx := 0; idx < 3; idx++ {
		if idx > 0 {
			p.expect(',')
		}
		key := p.parseString()
		p.expect(':')
		switch key {
		case "dtype":
			if hasDtype {
				throw(form(`duplicate key "dtype"`))
			}
			hasDtype = true
			dtype = p.parseString()
		case "shape":
			if hasShape {
				throw(form(`duplicate key "shape"`))
			}
			hasShape = true
			shapeDim, shapeOverflow = p.parseShape()
		case "data_offsets":
			if hasOffsets {
				throw(form(`duplicate key "data_offsets"`))
			}
			hasOffsets = true
			off0, off1 = p.parseDataOffsets()
		default:
			throw(formf("unexpected tensor key %q", key))
		}
	}
	p.expect('}')
	p.expect('}')
	p.skipWS()
	if p.pos != len(p.s) {
		throw(form("trailing bytes in SafeTensors JSON header"))
	}
	return dtype, shapeDim, shapeOverflow, off0, off1
}

func parseEmbedding(content string) (Embedding, string) {
	decoded := decodeStrictBase64(content)
	if len(decoded) < 8 {
		throw(form("SafeTensors buffer shorter than 8 bytes"))
	}
	headerLenU64 := binary.LittleEndian.Uint64(decoded[:8])
	remaining := uint64(len(decoded) - 8)
	if headerLenU64 > maxHeaderBytes || headerLenU64 > remaining {
		throw(form("SafeTensors header length exceeds 4,096 or buffer"))
	}
	headerLen := int(headerLenU64)
	headerBytes := decoded[8 : 8+headerLen]
	if !utf8.Valid(headerBytes) {
		throw(form("SafeTensors header is not valid UTF-8"))
	}
	p := headerParser{s: string(headerBytes)}
	dtypeRaw, shapeDim, shapeOverflow, off0, off1 := p.parseHeader()

	var dtype EmbeddingDtype
	switch dtypeRaw {
	case "F16":
		dtype = DtypeF16
	case "BF16":
		dtype = DtypeBF16
	case "F32":
		dtype = DtypeF32
	case "F64":
		dtype = DtypeF64
	default:
		throw(unsup(fmt.Sprintf("unsupported embedding dtype %q", dtypeRaw)))
	}

	if shapeOverflow {
		throw(rng("embedding dimension exceeds 64 unsigned bits"))
	}
	if shapeDim == 0 || shapeDim > MaxEmbeddingDimension {
		throw(rng(fmt.Sprintf("embedding dimension %d must be between 1 and %d", shapeDim, MaxEmbeddingDimension)))
	}
	dimension := int(shapeDim)

	expectedBytes := dimension * dtype.Width()
	if off0 != 0 || off1 != uint64(expectedBytes) {
		throw(formf("data_offsets [%d, %d] does not match [0, %d]", off0, off1, expectedBytes))
	}
	payloadStart := 8 + headerLen
	if len(decoded) != payloadStart+expectedBytes {
		throw(formf("payload length %d does not match expected %d", len(decoded)-payloadStart, expectedBytes))
	}

	payload := make([]byte, expectedBytes)
	copy(payload, decoded[payloadStart:])
	emb := Embedding{dtype: dtype, dimension: dimension, payload: payload}
	normal := encodeEmbeddingNormal(dtype, dimension, payload)
	return emb, normal
}
