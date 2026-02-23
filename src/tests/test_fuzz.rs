//! Fuzz-style tests that verify the parser never panics on pathological inputs.
//!
//! These tests run a large corpus of tricky, malformed, and edge-case inputs
//! through all three pipeline stages (tokenize, CST, AST) and assert that none
//! of them panic. This provides a broad safety net for refactoring.
//!
//! Note: We use `_seq` variants (`parse_bytes_cst_seq`, `parse_bytes_ast_seq`)
//! because the scalar variants (`parse_cst`, `parse_ast`) intentionally panic
//! on multi-expression input via `expect_single_item()`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::{
    parse_bytes_ast_seq, parse_bytes_cst_seq, tokenize_bytes,
    source::SourceConvention,
    FirstLineBehavior, ParseOptions,
};

/// Run a single input through all three pipeline stages, catching any panics.
/// Returns a list of (stage_name, panic_message) for any stages that panicked.
fn check_no_panic(input: &[u8], opts: &ParseOptions) -> Vec<(&'static str, String)> {
    let mut failures = Vec::new();

    // Stage 1: tokenize
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = tokenize_bytes(input, opts);
    }));
    if let Err(e) = result {
        let msg = panic_message(&e);
        failures.push(("tokenize", msg));
    }

    // Stage 2: CST parse (sequence variant)
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse_bytes_cst_seq(input, opts);
    }));
    if let Err(e) = result {
        let msg = panic_message(&e);
        failures.push(("parse_cst_seq", msg));
    }

    // Stage 3: AST parse (sequence variant)
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse_bytes_ast_seq(input, opts);
    }));
    if let Err(e) = result {
        let msg = panic_message(&e);
        failures.push(("parse_ast_seq", msg));
    }

    failures
}

fn panic_message(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
    }
}

fn format_input(input: &[u8]) -> String {
    match std::str::from_utf8(input) {
        Ok(s) if s.len() <= 80 => format!("{s:?}"),
        Ok(s) => format!("{:?}...", &s[..77]),
        Err(_) => {
            if input.len() <= 40 {
                format!("{input:?}")
            } else {
                format!("{:?}...", &input[..37])
            }
        }
    }
}

/// Run the full corpus through all pipeline stages and report any panics.
fn run_corpus(corpus: &[(&[u8], &str)]) {
    let opts = ParseOptions::default();
    let mut all_failures: Vec<(String, String, Vec<(&str, String)>)> = Vec::new();

    for &(input, desc) in corpus {
        let failures = check_no_panic(input, &opts);
        if !failures.is_empty() {
            all_failures.push((
                desc.to_string(),
                format_input(input),
                failures,
            ));
        }
    }

    if !all_failures.is_empty() {
        let mut report = format!("\n{} input(s) caused panics:\n", all_failures.len());
        for (desc, input_repr, failures) in &all_failures {
            report.push_str(&format!("\n  [{desc}] input={input_repr}\n"));
            for (stage, msg) in failures {
                report.push_str(&format!("    - {stage}: {msg}\n"));
            }
        }
        panic!("{report}");
    }
}

// =========================================================================
// A. Empty and degenerate inputs
// =========================================================================

#[test]
fn test_fuzz_empty_and_degenerate() {
    run_corpus(&[
        (b"", "empty input"),
        (b" ", "single space"),
        (b"  ", "double space"),
        (b"\t", "tab"),
        (b"\n", "newline"),
        (b"\r", "carriage return"),
        (b"\r\n", "CRLF"),
        (b"\n\n\n", "multiple newlines"),
        (b"\t \t \t", "mixed whitespace"),
    ]);
}

// =========================================================================
// B. Unmatched and mismatched brackets
// =========================================================================

#[test]
fn test_fuzz_unmatched_brackets() {
    run_corpus(&[
        (b"(", "unclosed paren"),
        (b")", "unmatched close paren"),
        (b"[", "unclosed square"),
        (b"]", "unmatched close square"),
        (b"{", "unclosed curly"),
        (b"}", "unmatched close curly"),
        (b"(]", "paren-square mismatch"),
        (b"[}", "square-curly mismatch"),
        (b"{)", "curly-paren mismatch"),
        (b"(((", "triple unclosed paren"),
        (b")))", "triple unmatched close"),
        (b"([{", "mixed unclosed"),
        (b"}])", "mixed unmatched close"),
        (b"({[}])", "interleaved brackets"),
        (b"f[g[h[", "nested unclosed calls"),
        (b"f[[", "unclosed Part"),
        (b"f[[x]", "Part missing outer close"),
        (b"<|", "unclosed association"),
        (b"|>", "unmatched association close"),
        (b"<||>", "empty association"),
    ]);
}

// =========================================================================
// C. Unterminated constructs
// =========================================================================

#[test]
fn test_fuzz_unterminated() {
    run_corpus(&[
        (b"\"", "lone quote"),
        (b"\"hello", "unterminated string"),
        (b"\"hello\\", "unterminated string with backslash"),
        (b"(* comment", "unterminated comment"),
        (b"(* (* nested", "unterminated nested comment"),
        (b"(* (* inner *)", "one level unclosed comment"),
        (b"\"\\[", "string with unterminated escape"),
        (b"\\[Alpha", "unterminated long name"),
        (b"\\[", "bare backslash-bracket"),
        (b"\\:", "bare backslash-colon"),
        (b"\\.", "bare backslash-dot"),
        (b"\\|", "bare backslash-pipe"),
        (b"\"\\", "string ending with backslash"),
    ]);
}

// =========================================================================
// D. Number edge cases
// =========================================================================

#[test]
fn test_fuzz_numbers() {
    run_corpus(&[
        (b".", "lone dot"),
        (b"..", "double dot (Repeated)"),
        (b"...", "triple dot (RepeatedNull)"),
        (b"....", "four dots"),
        (b"0^^", "base with no digits"),
        (b".2^^0", "decimal base"),
        (b"12^^a.a", "base-12 with dot"),
        (b"12..", "number then Repeated"),
        (b"1e", "incomplete scientific"),
        (b"1e+", "incomplete scientific with sign"),
        (b"1e-", "incomplete scientific with neg sign"),
        (b"123\\\n.45", "line continuation in number"),
        (b"1`", "precision mark"),
        (b"1``", "accuracy mark"),
        (b"1`5", "precision 5"),
        (b"1``5", "accuracy 5"),
        (b"6`5..", "precision then repeated (historical crash)"),
        (b"0.0.0", "multiple dots in number"),
        (b"16^^ff", "hex number"),
        (b"2^^101", "binary number"),
        (b"36^^zz", "base-36 number"),
        (b"37^^a", "invalid base"),
    ]);
}

// =========================================================================
// E. Operator edge cases
// =========================================================================

#[test]
fn test_fuzz_operators() {
    run_corpus(&[
        (b"+", "lone plus"),
        (b"*", "lone star"),
        (b"/", "lone slash"),
        (b"-", "lone minus"),
        (b"!", "lone bang"),
        (b"@", "lone at"),
        (b"@@", "lone apply"),
        (b"@@@", "lone map-apply"),
        (b"<>", "lone string-join"),
        (b"||", "lone or"),
        (b"&&", "lone and"),
        (b"/;", "lone condition"),
        (b"->", "lone rule"),
        (b":>", "lone rule-delayed"),
        (b"=", "lone equal"),
        (b":=", "lone set-delayed"),
        (b"/:", "lone tag-set prefix"),
        (b"//", "lone postfix"),
        (b"&", "lone function"),
        (b"?", "lone pattern-test"),
        (b"|", "lone alternative"),
        (b"~", "lone tilde"),
        (b"^", "lone caret"),
        // NOTE: "<<rr[R" panics in abstract_cst.rs:503 (todo!("unhandled case"))
        // during AST abstraction. Tokenize and CST stages are fine.
        // This is a known bug — tracked separately. Tested below with CST-only.
        // (b"<<rr[R", "historical hang input"),
        (b"a + * b", "missing operand between ops"),
        (b"+ + +", "repeated prefix operators"),
        (b"a !! b", "factorial with operand"),
        (b"a // b", "postfix application"),
    ]);
}

// =========================================================================
// F. Character encoding edge cases
// =========================================================================

#[test]
fn test_fuzz_encoding() {
    run_corpus(&[
        // Invalid UTF-8
        (&[0xff], "single 0xff byte"),
        (&[0xfe], "single 0xfe byte"),
        (&[0x80], "lone continuation byte"),
        (&[0xc0, 0x80], "overlong encoding of NUL"),
        (&[0xe2, 0x9a], "truncated 3-byte UTF-8"),
        (&[0xf0, 0x9f], "truncated 4-byte UTF-8"),
        (&[0xed, 0xa0, 0x80], "UTF-8 encoded high surrogate"),
        (&[0xed, 0xbf, 0xbf], "UTF-8 encoded low surrogate"),
        // BOM
        (&[0xef, 0xbb, 0xbf], "UTF-8 BOM"),
        (&[0xef, 0xbb, 0xbf, b'1', b'+', b'2'], "BOM then expression"),
        // Mixed valid and invalid
        (&[b'1', b'+', 0xe2, 0x9a, b'1'], "invalid UTF-8 mid-expression"),
        (&[0xff, b'a', b'+', b'b'], "invalid byte then valid expression"),
        (&[b'a', b'+', b'b', 0xff], "valid expression then invalid byte"),
        // WL escape sequences
        (b"\\:feff", "WL hex escape BOM"),
        (b"\\0560", "WL octal escape"),
        (b"\\:03b1", "WL 4-hex escape for alpha"),
        (b"\\.f2", "WL 2-hex escape"),
        (b"\\|0000f2", "WL 6-hex escape"),
    ]);
}

// =========================================================================
// G. Escape sequence edge cases
// =========================================================================

#[test]
fn test_fuzz_escapes() {
    run_corpus(&[
        (b"\\[Alpha]", "valid long name"),
        (b"\\[Beta]", "valid long name beta"),
        (b"\\[NonExistent]", "invalid long name"),
        (b"\\[Alpa]", "misspelled long name"),
        (b"\\[Alpha+2", "unterminated long name"),
        (b"\\:03b+1", "truncated hex4 escape"),
        (b"\\.g1", "invalid hex2 digit"),
        (b"\\|00zz00", "invalid hex6 digit"),
        (b"\\999", "octal escape near limit"),
        (b"\\[RawWedge]", "raw named character"),
        (b"\\[RawTab]", "raw tab character"),
        (b"\"\\n\"", "newline escape in string"),
        (b"\"\\t\"", "tab escape in string"),
        (b"\"\\\\\"", "backslash escape in string"),
        (b"\"\\\"\"", "quote escape in string"),
    ]);
}

// =========================================================================
// H. Line continuations
// =========================================================================

#[test]
fn test_fuzz_line_continuations() {
    run_corpus(&[
        (b"1\\\n", "line continuation LF"),
        (b"1\\\r\n", "line continuation CRLF"),
        (b"1\\\r", "line continuation CR"),
        (b"\\\n", "bare line continuation"),
        (b"\\\n\\\n\\\n", "triple line continuation"),
        (b"a\\\nb", "continuation mid-symbol"),
        (b"1::\\\n*", "continuation in message name"),
        (b"1::*\\\r\n", "historical crash: msg name with CR-LF"),
    ]);
}

// =========================================================================
// I. Prefix/postfix without operands
// =========================================================================

#[test]
fn test_fuzz_prefix_postfix_bare() {
    run_corpus(&[
        (b"!!", "double bang"),
        (b"++", "bare increment"),
        (b"--", "bare decrement"),
        (b"#", "bare slot"),
        (b"##", "bare slot-sequence"),
        (b"#1", "slot 1"),
        (b"##2", "slot-sequence 2"),
        (b"%", "bare Out"),
        (b"%%", "double Out"),
        (b"%%%", "triple Out"),
        (b"_", "bare blank"),
        (b"__", "bare blank-sequence"),
        (b"___", "bare blank-null-sequence"),
        (b"_.", "bare default"),
    ]);
}

// =========================================================================
// J. Complex combinations
// =========================================================================

#[test]
fn test_fuzz_complex_combinations() {
    run_corpus(&[
        (b"a:b~1:2", "pattern with tilde (historical crash)"),
        (b"a /: b", "incomplete tag-set"),
        (b"a /: b[a] =.", "tag-unset"),
        (b"f[[x", "unterminated Part"),
        (b"a_..", "default then repeated"),
        (b"_?!", "blank pattern-test factorial"),
        (b"a + * b", "missing operand between operators"),
        (b";;", "bare span"),
        (b";;;", "bare double-span"),
        (b"a;;b;;c;;d", "multi-span"),
        (b"a~f~b~g~c", "chained tilde"),
        (b"a//@b//@c", "chained MapAll"),
        (b"a::b::c::d", "chained message names"),
        (b"a_b_c", "chained patterns"),
        (b"a:b:c:d", "chained colons"),
        (b"a<->b<->c", "chained two-way rules"),
        (b"#1 + ##2 & @ f", "slot expression with function"),
        (b"a /. b /. c", "chained replace-all"),
        (b"a //. b //. c", "chained replace-repeated"),
        (b"\\!\\(a\\)", "linear syntax"),
    ]);
}

// =========================================================================
// Known panics (documented bugs — tokenize and CST work, AST panics)
// =========================================================================

/// `<<rr[R` triggers a `todo!("unhandled case")` in abstract_cst.rs:503
/// during CST→AST abstraction. Tokenize and CST stages handle it fine.
/// This should be fixed as part of the panic audit (refactoring item #12).
#[test]
fn test_fuzz_known_panic_get_operator_ast() {
    let input = b"<<rr[R";
    let opts = ParseOptions::default();

    // Tokenize: OK
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = tokenize_bytes(input, &opts);
    }));
    assert!(result.is_ok(), "tokenize should not panic on <<rr[R");

    // CST: OK
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse_bytes_cst_seq(input, &opts);
    }));
    assert!(result.is_ok(), "parse_cst_seq should not panic on <<rr[R");

    // AST: Known panic — verify it still panics (so we know when it's fixed)
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse_bytes_ast_seq(input, &opts);
    }));
    assert!(
        result.is_err(),
        "<<rr[R AST panic is now fixed! Remove this test and add to main corpus."
    );
}

// =========================================================================
// K. Stress tests
// =========================================================================

/// NOTE: Deep nesting (100+) causes stack overflow due to unbounded recursion
/// in the Pratt parser. This is a known limitation inherited from the upstream
/// C++ implementation. The commented-out test_crashers.rs StackOverflow1 test
/// also documents this. We test with moderate depth (30) which is safe.
#[test]
fn test_fuzz_stress_nested_parens() {
    let depth = 30;
    let mut input = Vec::with_capacity(depth * 2);
    for _ in 0..depth {
        input.push(b'(');
    }
    for _ in 0..depth {
        input.push(b')');
    }
    run_corpus(&[(&input, "30 nested parens")]);
}

#[test]
fn test_fuzz_stress_nested_brackets() {
    let depth = 30;
    let mut input = Vec::with_capacity(depth * 3 + 1);
    input.push(b'f');
    for _ in 0..depth {
        input.extend_from_slice(b"[g");
    }
    for _ in 0..depth {
        input.push(b']');
    }
    run_corpus(&[(&input, "30 nested function calls")]);
}

#[test]
fn test_fuzz_stress_long_symbol() {
    let input: Vec<u8> = std::iter::repeat(b'a').take(10_000).collect();
    run_corpus(&[(&input, "10,000 char symbol")]);
}

#[test]
fn test_fuzz_stress_many_semicolons() {
    let mut input = Vec::with_capacity(3000);
    for _ in 0..1000 {
        input.extend_from_slice(b"1; ");
    }
    run_corpus(&[(&input, "1000 semicolon-separated expressions")]);
}

#[test]
fn test_fuzz_stress_repeated_operators() {
    let input: Vec<u8> = std::iter::repeat(b'+').take(100).collect();
    run_corpus(&[(&input, "100 consecutive plus signs")]);
}

#[test]
fn test_fuzz_stress_long_number() {
    let input: Vec<u8> = std::iter::repeat(b'1').take(10_000).collect();
    run_corpus(&[(&input, "10,000 digit number")]);
}

#[test]
fn test_fuzz_stress_implicit_times() {
    let mut input = Vec::with_capacity(1000);
    for _ in 0..500 {
        input.extend_from_slice(b"a ");
    }
    run_corpus(&[(&input, "500 implicit multiplications")]);
}

// =========================================================================
// L. Parse options variations
// =========================================================================

#[test]
fn test_fuzz_parse_options_variations() {
    let representative_inputs: &[&[u8]] = &[
        b"a + b * c",
        b"f[x, y]",
        b"{1, 2, 3}",
        b"\"hello world\"",
        b"a /: b = c",
        b"a; b; c",
        b"",
        b"(",
    ];

    let option_sets: Vec<(&str, ParseOptions)> = vec![
        (
            "CharacterIndex",
            ParseOptions::default()
                .source_convention(SourceConvention::CharacterIndex),
        ),
        (
            "FirstLineBehavior::Check (no shebang)",
            ParseOptions::default()
                .first_line_behavior(FirstLineBehavior::Check),
        ),
        (
            "tab_width(1)",
            ParseOptions::default().tab_width(1),
        ),
        (
            "tab_width(8)",
            ParseOptions::default().tab_width(8),
        ),
    ];

    for (opt_desc, opts) in &option_sets {
        for input in representative_inputs {
            let failures = check_no_panic(input, opts);
            if !failures.is_empty() {
                let input_repr = format_input(input);
                let mut report = format!(
                    "\nPanic with options={opt_desc}, input={input_repr}\n"
                );
                for (stage, msg) in &failures {
                    report.push_str(&format!("  - {stage}: {msg}\n"));
                }
                panic!("{report}");
            }
        }
    }
}

#[test]
fn test_fuzz_shebang_variations() {
    let opts = ParseOptions::default()
        .first_line_behavior(FirstLineBehavior::Check);

    run_corpus(&[
        (b"#!/usr/bin/env wolfram\n1+2", "shebang then expression"),
        (b"#!/usr/bin/env wolfram\n", "shebang then empty"),
        (b"#!/usr/bin/env wolfram", "shebang without newline"),
        (b"#!", "bare shebang prefix"),
    ]);

    // Also test these with the Check option
    let shebang_inputs: &[(&[u8], &str)] = &[
        (b"#!/usr/bin/env wolfram\n1+2", "shebang+Check: expression"),
        (b"1+2", "no-shebang+Check: expression"),
        (b"", "empty+Check"),
    ];

    for &(input, desc) in shebang_inputs {
        let failures = check_no_panic(input, &opts);
        assert!(
            failures.is_empty(),
            "Panic in [{desc}]: {failures:?}"
        );
    }
}
