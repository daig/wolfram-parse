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
    use crate::tests::test_strategies::*;
    use proptest::prelude::*;

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
