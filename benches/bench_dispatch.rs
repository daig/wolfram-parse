//! Benchmark to measure the performance of character decoder dispatch
//! (match statement vs. the previous function pointer table).
//!
//! This focuses on inputs that exercise the backslash-escape dispatch path:
//! long names like \[Alpha], hex escapes, octal escapes, etc.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use wolfram_parser::ParseOptions;

fn tokenize(input: &[u8]) {
    wolfram_parser::tokenize_bytes(input, &ParseOptions::default()).unwrap();
}

fn parse(input: &[u8]) {
    wolfram_parser::parse_bytes_cst_seq(input, &ParseOptions::default());
}

fn bench_character_decoder_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("char_decoder_dispatch");

    // Input heavy in \[LongName] escapes — exercises the dispatch table path
    let long_names_heavy = {
        let mut s = String::new();
        for _ in 0..500 {
            s.push_str("\\[Alpha] + \\[Beta] * \\[Gamma] - \\[Delta] / \\[Epsilon]\n");
        }
        s.into_bytes()
    };
    group.bench_function("tokenize 500 lines of long names", |b| {
        b.iter(|| tokenize(&long_names_heavy))
    });
    group.bench_function("parse 500 lines of long names", |b| {
        b.iter(|| parse(&long_names_heavy))
    });

    // Input with hex escapes
    let hex_heavy = {
        let mut s = String::new();
        for _ in 0..500 {
            s.push_str("\"\\:03b1\\:03b2\\:03b3\\:03b4\\:03b5\"\n");
        }
        s.into_bytes()
    };
    group.bench_function("tokenize 500 lines of hex4 escapes", |b| {
        b.iter(|| tokenize(&hex_heavy))
    });

    // Input with single-char escapes (\\, \n, \t, \", etc.)
    let single_escape_heavy = {
        let mut s = String::new();
        for _ in 0..500 {
            s.push_str("\"hello\\nworld\\ttab\\\\backslash\\\"\"\n");
        }
        s.into_bytes()
    };
    group.bench_function("tokenize 500 lines of single escapes", |b| {
        b.iter(|| tokenize(&single_escape_heavy))
    });

    // Plain ASCII — baseline (no escapes, decoder is not the bottleneck)
    let plain_ascii = {
        let mut s = String::new();
        for _ in 0..500 {
            s.push_str("foo + bar * baz - qux / quux\n");
        }
        s.into_bytes()
    };
    group.bench_function("tokenize 500 lines of plain ASCII", |b| {
        b.iter(|| tokenize(&plain_ascii))
    });
    group.bench_function("parse 500 lines of plain ASCII", |b| {
        b.iter(|| parse(&plain_ascii))
    });

    group.finish();
}

fn bench_real_world_ish(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    // Load the test file if available
    let quantity_units = std::fs::read("tests/files/QuantityUnits.m").ok();
    if let Some(ref data) = quantity_units {
        group.bench_function("tokenize QuantityUnits.m", |b| {
            b.iter(|| tokenize(data))
        });
        group.bench_function("parse QuantityUnits.m", |b| {
            b.iter(|| parse(data))
        });
    }

    // Synthetic "realistic" input mixing operators, symbols, strings, numbers, escapes
    let mixed = {
        let mut s = String::new();
        for i in 0..200 {
            s.push_str(&format!(
                "f[x_{i}] := Module[{{y = x + \\[Pi]}}, y^2 + \"str\\n\" <> ToString[{i}]]\n"
            ));
        }
        s.into_bytes()
    };
    group.bench_function("tokenize 200 lines mixed WL", |b| {
        b.iter(|| tokenize(&mixed))
    });
    group.bench_function("parse 200 lines mixed WL", |b| {
        b.iter(|| parse(&mixed))
    });

    group.finish();
}

criterion_group!(benches, bench_character_decoder_dispatch, bench_real_world_ish);
criterion_main!(benches);
