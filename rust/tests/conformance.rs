//! Runs the language-neutral conformance corpus (ADR-001) against this package.
//!
//! The corpus is gram text. Each case's `value` is read with tree-sitter-gram and lowered to a
//! `gram_value::Value` by the functions below, which live in test code and are not shipped (ADR-001 2.3).

use gram_value::{tags, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

const GRAMMAR_VERSION: &str = "0.7.0";

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../conformance")
}

fn parse(src: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_gram::LANGUAGE.into())
        .expect("load grammar");
    parser.parse(src, None).expect("parse")
}

// ---- lowering -------------------------------------------------------------------------------

fn text<'a>(src: &'a str, n: Node<'_>) -> &'a str {
    &src[n.byte_range()]
}

fn content<'a>(src: &'a str, n: Node<'_>) -> &'a str {
    n.child_by_field_name("content")
        .map(|c| text(src, c))
        .unwrap_or("")
}

fn unescape(s: &str, quote: char) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('/') => out.push('/'),
            Some('\\') => out.push('\\'),
            Some(q) if q == quote => out.push(q),
            other => panic!("escape the grammar does not allow: {other:?}"),
        }
    }
    out
}

fn string_value(src: &str, n: Node<'_>) -> String {
    let full = text(src, n);
    if full.starts_with("```") {
        content(src, n).to_string()
    } else {
        unescape(content(src, n), full.chars().next().unwrap())
    }
}

/// Convert digits in `radix` to decimal digits, for hexadecimal and octal literals of any size.
fn radix_to_decimal(digits: &str, radix: u32) -> String {
    let mut dec: Vec<u32> = vec![0]; // little-endian decimal digits
    for c in digits.chars() {
        let mut carry = c.to_digit(radix).expect("digit");
        for d in dec.iter_mut() {
            let v = *d * radix + carry;
            *d = v % 10;
            carry = v / 10;
        }
        while carry > 0 {
            dec.push(carry % 10);
            carry /= 10;
        }
    }
    dec.iter()
        .rev()
        .map(|d| char::from_digit(*d, 10).unwrap())
        .collect()
}

fn lower_integer_text(t: &str) -> Value {
    let t = if t == "-0" { "0" } else { t };
    Value::integer_from_digits(t).expect("integer literal")
}

fn lower_number_text(t: &str) -> Value {
    if t.contains('.') {
        Value::decimal_from_text(t).expect("decimal literal")
    } else {
        lower_integer_text(t)
    }
}

fn lower_numeric(src: &str, n: Node<'_>) -> Value {
    let t = text(src, n);
    match n.kind() {
        "integer" | "decimal" => lower_number_text(t),
        "hexadecimal" => Value::integer_from_digits(&radix_to_decimal(&t[2..], 16)).unwrap(),
        "octal" => Value::integer_from_digits(&radix_to_decimal(&t[1..], 8)).unwrap(),
        "measurement" => {
            let split = t.find(|c: char| c.is_ascii_alphabetic()).unwrap();
            Value::measurement(lower_number_text(&t[..split]), &t[split..]).unwrap()
        }
        k => panic!("not a numeric literal: {k}"),
    }
}

fn key_name(src: &str, n: Node<'_>) -> String {
    match n.kind() {
        "quoted_name" => unescape(content(src, n), '`'),
        "double_quoted_name" => unescape(content(src, n), '"'),
        _ => text(src, n).to_string(),
    }
}

fn named_children<'t>(n: Node<'t>) -> Vec<Node<'t>> {
    let mut c = n.walk();
    n.named_children(&mut c)
        .filter(|k| k.kind() != "comment" && k.kind() != "doc_comment")
        .collect()
}

fn lower(src: &str, n: Node<'_>) -> Value {
    match n.kind() {
        "integer" | "decimal" | "hexadecimal" | "octal" | "measurement" => lower_numeric(src, n),
        "boolean_literal" => Value::boolean(text(src, n) == "true"),
        "symbol" => Value::string(text(src, n)),
        "string_literal" => Value::string(string_value(src, n)),
        "tagged_string" => {
            let tag = text(src, n.child_by_field_name("tag").unwrap());
            let full = text(src, n);
            let body = if full.starts_with("```") {
                content(src, n).to_string()
            } else {
                unescape(content(src, n), '`')
            };
            Value::tagged(tag, body).unwrap()
        }
        "range" => {
            let end = |f: &str| n.child_by_field_name(f).map(|e| lower_numeric(src, e));
            Value::range(end("lower"), end("upper")).unwrap()
        }
        "array" => Value::array(named_children(n).into_iter().map(|c| lower(src, c))),
        "map" => {
            let entries = named_children(n).into_iter().map(|e| {
                let k = key_name(src, e.child_by_field_name("key").unwrap());
                (k, lower(src, e.child_by_field_name("value").unwrap()))
            });
            Value::map(entries).unwrap_or_else(|d| panic!("{d}"))
        }
        k => panic!("cannot lower node kind {k}"),
    }
}

// ---- cases ----------------------------------------------------------------------------------

struct Case {
    id: String,
    label: String,
    props: BTreeMap<String, Value>,
    value: Value,
}

fn load_cases(file: &str) -> Vec<Case> {
    let src = std::fs::read_to_string(corpus_dir().join(file)).unwrap();
    let tree = parse(&src);
    assert!(!tree.root_node().has_error(), "{file} has a syntax error");
    let mut cases = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "node_pattern" {
            let id = n
                .child_by_field_name("identifier")
                .map(|i| text(&src, i).to_string());
            let labels = n.child_by_field_name("labels").map(|l| {
                let mut c = l.walk();
                l.named_children(&mut c)
                    .map(|s| text(&src, s).to_string())
                    .collect::<Vec<_>>()
            });
            if let (Some(id), Some(labels), Some(record)) =
                (id, labels, n.child_by_field_name("record"))
            {
                let mut props = BTreeMap::new();
                for p in named_children(record) {
                    let k = key_name(&src, p.child_by_field_name("key").unwrap());
                    props.insert(k, lower(&src, p.child_by_field_name("value").unwrap()));
                }
                let value = props.get("value").cloned().expect("a case has a value");
                cases.push(Case {
                    id,
                    label: labels[0].clone(),
                    props,
                    value,
                });
            }
            continue;
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    cases.sort_by(|a, b| a.id.cmp(&b.id));
    cases
}

fn prop_str<'a>(c: &'a Case, k: &str) -> &'a str {
    match c.props.get(k) {
        Some(Value::String(s)) => s,
        other => panic!(
            "case {}: property {k} should be a string, found {other:?}",
            c.id
        ),
    }
}

fn vocabulary() -> BTreeMap<String, BTreeSet<String>> {
    let src = std::fs::read_to_string(corpus_dir().join("vocabulary.gram")).unwrap();
    let tree = parse(&src);
    assert!(
        !tree.root_node().has_error(),
        "vocabulary.gram has a syntax error"
    );
    let mut vocab: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "node_pattern" {
            if let (Some(id), Some(labels)) = (
                n.child_by_field_name("identifier"),
                n.child_by_field_name("labels"),
            ) {
                let mut c = labels.walk();
                let label = labels
                    .named_children(&mut c)
                    .next()
                    .map(|l| text(&src, l).to_string())
                    .unwrap();
                vocab
                    .entry(label)
                    .or_default()
                    .insert(text(&src, id).to_string());
            }
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    vocab
}

fn check_vocab(vocab: &BTreeMap<String, BTreeSet<String>>, group: &str, name: &str, case: &str) {
    assert!(
        vocab.get(group).is_some_and(|s| s.contains(name)),
        "case {case}: {name:?} is not in the vocabulary group {group}"
    );
}

#[test]
fn corpus() {
    let version = std::fs::read_to_string(corpus_dir().join("GRAMMAR_VERSION")).unwrap();
    assert_eq!(
        version.trim(),
        GRAMMAR_VERSION,
        "the corpus pins a different grammar version"
    );

    let vocab = vocabulary();
    let mut results = String::new();
    let mut seen = BTreeSet::new();
    let mut failures = Vec::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for file in ["values.gram", "text.gram", "tags.gram"] {
        for case in load_cases(file) {
            assert!(
                seen.insert(case.id.clone()),
                "case id {} is used twice",
                case.id
            );
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_case(&case, &vocab)));
            let status = match outcome {
                Ok(()) => "passed",
                Err(e) => {
                    let msg = e
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    failures.push(format!("{}: {msg}", case.id));
                    "failed"
                }
            };
            *counts.entry(status.to_string()).or_default() += 1;
            writeln!(results, "({}:Result {{status: {status}}})", case.id).unwrap();
        }
    }
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/conformance-results.gram");
    let _ = std::fs::write(out, results);
    assert!(
        failures.is_empty(),
        "{} failed of {}:\n{}",
        failures.len(),
        seen.len(),
        failures.join("\n")
    );
    assert!(
        seen.len() > 100,
        "the corpus should hold many cases, found {}",
        seen.len()
    );
    eprintln!("conformance: {counts:?}");
}

fn run_case(case: &Case, vocab: &BTreeMap<String, BTreeSet<String>>) {
    match case.label.as_str() {
        "ValueCase" => {
            let kind = prop_str(case, "kind");
            check_vocab(vocab, "Kind", kind, &case.id);
            assert_eq!(case.value.kind().name(), kind, "wrong kind");
        }
        "TextCase" => {
            let want = prop_str(case, "canonical");
            let got = case
                .value
                .to_text()
                .unwrap_or_else(|e| panic!("unwritable: {e:?}"));
            assert_eq!(got, want, "canonical text");
            // The canonical text reads back as the same value.
            let wrapper = format!("(x {{value: {got}}})");
            let tree = parse(&wrapper);
            assert!(
                !tree.root_node().has_error(),
                "canonical text {got} does not parse"
            );
            let again = load_value(&wrapper, &tree);
            assert_eq!(
                again, case.value,
                "canonical text {got} reads back as a different value"
            );
        }
        "TagCase" => {
            let outcome = prop_str(case, "outcome");
            check_vocab(vocab, "Outcome", outcome, &case.id);
            let Value::Tagged(t) = &case.value else {
                panic!("a TagCase value is a tagged string")
            };
            check_vocab(vocab, "Tag", t.tag(), &case.id);
            match (outcome, tags::interpret(&case.value)) {
                ("accepted", Ok(Some(i))) => {
                    assert_eq!(
                        i.normal_content,
                        prop_str(case, "interpreted"),
                        "normal content"
                    )
                }
                ("rejected", Err(d)) => {
                    let want = prop_str(case, "diagnostic");
                    check_vocab(vocab, "Diagnostic", want, &case.id);
                    assert_eq!(d.code, want, "diagnostic code ({})", d.message);
                }
                (o, r) => panic!("expected {o}, got {r:?}"),
            }
        }
        other => panic!("unknown case kind {other}"),
    }
}

fn load_value(src: &str, tree: &tree_sitter::Tree) -> Value {
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "record_property" {
            return lower(src, n.child_by_field_name("value").unwrap());
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    panic!("no value");
}
