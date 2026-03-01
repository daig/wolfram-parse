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

// ============================================================================
// Edge-case tests not covered by exhaustive or proptest suites
// ============================================================================

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
fn test_negative_literal_head() {
    // a // -1 means (-1)[a]; negative literals need parens as call heads
    assert_eq!(full_form("a // -1"), "(-1)[a]");
    assert_eq!(full_form("a // -1.5"), "(-1.5)[a]");
}

#[test]
fn test_negative_literals() {
    // The parser folds prefix-minus into the literal (not Times[-1, n]).
    // Only -x (non-literal operand) produces Times[-1, x].
    assert_eq!(full_form("-5"), "-5");
    assert_eq!(full_form("-3.14"), "-3.14");
    assert_eq!(full_form("-0"), "0"); // special case: -0 folds to 0
    assert_eq!(full_form("f[-5]"), "f[-5]");
    assert_eq!(full_form("-5 + 3"), "Plus[-5, 3]");
    assert_eq!(full_form("1 - 5"), "Plus[1, -5]");
}

// ============================================================================
// Exhaustive sugar → full-form correctness test
// ============================================================================

/// Verify that every ASCII-syntax operator maps to the correct full-form name,
/// and that the full-form output itself round-trips (stability).
#[test]
fn test_sugar_to_full_form_exhaustive() {
    let cases: &[(&str, &str)] = &[
        // === Arithmetic & math ===
        ("a + b", "Plus[a, b]"),
        ("a - b", "Plus[a, Times[-1, b]]"),
        ("a * b", "Times[a, b]"),
        ("a b", "Times[a, b]"),             // implicit times
        ("a / b", "Times[a, Power[b, -1]]"),
        ("a ^ b", "Power[a, b]"),
        ("a . b", "Dot[a, b]"),
        ("a ** b", "NonCommutativeMultiply[a, b]"),

        // === Comparison ===
        ("a == b", "Equal[a, b]"),
        ("a != b", "Unequal[a, b]"),
        ("a < b", "Less[a, b]"),
        ("a > b", "Greater[a, b]"),
        ("a <= b", "LessEqual[a, b]"),
        ("a >= b", "GreaterEqual[a, b]"),
        ("a === b", "SameQ[a, b]"),
        ("a =!= b", "UnsameQ[a, b]"),

        // === Logic & alternatives ===
        ("a && b", "And[a, b]"),
        ("a || b", "Or[a, b]"),
        ("!a", "Not[a]"),
        ("a | b", "Alternatives[a, b]"),

        // === String operators ===
        ("a <> b", "StringJoin[a, b]"),
        ("a ~~ b", "StringExpression[a, b]"),

        // === Pattern & blank ===
        ("_", "Blank[]"),
        ("__", "BlankSequence[]"),
        ("___", "BlankNullSequence[]"),
        ("_Integer", "Blank[Integer]"),
        ("_.", "Optional[Blank[]]"),
        ("x_", "Pattern[x, Blank[]]"),
        ("x__", "Pattern[x, BlankSequence[]]"),
        ("x___", "Pattern[x, BlankNullSequence[]]"),
        ("x_Integer", "Pattern[x, Blank[Integer]]"),
        ("x_?f", "PatternTest[Pattern[x, Blank[]], f]"),
        ("x_:0", "Optional[Pattern[x, Blank[]], 0]"),
        ("x:f[a]", "Pattern[x, f[a]]"),

        // === Slot, SlotSequence, Out ===
        ("#", "Slot[1]"),
        ("#2", "Slot[2]"),
        ("##", "SlotSequence[1]"),
        ("##3", "SlotSequence[3]"),
        ("%", "Out[]"),
        ("%%", "Out[-2]"),

        // === Assignment & unset ===
        ("x = 1", "Set[x, 1]"),
        ("x := a", "SetDelayed[x, a]"),
        ("x ^= a", "UpSet[x, a]"),
        ("x ^:= a", "UpSetDelayed[x, a]"),
        ("x =.", "Unset[x]"),
        ("x += 1", "AddTo[x, 1]"),
        ("x -= 1", "SubtractFrom[x, 1]"),
        ("x *= 2", "TimesBy[x, 2]"),
        ("x /= 2", "DivideBy[x, 2]"),

        // === Rule ===
        ("a -> b", "Rule[a, b]"),
        ("a :> b", "RuleDelayed[a, b]"),
        ("a <-> b", "TwoWayRule[a, b]"),

        // === Application, map, postfix/prefix call ===
        ("f @ x", "f[x]"),                  // prefix application
        ("x // f", "f[x]"),                 // postfix application
        ("a ~ f ~ b", "f[a, b]"),           // infix application
        ("f /@ x", "Map[f, x]"),
        ("f @@ x", "Apply[f, x]"),
        ("f @@@ x", "MapApply[f, x]"),
        ("f //@ x", "MapAll[f, x]"),

        // === Replacement & condition ===
        ("x /. y", "ReplaceAll[x, y]"),
        ("x //. y", "ReplaceRepeated[x, y]"),
        ("x /; y", "Condition[x, y]"),

        // === Prefix operators ===
        ("-x", "Times[-1, x]"),
        ("++x", "PreIncrement[x]"),
        ("--x", "PreDecrement[x]"),

        // === Postfix operators ===
        ("x!", "Factorial[x]"),
        ("x!!", "Factorial2[x]"),
        ("x++", "Increment[x]"),
        ("x--", "Decrement[x]"),
        ("x &", "Function[x]"),
        ("f'", "Derivative[1][f]"),
        ("f''", "Derivative[2][f]"),
        ("x..", "Repeated[x]"),
        ("x...", "RepeatedNull[x]"),

        // === Structure ===
        ("{a, b}", "List[a, b]"),
        ("<|a -> 1|>", "Association[Rule[a, 1]]"),
        ("a[[1]]", "Part[a, 1]"),
        ("a[[1, 2]]", "Part[a, 1, 2]"),
        ("a[[1;;3]]", "Part[a, Span[1, 3]]"),
        ("a[[;;]]", "Part[a, Span[1, All]]"),
        ("a[[2;;]]", "Part[a, Span[2, All]]"),
        ("a ; b", "CompoundExpression[a, b]"),
        ("a ;", "CompoundExpression[a, Null]"),

        // === Ternary / tag ===
        ("a /: b = c", "TagSet[a, b, c]"),
        ("a /: b := c", "TagSetDelayed[a, b, c]"),
        ("a /: b =.", "TagUnset[a, b]"),

        // === MessageName ===
        ("sym::tag", r#"MessageName[sym, "tag"]"#),

        // === File I/O & introspection ===
        ("?z", "Information[z]"),
        (r#"<<"file""#, r#"Get["file"]"#),
        (r#"x >> "file""#, r#"Put[x, "file"]"#),
        (r#"x >>> "file""#, r#"PutAppend[x, "file"]"#),
        ("+x", "Plus[x]"),
    ];

    for &(input, expected) in cases {
        // 1. Correctness: sugar produces expected full form
        let result = crate::parse_ast(input, &ParseOptions::default());
        let ff = result
            .syntax
            .to_full_form_string()
            .unwrap_or_else(|e| panic!("serialize failed for {input:?}: {e}"));
        assert_eq!(ff, expected, "sugar={input:?}");

        // 2. Stability: full-form output itself round-trips
        let result2 = crate::parse_ast(&ff, &ParseOptions::default());
        let ff2 = result2
            .syntax
            .to_full_form_string()
            .unwrap_or_else(|e| {
                panic!("re-serialize of {input:?} → {ff:?} failed: {e}")
            });
        assert_eq!(ff, ff2, "stability failed for sugar={input:?}");
    }
}

// ============================================================================
// Property-based tests (proptest)
// ============================================================================

mod proptest_full_form {
    use super::*;
    use crate::tests::test_strategies::*;
    use proptest::prelude::*;

    // --- AST-level leaf strategies (specific to full-form tests) ---

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

    fn arb_negative_integer() -> BoxedStrategy<Ast> {
        (1u32..1000) // exclude 0: -0 normalizes to 0
            .prop_map(|n| Ast::Leaf {
                kind: TokenKind::Integer,
                input: TokenString::from_string(format!("-{n}")),
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

    fn arb_negative_real() -> BoxedStrategy<Ast> {
        (0u32..1000, 1u32..1000)
            .prop_map(|(int, frac)| Ast::Leaf {
                kind: TokenKind::Real,
                input: TokenString::from_string(format!("-{int}.{frac}")),
                data: AstMetadata::empty(),
            })
            .boxed()
    }

    fn arb_rational() -> BoxedStrategy<Ast> {
        (1u32..100, 1u32..10)
            .prop_map(|(mantissa, exp)| Ast::Leaf {
                kind: TokenKind::Rational,
                input: TokenString::from_string(format!("{mantissa}*^-{exp}")),
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

    fn arb_string_with_escapes() -> BoxedStrategy<Ast> {
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
        .prop_map(|parts| {
            let content = parts.join("");
            Ast::Leaf {
                kind: TokenKind::String,
                input: TokenString::from_string(format!("\"{content}\"")),
                data: AstMetadata::empty(),
            }
        })
        .boxed()
    }

    fn arb_base_integer() -> BoxedStrategy<Ast> {
        prop_oneof![
            "[01]{1,8}".prop_map(|d| format!("2^^{d}")).boxed(),
            "[0-7]{1,6}".prop_map(|d| format!("8^^{d}")).boxed(),
            "[0-9a-f]{1,4}".prop_map(|d| format!("16^^{d}")).boxed(),
        ]
        .prop_map(|s| Ast::Leaf {
            kind: TokenKind::Integer,
            input: TokenString::from_string(s),
            data: AstMetadata::empty(),
        })
        .boxed()
    }

    fn arb_positive_sci_integer() -> BoxedStrategy<Ast> {
        (1u32..1000, 1u32..10)
            .prop_map(|(mantissa, exp)| Ast::Leaf {
                kind: TokenKind::Integer,
                input: TokenString::from_string(format!("{mantissa}*^{exp}")),
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
            1 => arb_negative_integer(),
            1 => arb_real(),
            1 => arb_negative_real(),
            1 => arb_rational(),
            1 => arb_base_integer(),
            1 => arb_positive_sci_integer(),
            1 => arb_string_literal(),
            1 => arb_string_with_escapes(),
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
