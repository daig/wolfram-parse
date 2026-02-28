use crate::{
    cst::CstSeq,
    tokenize::TokenStr,
    ParseOptions,
};

/// Parse a string into a concrete CST sequence (preserving all trivia).
fn parse_concrete(input: &str) -> CstSeq<TokenStr> {
    crate::parse_cst_seq(input, &ParseOptions::default()).syntax
}

// ============================================================================
// Property-based tests (proptest)
// ============================================================================

mod proptest_source_char {
    use super::*;
    use proptest::prelude::*;

    // --- Atom strategies (reused from test_input_form_string) ---

    fn arb_symbol_str() -> BoxedStrategy<String> {
        "[a-zA-Z][a-zA-Z0-9]{0,5}".prop_map(|s| s).boxed()
    }

    fn arb_integer_str() -> BoxedStrategy<String> {
        (0u32..1000).prop_map(|n| n.to_string()).boxed()
    }

    fn arb_real_str() -> BoxedStrategy<String> {
        (0u32..1000, 1u32..1000)
            .prop_map(|(int, frac)| format!("{int}.{frac}"))
            .boxed()
    }

    fn arb_negative_integer_str() -> BoxedStrategy<String> {
        (1u32..1000).prop_map(|n| format!("-{n}")).boxed()
    }

    fn arb_negative_real_str() -> BoxedStrategy<String> {
        (0u32..1000, 1u32..1000)
            .prop_map(|(int, frac)| format!("-{int}.{frac}"))
            .boxed()
    }

    fn arb_rational_str() -> BoxedStrategy<String> {
        (1u32..100, 1u32..10)
            .prop_map(|(m, e)| format!("{m}*^-{e}"))
            .boxed()
    }

    fn arb_string_literal_str() -> BoxedStrategy<String> {
        "[a-zA-Z0-9 ]{0,10}"
            .prop_map(|s| format!("\"{s}\""))
            .boxed()
    }

    fn arb_string_with_escapes_str() -> BoxedStrategy<String> {
        prop::collection::vec(
            prop_oneof![
                4 => "[a-zA-Z0-9 ]".prop_map(|s: String| s).boxed(),
                1 => Just(r"\n".to_string()).boxed(),
                1 => Just(r"\t".to_string()).boxed(),
                1 => Just(r"\\".to_string()).boxed(),
                1 => Just(r#"\""#.to_string()).boxed(),
            ],
            0..=6,
        )
        .prop_map(|parts| format!("\"{}\"", parts.join("")))
        .boxed()
    }

    fn arb_base_integer_str() -> BoxedStrategy<String> {
        prop_oneof![
            "[01]{1,8}".prop_map(|d| format!("2^^{d}")).boxed(),
            "[0-7]{1,6}".prop_map(|d| format!("8^^{d}")).boxed(),
            "[0-9a-f]{1,4}".prop_map(|d| format!("16^^{d}")).boxed(),
        ]
        .boxed()
    }

    fn arb_positive_sci_str() -> BoxedStrategy<String> {
        (1u32..1000, 1u32..10)
            .prop_map(|(m, e)| format!("{m}*^{e}"))
            .boxed()
    }

    fn arb_atom_str() -> BoxedStrategy<String> {
        prop_oneof![
            3 => arb_symbol_str(),
            2 => arb_integer_str(),
            1 => arb_negative_integer_str(),
            1 => arb_real_str(),
            1 => arb_negative_real_str(),
            1 => arb_rational_str(),
            1 => arb_base_integer_str(),
            1 => arb_positive_sci_str(),
            1 => arb_string_literal_str(),
            1 => arb_string_with_escapes_str(),
        ]
        .boxed()
    }

    // --- Trivia injection strategies ---

    /// Optional whitespace: empty, single space, or multiple spaces.
    fn arb_ws() -> BoxedStrategy<String> {
        prop_oneof![
            3 => Just(String::new()),
            2 => Just(" ".to_string()),
            1 => Just("  ".to_string()),
        ]
        .boxed()
    }

    /// Optional comment.
    fn arb_comment() -> BoxedStrategy<String> {
        prop_oneof![
            4 => Just(String::new()),
            1 => "[a-zA-Z0-9 ]{0,6}".prop_map(|s| format!("(*{s}*)")).boxed(),
        ]
        .boxed()
    }

    /// Optional trivia (whitespace and/or comment) between tokens.
    fn arb_trivia() -> BoxedStrategy<String> {
        (arb_ws(), arb_comment(), arb_ws())
            .prop_map(|(w1, c, w2)| format!("{w1}{c}{w2}"))
            .boxed()
    }

    // --- Full-form expression strategies (clean, no extra trivia) ---

    fn arb_full_form_expr() -> BoxedStrategy<String> {
        arb_atom_str()
            .prop_recursive(
                4,
                64,
                8,
                |inner| {
                    (arb_symbol_str(), prop::collection::vec(inner, 0..=4)).prop_map(
                        |(head, args)| {
                            if args.is_empty() {
                                format!("{head}[]")
                            } else {
                                format!("{head}[{}]", args.join(", "))
                            }
                        },
                    )
                },
            )
            .boxed()
    }

    // --- Source expression strategies (with trivia) ---

    fn arb_source_expr_with_trivia() -> BoxedStrategy<String> {
        arb_atom_str()
            .prop_recursive(
                4,
                64,
                8,
                |inner| {
                    (
                        arb_symbol_str(),
                        arb_trivia(),
                        prop::collection::vec(
                            (inner, arb_trivia()),
                            0..=4,
                        ),
                    ).prop_map(|(head, t_before_bracket, args)| {
                        if args.is_empty() {
                            format!("{head}{t_before_bracket}[]")
                        } else {
                            let parts: Vec<String> = args
                                .into_iter()
                                .map(|(arg, t)| format!("{arg}{t}"))
                                .collect();
                            format!(
                                "{head}{t_before_bracket}[{}]",
                                parts.join(", ")
                            )
                        }
                    })
                },
            )
            .boxed()
    }

    /// Combined generator: mix of clean and trivia-injected expressions.
    fn arb_source_expr() -> BoxedStrategy<String> {
        prop_oneof![
            2 => arb_full_form_expr(),
            3 => arb_source_expr_with_trivia(),
        ]
        .boxed()
    }

    // --- Property tests ---

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Exact round-trip identity:
        /// to_source_character_string(parse_cst(s)) == s
        ///
        /// Concrete CSTs are lossless, so serializing back to source characters
        /// must produce the exact original string.
        #[test]
        fn proptest_source_character_string_roundtrip(s in arb_source_expr()) {
            let cst = parse_concrete(&s);
            let result = cst.to_source_character_string()
                .unwrap_or_else(|e| panic!(
                    "to_source_character_string failed for {s:?}: {e}"
                ));
            prop_assert_eq!(
                &s, &result,
                "round-trip failed for input={:?}", s
            );
        }
    }
}
