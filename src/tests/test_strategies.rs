//! Shared proptest strategies for generating Wolfram Language atoms and
//! full-form expressions.  Used by test_full_form_string,
//! test_input_form_string, and test_source_character_string.

use proptest::prelude::*;

pub fn arb_symbol_str() -> BoxedStrategy<String> {
    "[a-zA-Z][a-zA-Z0-9]{0,5}".prop_map(|s| s).boxed()
}

pub fn arb_integer_str() -> BoxedStrategy<String> {
    (0u32..1000).prop_map(|n| n.to_string()).boxed()
}

pub fn arb_real_str() -> BoxedStrategy<String> {
    (0u32..1000, 1u32..1000)
        .prop_map(|(int, frac)| format!("{int}.{frac}"))
        .boxed()
}

pub fn arb_negative_integer_str() -> BoxedStrategy<String> {
    (1u32..1000).prop_map(|n| format!("-{n}")).boxed()
}

pub fn arb_negative_real_str() -> BoxedStrategy<String> {
    (0u32..1000, 1u32..1000)
        .prop_map(|(int, frac)| format!("-{int}.{frac}"))
        .boxed()
}

pub fn arb_rational_str() -> BoxedStrategy<String> {
    (1u32..100, 1u32..10)
        .prop_map(|(m, e)| format!("{m}*^-{e}"))
        .boxed()
}

pub fn arb_string_literal_str() -> BoxedStrategy<String> {
    "[a-zA-Z0-9 ]{0,10}"
        .prop_map(|s| format!("\"{s}\""))
        .boxed()
}

pub fn arb_string_with_escapes_str() -> BoxedStrategy<String> {
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

pub fn arb_base_integer_str() -> BoxedStrategy<String> {
    prop_oneof![
        "[01]{1,8}".prop_map(|d| format!("2^^{d}")).boxed(),
        "[0-7]{1,6}".prop_map(|d| format!("8^^{d}")).boxed(),
        "[0-9a-f]{1,4}".prop_map(|d| format!("16^^{d}")).boxed(),
    ]
    .boxed()
}

pub fn arb_positive_sci_str() -> BoxedStrategy<String> {
    (1u32..1000, 1u32..10)
        .prop_map(|(m, e)| format!("{m}*^{e}"))
        .boxed()
}

pub fn arb_atom_str() -> BoxedStrategy<String> {
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

pub fn arb_full_form_expr() -> BoxedStrategy<String> {
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
