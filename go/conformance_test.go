package gramvalue_test

// Runs the language-neutral conformance corpus (ADR-001) against this package.
//
// The corpus is gram text. Each case's value is read with tree-sitter-gram and lowered to a gramvalue.Value
// by the functions below, which live in test code and are not shipped (ADR-001 2.3).

import (
	"fmt"
	"math/big"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"testing"
	"unicode"

	gv "github.com/gram-data/gram-value/go"
	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_gram "github.com/tree-sitter/tree-sitter-gram/bindings/go"
)

const grammarVersion = "0.7.0"

func corpus(file string) string { return filepath.Join("..", "conformance", file) }

func parse(src []byte) *tree_sitter.Tree {
	p := tree_sitter.NewParser()
	defer p.Close()
	if err := p.SetLanguage(tree_sitter.NewLanguage(tree_sitter_gram.Language())); err != nil {
		panic(err)
	}
	return p.Parse(src, nil)
}

type source struct{ src []byte }

func (s source) text(n *tree_sitter.Node) string { return n.Utf8Text(s.src) }

func (s source) content(n *tree_sitter.Node) string {
	if c := n.ChildByFieldName("content"); c != nil {
		return s.text(c)
	}
	return ""
}

func unescape(s string, quote rune) string {
	var b strings.Builder
	rs := []rune(s)
	for i := 0; i < len(rs); i++ {
		if rs[i] != '\\' {
			b.WriteRune(rs[i])
			continue
		}
		i++
		switch e := rs[i]; e {
		case 'b':
			b.WriteRune('\b')
		case 'f':
			b.WriteRune('\f')
		case 'n':
			b.WriteRune('\n')
		case 'r':
			b.WriteRune('\r')
		case 't':
			b.WriteRune('\t')
		case '/', '\\':
			b.WriteRune(e)
		default:
			if e != quote {
				panic("escape the grammar does not allow: " + string(e))
			}
			b.WriteRune(e)
		}
	}
	return b.String()
}

func (s source) stringValue(n *tree_sitter.Node) string {
	full := s.text(n)
	if strings.HasPrefix(full, "```") {
		return s.content(n)
	}
	return unescape(s.content(n), []rune(full)[0])
}

func must(v gv.Value, err error) gv.Value {
	if err != nil {
		panic(err)
	}
	return v
}

func lowerNumberText(t string) gv.Value {
	if strings.Contains(t, ".") {
		return must(gv.NewDecimalFromText(t))
	}
	if t == "-0" {
		t = "0"
	}
	return must(gv.NewIntegerFromDigits(t))
}

func (s source) lowerNumeric(n *tree_sitter.Node) gv.Value {
	t := s.text(n)
	switch n.Kind() {
	case "integer", "decimal":
		return lowerNumberText(t)
	case "hexadecimal":
		i, _ := new(big.Int).SetString(t[2:], 16)
		return gv.NewIntegerFromBig(i)
	case "octal":
		i, _ := new(big.Int).SetString(t[1:], 8)
		return gv.NewIntegerFromBig(i)
	case "measurement":
		split := strings.IndexFunc(t, unicode.IsLetter)
		return must(gv.NewMeasurement(lowerNumberText(t[:split]), t[split:]))
	}
	panic("not a numeric literal: " + n.Kind())
}

func (s source) keyName(n *tree_sitter.Node) string {
	switch n.Kind() {
	case "quoted_name":
		return unescape(s.content(n), '`')
	case "double_quoted_name":
		return unescape(s.content(n), '"')
	}
	return s.text(n)
}

func named(n *tree_sitter.Node) []*tree_sitter.Node {
	var out []*tree_sitter.Node
	for i := uint(0); i < n.NamedChildCount(); i++ {
		c := n.NamedChild(i)
		if c.Kind() != "comment" && c.Kind() != "doc_comment" {
			out = append(out, c)
		}
	}
	return out
}

func (s source) lower(n *tree_sitter.Node) gv.Value {
	switch n.Kind() {
	case "integer", "decimal", "hexadecimal", "octal", "measurement":
		return s.lowerNumeric(n)
	case "boolean_literal":
		return gv.NewBoolean(s.text(n) == "true")
	case "symbol":
		return must(gv.NewString(s.text(n)))
	case "string_literal":
		return must(gv.NewString(s.stringValue(n)))
	case "tagged_string":
		body := s.content(n)
		if !strings.HasPrefix(s.text(n), "```") {
			body = unescape(body, '`')
		}
		return must(gv.NewTagged(s.text(n.ChildByFieldName("tag")), body))
	case "range":
		var ends [2]gv.Value
		for i, f := range []string{"lower", "upper"} {
			if e := n.ChildByFieldName(f); e != nil {
				ends[i] = s.lowerNumeric(e)
			}
		}
		return must(gv.NewRange(ends[0], ends[1]))
	case "array":
		var items []gv.Value
		for _, c := range named(n) {
			items = append(items, s.lower(c))
		}
		return gv.NewArray(items...)
	case "map":
		var entries []gv.Entry
		for _, e := range named(n) {
			entries = append(entries, gv.Entry{Key: s.keyName(e.ChildByFieldName("key")), Value: s.lower(e.ChildByFieldName("value"))})
		}
		return must(gv.NewMap(entries...))
	}
	panic("cannot lower node kind " + n.Kind())
}

// ---- cases ----------------------------------------------------------------------------------

type testCase struct {
	id, label string
	props     map[string]gv.Value
	value     gv.Value
}

func walk(n *tree_sitter.Node, visit func(*tree_sitter.Node)) {
	visit(n)
	for i := uint(0); i < n.ChildCount(); i++ {
		walk(n.Child(i), visit)
	}
}

func loadNodes(t *testing.T, path string) []testCase {
	src, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	tree := parse(src)
	defer tree.Close()
	if tree.RootNode().HasError() {
		t.Fatalf("%s has a syntax error", path)
	}
	s := source{src}
	var cases []testCase
	walk(tree.RootNode(), func(n *tree_sitter.Node) {
		if n.Kind() != "node_pattern" {
			return
		}
		id, labels, record := n.ChildByFieldName("identifier"), n.ChildByFieldName("labels"), n.ChildByFieldName("record")
		if id == nil || labels == nil || record == nil {
			return
		}
		props := map[string]gv.Value{}
		for _, p := range named(record) {
			props[s.keyName(p.ChildByFieldName("key"))] = s.lower(p.ChildByFieldName("value"))
		}
		cases = append(cases, testCase{s.text(id), s.text(named(labels)[0]), props, props["value"]})
	})
	sort.Slice(cases, func(i, j int) bool { return cases[i].id < cases[j].id })
	return cases
}

func loadSkips(t *testing.T) map[string]string {
	skips := map[string]string{}
	for _, n := range loadNodes(t, "skips.gram") {
		if n.label != "Skip" {
			t.Fatalf("%s: a skips file holds Skip nodes", n.id)
		}
		skips[propStr(n, "case")] = propStr(n, "reason")
	}
	return skips
}

func vocabulary(t *testing.T) map[string]map[string]bool {
	src, err := os.ReadFile(corpus("vocabulary.gram"))
	if err != nil {
		t.Fatal(err)
	}
	tree := parse(src)
	defer tree.Close()
	if tree.RootNode().HasError() {
		t.Fatal("vocabulary.gram has a syntax error")
	}
	s := source{src}
	vocab := map[string]map[string]bool{}
	walk(tree.RootNode(), func(n *tree_sitter.Node) {
		if n.Kind() != "node_pattern" {
			return
		}
		id, labels := n.ChildByFieldName("identifier"), n.ChildByFieldName("labels")
		if id == nil || labels == nil {
			return
		}
		for _, l := range named(labels) {
			if vocab[s.text(l)] == nil {
				vocab[s.text(l)] = map[string]bool{}
			}
			vocab[s.text(l)][s.text(id)] = true
		}
	})
	return vocab
}

func propStr(c testCase, k string) string {
	v, ok := c.props[k].(gv.StringValue)
	if !ok {
		panic(fmt.Sprintf("case %s: property %s should be a string", c.id, k))
	}
	return v.Text()
}

// failure is a failed check inside a case.
type failure string

func check(cond bool, format string, a ...any) {
	if !cond {
		panic(failure(fmt.Sprintf(format, a...)))
	}
}

func checkVocab(vocab map[string]map[string]bool, group, name, id string) {
	check(vocab[group][name], "case %s: %q is not in the vocabulary group %s", id, name, group)
}

func textOf(v gv.Value) string {
	s, err := gv.ToText(v, gv.ContextValue)
	check(err == nil, "unwritable: %v", err)
	return s
}

func loadValueFromText(text string) gv.Value {
	src := []byte("(x {value: " + text + "})")
	tree := parse(src)
	defer tree.Close()
	check(!tree.RootNode().HasError(), "canonical text %s does not parse", text)
	s := source{src}
	var found gv.Value
	walk(tree.RootNode(), func(n *tree_sitter.Node) {
		if n.Kind() == "record_property" && found == nil {
			found = s.lower(n.ChildByFieldName("value"))
		}
	})
	return found
}

var tagForNative = map[string]string{"Date": "date", "Time": "time", "DateTime": "datetime", "Duration": "duration", "Point": "point", "UUID": "uuid", "Week": "week"}

func nativeTypeName(n any) string {
	switch n.(type) {
	case int64:
		return "int64"
	case *big.Int:
		return "BigInt"
	case float64:
		return "float64"
	case bool:
		return "bool"
	case string:
		return "string"
	case gv.Tagged:
		return "Tagged"
	case gv.Range:
		return "Range"
	case gv.Measurement:
		return "Measurement"
	case []any:
		return "slice"
	case map[string]any:
		return "map"
	}
	return fmt.Sprintf("%T", n)
}

func runMapping(c testCase, vocab map[string]map[string]bool) {
	for k := range c.props {
		if k != "name" && k != "value" {
			checkVocab(vocab, "Language", k, c.id)
		}
	}
	entry, ok := c.props["go"].(gv.MapValue)
	if !ok {
		return
	}
	word := func(k string) string {
		if v, ok := entry.Get(k); ok {
			if s, ok := v.(gv.StringValue); ok {
				return s.Text()
			}
		}
		return ""
	}
	class, native := word("class"), word("native")
	checkVocab(vocab, "Class", class, c.id)
	checkVocab(vocab, "GoNative", native, c.id)
	var wantLoss []string
	if l, ok := entry.Get("loss"); ok {
		for _, x := range l.(gv.ArrayValue).Items() {
			wantLoss = append(wantLoss, x.(gv.StringValue).Text())
			checkVocab(vocab, "Loss", wantLoss[len(wantLoss)-1], c.id)
		}
	}
	check((class == "lossy") == (len(wantLoss) > 0), "loss kinds are present exactly when the class is lossy")
	// Go never reports a loss (ADR-006): every entry is exact or unsupported.
	check(class != "lossy", "Go has no lossy entries")

	if class == "unsupported" {
		check(native == "none", "an unsupported entry has native none")
		_, err := gv.ToNative(c.value)
		check(err != nil, "the conversion should fail")
		ds := err.(gv.Diagnostics)
		check(ds[0].Code == "unsupported", "code %s", ds[0].Code)
		return
	}
	var back gv.Value
	if tag, isTag := tagForNative[native]; isTag {
		i, err := gv.Interpret(c.value)
		check(err == nil && i != nil, "a known tag interprets: %v", err)
		check(i.Tag == tag, "the typed value is for a different tag")
		back = i.ToValue()
	} else {
		n, err := gv.ToNative(c.value)
		check(err == nil, "conversion failed: %v", err)
		check(nativeTypeName(n) == native, "native type %s, want %s", nativeTypeName(n), native)
		back, err = gv.FromNative(n)
		check(err == nil, "FromNative failed: %v", err)
	}
	check(textOf(back) == word("roundtrip"), "round trip through %s: %s", native, textOf(back))
	check(gv.Equal(back, c.value), "exact means the round trip gives an equal value")
}

func runCase(c testCase, vocab map[string]map[string]bool) {
	switch c.label {
	case "ValueCase":
		kind := propStr(c, "kind")
		checkVocab(vocab, "Kind", kind, c.id)
		check(c.value.Kind().String() == kind, "wrong kind %s", c.value.Kind())
	case "TextCase":
		got := textOf(c.value)
		check(got == propStr(c, "canonical"), "canonical text %s", got)
		check(gv.Equal(loadValueFromText(got), c.value), "canonical text %s reads back as a different value", got)
	case "TagCase":
		outcome := propStr(c, "outcome")
		checkVocab(vocab, "Outcome", outcome, c.id)
		tv, ok := c.value.(gv.TaggedValue)
		check(ok, "a TagCase value is a tagged string")
		checkVocab(vocab, "Tag", tv.Tag(), c.id)
		i, err := gv.Interpret(c.value)
		if outcome == "accepted" {
			check(err == nil && i != nil, "expected accepted, got %v", err)
			check(i.NormalContent == propStr(c, "interpreted"), "normal content %s", i.NormalContent)
		} else {
			check(err != nil, "expected rejected")
			want := propStr(c, "diagnostic")
			checkVocab(vocab, "Diagnostic", want, c.id)
			ds := err.(gv.Diagnostics)
			check(ds[0].Code == want, "diagnostic code %s (%s)", ds[0].Code, ds[0].Message)
		}
	case "MappingCase":
		runMapping(c, vocab)
	default:
		panic(failure("unknown case kind " + c.label))
	}
}

func try(f func()) (msg string) {
	defer func() {
		if r := recover(); r != nil {
			if fl, ok := r.(failure); ok {
				msg = string(fl)
				return
			}
			msg = fmt.Sprint("panic: ", r)
		}
	}()
	f()
	return ""
}

func TestCorpus(t *testing.T) {
	v, err := os.ReadFile(corpus("GRAMMAR_VERSION"))
	if err != nil || strings.TrimSpace(string(v)) != grammarVersion {
		t.Fatalf("the corpus pins a different grammar version: %q", v)
	}
	vocab := vocabulary(t)
	skips := loadSkips(t)
	seen := map[string]bool{}
	counts := map[string]int{}
	var failures []string
	var results strings.Builder
	for _, file := range []string{"values.gram", "text.gram", "tags.gram", "mappings.gram"} {
		for _, c := range loadNodes(t, corpus(file)) {
			if seen[c.id] {
				t.Fatalf("case id %s is used twice", c.id)
			}
			seen[c.id] = true
			// A mapping case with no entry for this language does not apply to it (ADR-001 2.4).
			if c.label == "MappingCase" {
				if _, ok := c.props["go"]; !ok {
					counts["not-applicable"]++
					fmt.Fprintf(&results, "(%s:Result {language: go, status: not-applicable})\n", c.id)
					continue
				}
			}
			msg := try(func() { runCase(c, vocab) })
			reason, skipped := skips[c.id]
			status, extra := "passed", ""
			switch {
			case msg == "" && !skipped:
			case msg == "" && skipped:
				status = "failed"
				failures = append(failures, c.id+": unexpected pass of a skipped case")
			case skipped:
				status, extra = "skipped", ", reason: "+strconv.Quote(reason)
			default:
				status = "failed"
				failures = append(failures, c.id+": "+msg)
			}
			counts[status]++
			fmt.Fprintf(&results, "(%s:Result {language: go, status: %s%s})\n", c.id, status, extra)
		}
	}
	for id := range skips {
		if !seen[id] {
			t.Errorf("skips.gram names an unknown case: %s", id)
		}
	}
	_ = os.MkdirAll("target", 0o755)
	_ = os.WriteFile(filepath.Join("target", "conformance-results.gram"), []byte(results.String()), 0o644)
	if len(failures) > 0 {
		t.Fatalf("%d failed of %d:\n%s", len(failures), len(seen), strings.Join(failures, "\n"))
	}
	if len(seen) <= 100 {
		t.Fatalf("the corpus should hold many cases, found %d", len(seen))
	}
	t.Logf("conformance: %v", counts)
}
