//! Benchmarks for string interning performance and memory usage

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wolfram_parser::{tokenize, ParseOptions};

fn bench_tokenization_memory_usage(c: &mut Criterion) {
    // Test with operator-heavy expressions
    let expressions = vec![
        "a + b * c - d / e ^ f",
        "(x + y) * (z - w) / (u + v)",
        "f[a, b, c] + g[x, y] * h[z]",
        "Plus[a, Times[b, c], Minus[d, Power[e, f]]]",
        "Apply[Plus, {1, 2, 3}] * Apply[Times, {4, 5, 6}]",
        "Map[Function[x, x + 1], {1, 2, 3, 4, 5}]",
        // Repeat common operators many times
        "+ + + + + + + + + +",
        "* * * * * * * * * *",
        "= = = = = = = = = =",
        "( ) ( ) ( ) ( ) ( )",
        "[ ] [ ] [ ] [ ] [ ]",
        "{ } { } { } { } { }",
    ];

    c.bench_function("tokenize_operators", |b| {
        b.iter(|| {
            for expr in &expressions {
                let tokens = tokenize(black_box(expr), &ParseOptions::default());
                black_box(tokens);
            }
        })
    });
}

fn bench_repeated_operators(c: &mut Criterion) {
    // Create an expression with many repeated operators
    let mut expr = String::new();
    for i in 0..100 {
        if i > 0 {
            expr.push_str(" + ");
        }
        expr.push_str(&format!("x{}", i));
    }
    
    c.bench_function("repeated_plus_operators", |b| {
        b.iter(|| {
            let tokens = tokenize(black_box(&expr), &ParseOptions::default());
            black_box(tokens);
        })
    });
}

fn bench_common_functions(c: &mut Criterion) {
    let expr = "Plus[Times[Power[a, b], c], Set[d, List[e, f, g]], Apply[Function[x, If[x > 0, x, 0]], data]]";
    
    c.bench_function("common_function_names", |b| {
        b.iter(|| {
            let tokens = tokenize(black_box(expr), &ParseOptions::default());
            black_box(tokens);
        })
    });
}

#[cfg(feature = "string-interning")]
fn bench_interning_vs_owned(_c: &mut Criterion) {
    // Note: InterningTokenInput is private, so we can't benchmark it directly
    // The main benefit is measured indirectly through the tokenization benchmarks
}

#[cfg(not(feature = "string-interning"))]
fn bench_interning_vs_owned(_c: &mut Criterion) {
    // No-op when string interning is not enabled
}

criterion_group!(
    benches,
    bench_tokenization_memory_usage,
    bench_repeated_operators,
    bench_common_functions,
    bench_interning_vs_owned
);
criterion_main!(benches);