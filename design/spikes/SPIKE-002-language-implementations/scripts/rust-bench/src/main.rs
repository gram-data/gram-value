//! SPIKE-002 baseline: the native Rust enum, no boundary. Prints one line per segment.
use bench_core::*;
use std::time::Instant;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn run(n: u32, runs: usize) {
    let (mut create, mut collect, mut map, mut read, mut text) = (vec![], vec![], vec![], vec![], vec![]);
    let mut sink = 0.0;
    for r in 0..(runs + 3) {
        let t = Instant::now();
        let vals: Vec<Value> = (0..n).map(gen).collect();
        let c1 = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let list = Value::Array { items: vals };
        let c2 = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let mapped = Value::Array {
            items: match &list { Value::Array { items } => items.iter().map(map_one).collect(), _ => unreachable!() },
        };
        let c3 = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let s = match &mapped { Value::Array { items } => items.iter().map(checksum).sum::<f64>(), _ => 0.0 };
        let c4 = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let txt = to_gram(&mapped);
        let c5 = t.elapsed().as_secs_f64();
        sink += s + txt.len() as f64;
        if r >= 3 { create.push(c1); collect.push(c2); map.push(c3); read.push(c4); text.push(c5); }
    }
    let ms = |v: Vec<f64>| median(v) * 1000.0;
    let (a, b, c, d, e) = (ms(create), ms(collect), ms(map), ms(read), ms(text));
    println!("rust-native n={n}: create {a:.4} ms, collect {b:.4} ms, map {c:.4} ms, read {d:.4} ms, text {e:.4} ms, pipeline(create+collect+map+read) {:.4} ms [{:.3} us/value] (sink {sink:.0})", a + b + c + d, (a + b + c + d) * 1000.0 / f64::from(n));
}

fn main() {
    run(1_000, 200);
    run(100_000, 20);
}
