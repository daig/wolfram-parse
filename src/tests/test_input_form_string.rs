use crate::{
    cst::Cst,
    tokenize::TokenStr,
    NodeSeq, ParseOptions,
};

/// Parse and aggregate a string into a sequence of aggregated CST nodes.
fn aggregate(input: &str) -> Vec<Cst<TokenStr>> {
    let cst_seq = crate::parse_cst_seq(input, &ParseOptions::default()).syntax;
    let NodeSeq(nodes) = crate::abstract_cst::aggregate_cst_seq(cst_seq);
    nodes
}

// ============================================================================
// Property-based tests (proptest)
// ============================================================================

mod proptest_input_form {
    use super::*;
    use proptest::prelude::*;

    // --- String-level strategies ---
    // (Duplicated from test_full_form_string; full-form strings are valid WL
    //  input, so they serve as our generator domain.)

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

    fn arb_full_form_expr() -> BoxedStrategy<String> {
        arb_atom_str()
            .prop_recursive(
                4,  // max depth
                64, // desired node count
                8,  // expected items per collection
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

    // --- Property tests ---

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property A — Stability (idempotence):
        /// to_input_form(aggregate(parse(to_input_form(aggregate(parse(s))))))
        ///   == to_input_form(aggregate(parse(s)))
        ///
        /// The second serialization is byte-identical to the first.
        #[test]
        fn proptest_input_form_stability(s in arb_full_form_expr()) {
            let nodes1 = aggregate(&s);

            // First serialization
            let mut parts1 = Vec::new();
            for node in &nodes1 {
                parts1.push(
                    node.to_input_form_string()
                        .unwrap_or_else(|e| panic!(
                            "first to_input_form_string failed for {s:?}: {e}"
                        )),
                );
            }
            let input_form1 = parts1.join("");

            // Re-parse and re-serialize
            let nodes2 = aggregate(&input_form1);
            let mut parts2 = Vec::new();
            for node in &nodes2 {
                parts2.push(
                    node.to_input_form_string()
                        .unwrap_or_else(|e| panic!(
                            "second to_input_form_string failed for {input_form1:?} \
                             (original: {s:?}): {e}"
                        )),
                );
            }
            let input_form2 = parts2.join("");

            prop_assert_eq!(
                input_form1, input_form2,
                "stability failed for input={:?}", s
            );
        }

        /// Property B — Semantic preservation (consistency with full form):
        /// to_full_form(parse_ast(to_input_form(aggregate(parse_cst(s)))))
        ///   == to_full_form(parse_ast(s))
        ///
        /// Converting to input form and re-parsing produces the same abstract
        /// meaning (as measured by full-form serialization).
        #[test]
        fn proptest_input_form_semantic_preservation(s in arb_full_form_expr()) {
            // Original full-form
            let original_ast = crate::parse_ast(&s, &ParseOptions::default());
            let original_ff = original_ast
                .syntax
                .to_full_form_string()
                .unwrap_or_else(|e| panic!(
                    "original full form failed for {s:?}: {e}"
                ));

            // CST → aggregate → input form → re-parse AST → full form
            let nodes = aggregate(&s);
            let mut parts = Vec::new();
            for node in &nodes {
                parts.push(
                    node.to_input_form_string()
                        .unwrap_or_else(|e| panic!(
                            "to_input_form_string failed for {s:?}: {e}"
                        )),
                );
            }
            let input_form = parts.join("");

            let reparsed_ast = crate::parse_ast(&input_form, &ParseOptions::default());
            let reparsed_ff = reparsed_ast
                .syntax
                .to_full_form_string()
                .unwrap_or_else(|e| panic!(
                    "re-parsed full form failed for input_form={input_form:?} \
                     (original: {s:?}): {e}"
                ));

            prop_assert_eq!(
                original_ff, reparsed_ff,
                "semantic preservation failed: input={:?}, input_form={:?}", s, input_form
            );
        }
    }
}
