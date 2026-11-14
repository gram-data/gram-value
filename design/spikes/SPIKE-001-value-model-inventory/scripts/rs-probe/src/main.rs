//! SPIKE-001 probe. Disposable. Runs one gram input per value kind through pattern-rs's parsers and serializer.
use gram_codec::{lower, parse_gram, parse_gram_cst, to_gram, Pattern, Subject};

fn props(ps: &[Pattern<Subject>]) -> String {
    let mut out = Vec::new();
    for p in ps {
        let mut keys: Vec<_> = p.value.properties.iter().collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        out.push(
            keys.iter()
                .map(|(k, v)| format!("{k} = {v:?}"))
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    out.join(" | ")
}

fn show(label: &str, which: &str, r: Result<Vec<Pattern<Subject>>, String>) {
    match r {
        Ok(ps) => {
            let text = to_gram(&ps).unwrap_or_else(|e| format!("<serialize error: {e}>"));
            println!("  {which:<4} -> {}\n       back -> {}", props(&ps), text.replace('\n', "\\n"));
        }
        Err(e) => println!("  {which:<4} -> ERROR {}", e.lines().next().unwrap_or("")),
    }
    let _ = label;
}

fn main() {
    let cases: Vec<(&str, String)> = vec![
        ("integer 0", "(n {k: 0})".into()),
        ("integer -7", "(n {k: -7})".into()),
        ("integer i64 max", "(n {k: 9223372036854775807})".into()),
        ("integer past i64", "(n {k: 9223372036854775808})".into()),
        ("hexadecimal", "(n {k: 0x1F})".into()),
        ("octal", "(n {k: 010})".into()),
        ("decimal", "(n {k: 3.14})".into()),
        ("decimal 1.0", "(n {k: 1.0})".into()),
        ("decimal negative", "(n {k: -0.5})".into()),
        ("measurement 5kg", "(n {k: 5kg})".into()),
        ("measurement 5.0kg", "(n {k: 5.0kg})".into()),
        ("measurement -5m", "(n {k: -5m})".into()),
        ("measurement 3.5kg", "(n {k: 3.5kg})".into()),
        ("range 1..5", "(n {k: 1..5})".into()),
        ("range open upper", "(n {k: 1...})".into()),
        ("range open lower", "(n {k: ...5})".into()),
        ("range decimals", "(n {k: 1.5..2.5})".into()),
        ("range measurements", "(n {k: 1.5m..2m})".into()),
        ("range hex", "(n {k: 0x1..0x10})".into()),
        ("boolean", "(n {k: true})".into()),
        ("symbol", "(n {k: active})".into()),
        ("symbol with punctuation", "(n {k: a.b-c@d})".into()),
        ("string double", "(n {k: \"dq\"})".into()),
        ("string single", "(n {k: 'sq'})".into()),
        ("string backtick", "(n {k: `bt`})".into()),
        ("string escapes", "(n {k: \"a\\n\\t\\\\\\\"b\"})".into()),
        ("string fenced", "(n {k: ```\nline1\nline2```})".into()),
        ("string empty", "(n {k: \"\"})".into()),
        ("tagged backtick", "(n {k: md`# T`})".into()),
        ("tagged fenced", "(n {k: ```md\nbody```})".into()),
        ("tagged date", "(n {k: date`2024-01-15`})".into()),
        ("array empty", "(n {k: []})".into()),
        ("array integers", "(n {k: [1, 2, 3]})".into()),
        ("array mixed", "(n {k: [1, \"a\", true]})".into()),
        ("array ranges and measurements", "(n {k: [1..2, 5kg]})".into()),
        ("array of arrays", "(n {k: [[1]]})".into()),
        ("map", "(n {k: {b: 1, a: 2}})".into()),
        ("map with array", "(n {k: {a: [1, 2]}})".into()),
        ("map repeated key", "(n {k: {a: 1, a: 2}})".into()),
        ("map nested", "(n {k: {a: {b: 1}}})".into()),
        ("record repeated key", "(n {k: 1, k: 2})".into()),
        ("record key order", "(n {z: 1, y: 2, x: 3, w: 4})".into()),
    ];
    for (label, text) in &cases {
        println!("== {label}: {}", text.replace('\n', "\\n"));
        show(label, "nom", parse_gram(text).map_err(|e| e.to_string()));
        let cst = parse_gram_cst(text);
        if cst.is_valid() {
            show(label, "cst", Ok(lower(cst.tree)));
        } else {
            println!("  cst  -> ERROR ({} syntax error(s))", cst.errors.len());
        }
    }
}
