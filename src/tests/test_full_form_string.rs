use crate::{
    ast::{AstMetadata, Ast, ToFullFormError},
    tokenize::{TokenKind, TokenString},
    ParseOptions,
};

fn full_form(input: &str) -> String {
    let result = crate::parse_ast(input, &ParseOptions::default());
    result
        .syntax
        .to_full_form_string()
        .unwrap_or_else(|e| panic!("to_full_form_string failed for {input:?}: {e}"))
}

#[test]
fn test_addition() {
    assert_eq!(full_form("1+1"), "Plus[1, 1]");
}

#[test]
fn test_multiplication() {
    assert_eq!(full_form("a*b"), "Times[a, b]");
}

#[test]
fn test_already_full_form() {
    assert_eq!(full_form("f[x, y]"), "f[x, y]");
}

#[test]
fn test_nested_call() {
    assert_eq!(full_form("f[g[x]]"), "f[g[x]]");
}

#[test]
fn test_negation() {
    // -x parses as Times[-1, x]
    assert_eq!(full_form("-x"), "Times[-1, x]");
}

#[test]
fn test_factorial() {
    assert_eq!(full_form("x!"), "Factorial[x]");
}

#[test]
fn test_string_literal() {
    assert_eq!(full_form(r#"{"a", "b"}"#), r#"List["a", "b"]"#);
}

#[test]
fn test_power() {
    assert_eq!(full_form("x^2"), "Power[x, 2]");
}

#[test]
fn test_compound_expression() {
    // a;b parses as CompoundExpression[a, b]
    assert_eq!(full_form("a;b"), "CompoundExpression[a, b]");
}

#[test]
fn test_compound_expression_trailing() {
    // a; parses as CompoundExpression[a, Null]
    assert_eq!(full_form("a;"), "CompoundExpression[a, Null]");
}

#[test]
fn test_negative_literal_head() {
    // a // -1 means (-1)[a]
    assert_eq!(full_form("a // -1"), "(-1)[a]");
}

#[test]
fn test_list() {
    assert_eq!(full_form("{1, 2, 3}"), "List[1, 2, 3]");
}

#[test]
fn test_rule() {
    assert_eq!(full_form("a -> b"), "Rule[a, b]");
}

#[test]
fn test_set() {
    assert_eq!(full_form("x = 1"), "Set[x, 1]");
}

#[test]
fn test_part() {
    assert_eq!(full_form("a[[1]]"), "Part[a, 1]");
}

#[test]
fn test_slot() {
    // # is syntactic sugar for Slot[1] (implicit one)
    assert_eq!(full_form("#"), "Slot[1]");
}

#[test]
fn test_no_args_call() {
    assert_eq!(full_form("f[]"), "f[]");
}

#[test]
fn test_error_node() {
    // Unterminated string produces an error node
    let result = crate::parse_ast("\"\n", &ParseOptions::default());
    assert_eq!(
        result.syntax.to_full_form_string(),
        Err(ToFullFormError::ErrorNode)
    );
}

#[test]
fn test_implicit_times() {
    // "a b" parses as Times[a, b] with implicit multiplication
    assert_eq!(full_form("a b"), "Times[a, b]");
}

// ============================================================================
// Property-based tests (proptest)
// ============================================================================

mod proptest_full_form {
    use super::*;
    use proptest::prelude::*;

    // --- Leaf strategies ---

    fn arb_symbol() -> BoxedStrategy<Ast> {
        "[a-zA-Z][a-zA-Z0-9]{0,5}"
            .prop_map(|s| Ast::Leaf {
                kind: TokenKind::Symbol,
                input: TokenString::from_string(s),
                data: AstMetadata::empty(),
            })
            .boxed()
    }

    fn arb_integer() -> BoxedStrategy<Ast> {
        (0u32..1000)
            .prop_map(|n| Ast::Leaf {
                kind: TokenKind::Integer,
                input: TokenString::from_string(n.to_string()),
                data: AstMetadata::empty(),
            })
            .boxed()
    }

    fn arb_real() -> BoxedStrategy<Ast> {
        (0u32..1000, 1u32..1000)
            .prop_map(|(int, frac)| Ast::Leaf {
                kind: TokenKind::Real,
                input: TokenString::from_string(format!("{int}.{frac}")),
                data: AstMetadata::empty(),
            })
            .boxed()
    }

    fn arb_string_literal() -> BoxedStrategy<Ast> {
        "[a-zA-Z0-9 ]{0,10}"
            .prop_map(|s| Ast::Leaf {
                kind: TokenKind::String,
                input: TokenString::from_string(format!("\"{s}\"")),
                data: AstMetadata::empty(),
            })
            .boxed()
    }

    fn arb_fake_implicit_null() -> BoxedStrategy<Ast> {
        Just(Ast::Leaf {
            kind: TokenKind::Fake_ImplicitNull,
            input: TokenString::from_string(String::new()),
            data: AstMetadata::empty(),
        })
        .boxed()
    }

    fn arb_fake_implicit_one() -> BoxedStrategy<Ast> {
        Just(Ast::Leaf {
            kind: TokenKind::Fake_ImplicitOne,
            input: TokenString::from_string(String::new()),
            data: AstMetadata::empty(),
        })
        .boxed()
    }

    fn arb_fake_implicit_all() -> BoxedStrategy<Ast> {
        Just(Ast::Leaf {
            kind: TokenKind::Fake_ImplicitAll,
            input: TokenString::from_string(String::new()),
            data: AstMetadata::empty(),
        })
        .boxed()
    }

    fn arb_ast_leaf() -> BoxedStrategy<Ast> {
        prop_oneof![
            3 => arb_symbol(),
            2 => arb_integer(),
            1 => arb_real(),
            1 => arb_string_literal(),
            1 => arb_fake_implicit_null(),
            1 => arb_fake_implicit_one(),
            1 => arb_fake_implicit_all(),
        ]
        .boxed()
    }

    // --- Recursive AST strategy ---

    fn arb_ast() -> BoxedStrategy<Ast> {
        arb_ast_leaf()
            .prop_recursive(
                4,  // max depth
                64, // desired node count
                8,  // expected items per collection
                |inner| {
                    (inner.clone(), prop::collection::vec(inner, 0..=4)).prop_map(
                        |(head, args)| Ast::Call {
                            head: Box::new(head),
                            args,
                            data: AstMetadata::empty(),
                        },
                    )
                },
            )
            .boxed()
    }

    // --- String-level strategies (for Property 2) ---

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

    fn arb_string_literal_str() -> BoxedStrategy<String> {
        "[a-zA-Z0-9 ]{0,10}"
            .prop_map(|s| format!("\"{s}\""))
            .boxed()
    }

    fn arb_atom_str() -> BoxedStrategy<String> {
        prop_oneof![
            3 => arb_symbol_str(),
            2 => arb_integer_str(),
            1 => arb_real_str(),
            1 => arb_string_literal_str(),
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

        /// Property 1: AST stability (double serialization).
        /// serialize(ast) == serialize(parse(serialize(ast)))
        #[test]
        fn proptest_ast_stability(ast in arb_ast()) {
            let s1 = ast
                .to_full_form_string()
                .expect("generated AST should serialize");
            let parsed = crate::parse_ast(&s1, &ParseOptions::default());
            let s2 = parsed
                .syntax
                .to_full_form_string()
                .unwrap_or_else(|e| {
                    panic!("re-parse of {s1:?} failed to serialize: {e}")
                });
            prop_assert_eq!(s1, s2);
        }

        /// Property 2 (atoms): parse(atom) round-trips through serialize.
        #[test]
        fn proptest_full_form_atom_round_trip(s in arb_atom_str()) {
            let parsed = crate::parse_ast(&s, &ParseOptions::default());
            let s2 = parsed
                .syntax
                .to_full_form_string()
                .unwrap_or_else(|e| {
                    panic!("parse of {s:?} failed to serialize: {e}")
                });
            prop_assert_eq!(s, s2);
        }

        /// Property 2 (recursive): parse(expr) round-trips through serialize.
        #[test]
        fn proptest_full_form_round_trip(s in arb_full_form_expr()) {
            let parsed = crate::parse_ast(&s, &ParseOptions::default());
            let s2 = parsed
                .syntax
                .to_full_form_string()
                .unwrap_or_else(|e| {
                    panic!("parse of {s:?} failed to serialize: {e}")
                });
            prop_assert_eq!(s, s2);
        }
    }
}
