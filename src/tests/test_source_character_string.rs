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
    use crate::tests::test_strategies::*;
    use proptest::prelude::*;

    // --- Trivia injection strategies (specific to source-character tests) ---

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
