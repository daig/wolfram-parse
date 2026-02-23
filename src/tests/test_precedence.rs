//! Comprehensive tests for operator precedence and associativity.
//!
//! These tests verify that the Pratt parser correctly resolves operator priority
//! (which operator binds tighter) and associativity (how repeated operators of
//! the same precedence nest or flatten).
//!
//! Key design rules in Wolfram Language's CST:
//! - "Non-right" (left-associative / flat) operators produce flat `InfixNode`
//!   with all operands as siblings: `a + b + c` → InfixNode(Plus, [a, +, b, +, c])
//! - "Right" associative operators produce nested `BinaryNode`:
//!   `a ^ b ^ c` → BinaryNode(Power, [a, ^, BinaryNode(Power, [b, ^, c])])
//! - Division (`/`) is a BinaryNode, not flat Infix like Times
//! - Comparisons (`<`, `>`) produce InfixNode(CodeParser_InfixInequality)
//! - Application (`@`) produces BinaryNode(CodeParser_BinaryAt)
//!
//! Precedence values (from `src/generated/precedence/values.rs`):
//!   Semi=2, Set=4, Rule=15, Condition=17, Alternatives=21, And=31,
//!   Or=29, Inequality=38, Plus=43, Times=54, Divide=61, PrefixMinus=62,
//!   Power=75, StringJoin=76, Factorial=79, Apply=80, At=82, PatternTest=89

use crate::{
    cst::{
        BinaryNode, CallBody, CallHead, CallNode,
        Cst::{Binary, Call, Infix, Postfix, Prefix, Token},
        GroupNode, InfixNode, OperatorNode, PostfixNode, PrefixNode,
    },
    macros::{src, token},
    parse::operators::{
        BinaryOperator as BinaryOp, CallOperator as CallOp, InfixOperator as InfixOp,
        PostfixOperator as PostfixOp, PrefixOperator as PrefixOp,
    },
    parse_cst, parse_cst_seq,
    tests::assert_src,
    NodeSeq,
};

use pretty_assertions::assert_eq;

// =========================================================================
// A. Precedence ordering
// =========================================================================

/// Times (prec 54) binds tighter than Plus (prec 43):
/// `a + b * c` → Plus(a, Times(b, c))
#[test]
fn test_precedence_arithmetic() {
    assert_eq!(
        parse_cst("a + b * c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Plus, "+", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::Times,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Star, "*", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "c", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Same precedence test, operands reversed: `a * b + c` → Plus(Times(a, b), c)
#[test]
fn test_precedence_arithmetic_reversed() {
    assert_eq!(
        parse_cst("a * b + c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::Times,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(Star, "*", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Symbol, "b", 1:5-6)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Plus, "+", 1:7-8)),
                Token(token!(Whitespace, " ", 1:8-9)),
                Token(token!(Symbol, "c", 1:9-10)),
            ]),
        })))
    );
}

/// Power (prec 75) binds tighter than Times (prec 54):
/// `a * b ^ c` → Times(a, Power(b, c))
#[test]
fn test_precedence_power_vs_times() {
    assert_eq!(
        parse_cst("a * b ^ c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Star, "*", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Power,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Caret, "^", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "c", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Divide (prec 61) binds tighter than Plus (prec 43).
/// Note: Division is a BinaryNode, not flat Infix like Times.
#[test]
fn test_precedence_divide_vs_plus() {
    assert_eq!(
        parse_cst("a + b / c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Plus, "+", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Divide,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Slash, "/", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "c", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// And (prec 31) binds tighter than Or (prec 29):
/// `a || b && c` → Or(a, And(b, c))
#[test]
fn test_precedence_and_vs_or() {
    assert_eq!(
        parse_cst("a || b && c", &Default::default()).syntax,
        assert_src!(1:1-12 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Or,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(BarBar, "||", 1:3-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                assert_src!(1:6-12 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::And,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:6-7)),
                        Token(token!(Whitespace, " ", 1:7-8)),
                        Token(token!(AmpAmp, "&&", 1:8-10)),
                        Token(token!(Whitespace, " ", 1:10-11)),
                        Token(token!(Symbol, "c", 1:11-12)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Not prefix (prec 32) binds tighter than And (prec 31):
/// `!a && b` → And(Not(a), b)
#[test]
fn test_precedence_not_vs_and() {
    assert_eq!(
        parse_cst("!a && b", &Default::default()).syntax,
        assert_src!(1:1-8 => Infix(InfixNode(OperatorNode {
            op: InfixOp::And,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Prefix(PrefixNode(OperatorNode {
                    op: PrefixOp::Not,
                    children: NodeSeq(vec![
                        Token(token!(Bang, "!", 1:1-2)),
                        Token(token!(Symbol, "a", 1:2-3)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(AmpAmp, "&&", 1:4-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Symbol, "b", 1:7-8)),
            ]),
        })))
    );
}

/// Inequality (prec 38) binds tighter than And (prec 31):
/// `a < b && c > d` → And(Inequality(a, <, b), Inequality(c, >, d))
/// Note: `<` and `>` produce CodeParser_InfixInequality, not Less/Greater.
#[test]
fn test_precedence_comparison_vs_logic() {
    assert_eq!(
        parse_cst("a < b && c > d", &Default::default()).syntax,
        assert_src!(1:1-15 => Infix(InfixNode(OperatorNode {
            op: InfixOp::And,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::CodeParser_InfixInequality,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(Less, "<", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Symbol, "b", 1:5-6)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(AmpAmp, "&&", 1:7-9)),
                Token(token!(Whitespace, " ", 1:9-10)),
                assert_src!(1:10-15 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::CodeParser_InfixInequality,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "c", 1:10-11)),
                        Token(token!(Whitespace, " ", 1:11-12)),
                        Token(token!(Greater, ">", 1:12-13)),
                        Token(token!(Whitespace, " ", 1:13-14)),
                        Token(token!(Symbol, "d", 1:14-15)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Plus (prec 43) binds tighter than Rule (prec 15):
/// `a + b -> c + d` → Rule(Plus(a, b), Plus(c, d))
#[test]
fn test_precedence_rule_vs_plus() {
    assert_eq!(
        parse_cst("a + b -> c + d", &Default::default()).syntax,
        assert_src!(1:1-15 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Rule,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::Plus,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(Plus, "+", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Symbol, "b", 1:5-6)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(MinusGreater, "->", 1:7-9)),
                Token(token!(Whitespace, " ", 1:9-10)),
                assert_src!(1:10-15 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::Plus,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "c", 1:10-11)),
                        Token(token!(Whitespace, " ", 1:11-12)),
                        Token(token!(Plus, "+", 1:12-13)),
                        Token(token!(Whitespace, " ", 1:13-14)),
                        Token(token!(Symbol, "d", 1:14-15)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Plus (prec 43) binds tighter than Set/assignment (prec 4):
/// `x = a + b` → Set(x, Plus(a, b))
#[test]
fn test_precedence_assignment_vs_plus() {
    assert_eq!(
        parse_cst("x = a + b", &Default::default()).syntax,
        assert_src!(1:1-10 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Set,
            children: NodeSeq(vec![
                Token(token!(Symbol, "x", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Equal, "=", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::Plus,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Plus, "+", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "b", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Factorial postfix (prec 79) binds tighter than Times (prec 54):
/// `a! * b` → Times(Factorial(a), b)
#[test]
fn test_precedence_postfix_vs_times() {
    assert_eq!(
        parse_cst("a! * b", &Default::default()).syntax,
        assert_src!(1:1-7 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Postfix(PostfixNode(OperatorNode {
                    op: PostfixOp::Factorial,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Bang, "!", 1:2-3)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(Star, "*", 1:4-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
            ]),
        })))
    );
}

/// Factorial postfix (prec 79) binds tighter than Power (prec 75):
/// `a!^b` → Power(Factorial(a), b)
#[test]
fn test_precedence_power_vs_postfix() {
    assert_eq!(
        parse_cst("a!^b", &Default::default()).syntax,
        assert_src!(1:1-5 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Power,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Postfix(PostfixNode(OperatorNode {
                    op: PostfixOp::Factorial,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Bang, "!", 1:2-3)),
                    ]),
                }))),
                Token(token!(Caret, "^", 1:3-4)),
                Token(token!(Symbol, "b", 1:4-5)),
            ]),
        })))
    );
}

/// Application @ (prec 82) binds tighter than Times (prec 54):
/// `f @ a * b` → Times(BinaryAt(f, a), b)
#[test]
fn test_precedence_application_vs_times() {
    assert_eq!(
        parse_cst("f @ a * b", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::CodeParser_BinaryAt,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "f", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(At, "@", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Symbol, "a", 1:5-6)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Star, "*", 1:7-8)),
                Token(token!(Whitespace, " ", 1:8-9)),
                Token(token!(Symbol, "b", 1:9-10)),
            ]),
        })))
    );
}

/// PatternTest ? (prec 89) binds tighter than Alternatives | (prec 21):
/// `a?f | b` → Alternatives(PatternTest(a, f), b)
#[test]
fn test_precedence_pattern_test_vs_alternatives() {
    assert_eq!(
        parse_cst("a?f | b", &Default::default()).syntax,
        assert_src!(1:1-8 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Alternatives,
            children: NodeSeq(vec![
                assert_src!(1:1-4 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::PatternTest,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Question, "?", 1:2-3)),
                        Token(token!(Symbol, "f", 1:3-4)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:4-5)),
                Token(token!(Bar, "|", 1:5-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Symbol, "b", 1:7-8)),
            ]),
        })))
    );
}

/// Condition /; (prec 17) binds tighter than Rule -> (prec 15):
/// `a /; b -> c` → Rule(Condition(a, b), c)
#[test]
fn test_precedence_condition_vs_rule() {
    assert_eq!(
        parse_cst("a /; b -> c", &Default::default()).syntax,
        assert_src!(1:1-12 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Rule,
            children: NodeSeq(vec![
                assert_src!(1:1-7 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Condition,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(SlashSemi, "/;", 1:3-5)),
                        Token(token!(Whitespace, " ", 1:5-6)),
                        Token(token!(Symbol, "b", 1:6-7)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:7-8)),
                Token(token!(MinusGreater, "->", 1:8-10)),
                Token(token!(Whitespace, " ", 1:10-11)),
                Token(token!(Symbol, "c", 1:11-12)),
            ]),
        })))
    );
}

/// StringJoin <> (prec 76) binds tighter than Plus (prec 43):
/// `a <> b + c` → Plus(StringJoin(a, b), c)
#[test]
fn test_precedence_string_join_vs_plus() {
    assert_eq!(
        parse_cst("a <> b + c", &Default::default()).syntax,
        assert_src!(1:1-11 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                assert_src!(1:1-7 => Infix(InfixNode(OperatorNode {
                    op: InfixOp::StringJoin,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(LessGreater, "<>", 1:3-5)),
                        Token(token!(Whitespace, " ", 1:5-6)),
                        Token(token!(Symbol, "b", 1:6-7)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:7-8)),
                Token(token!(Plus, "+", 1:8-9)),
                Token(token!(Whitespace, " ", 1:9-10)),
                Token(token!(Symbol, "c", 1:10-11)),
            ]),
        })))
    );
}

/// Set (prec 4) binds tighter than Semi/CompoundExpression (prec 2):
/// `a = b; c` → CompoundExpression(Set(a, b), c)
#[test]
fn test_precedence_semicolon_vs_assignment() {
    let result = parse_cst_seq("a = b; c", &Default::default());
    let NodeSeq(nodes) = result.syntax;
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0],
        assert_src!(1:1-9 => Infix(InfixNode(OperatorNode {
            op: InfixOp::CompoundExpression,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Set,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(Equal, "=", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Symbol, "b", 1:5-6)),
                    ]),
                }))),
                Token(token!(Semi, ";", 1:6-7)),
                Token(token!(Whitespace, " ", 1:7-8)),
                Token(token!(Symbol, "c", 1:8-9)),
            ]),
        })))
    );
}

/// Power (prec 75) binds tighter than prefix Minus (prec 62):
/// `-a ^ b` → PrefixMinus(Power(a, b))
/// This matches Mathematica's behavior: -a^b means -(a^b), not (-a)^b.
#[test]
fn test_precedence_prefix_minus_vs_power() {
    assert_eq!(
        parse_cst("-a ^ b", &Default::default()).syntax,
        assert_src!(1:1-7 => Prefix(PrefixNode(OperatorNode {
            op: PrefixOp::Minus,
            children: NodeSeq(vec![
                Token(token!(Minus, "-", 1:1-2)),
                assert_src!(1:2-7 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Power,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:2-3)),
                        Token(token!(Whitespace, " ", 1:3-4)),
                        Token(token!(Caret, "^", 1:4-5)),
                        Token(token!(Whitespace, " ", 1:5-6)),
                        Token(token!(Symbol, "b", 1:6-7)),
                    ]),
                }))),
            ]),
        })))
    );
}

// =========================================================================
// B. Associativity
// =========================================================================

/// Plus is flat (non-right associative) — all operands at the same level:
/// `a + b + c` → InfixNode(Plus, [a, +, b, +, c])
#[test]
fn test_associativity_plus_left_flat() {
    assert_eq!(
        parse_cst("a + b + c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Plus, "+", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                Token(token!(Symbol, "b", 1:5-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Plus, "+", 1:7-8)),
                Token(token!(Whitespace, " ", 1:8-9)),
                Token(token!(Symbol, "c", 1:9-10)),
            ]),
        })))
    );
}

/// Times is flat (non-right associative):
/// `a * b * c` → InfixNode(Times, [a, *, b, *, c])
#[test]
fn test_associativity_times_left_flat() {
    assert_eq!(
        parse_cst("a * b * c", &Default::default()).syntax,
        assert_src!(1:1-10 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Star, "*", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                Token(token!(Symbol, "b", 1:5-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Star, "*", 1:7-8)),
                Token(token!(Whitespace, " ", 1:8-9)),
                Token(token!(Symbol, "c", 1:9-10)),
            ]),
        })))
    );
}

/// Power is right-associative:
/// `a ^ b ^ c` → BinaryNode(Power, [a, ^, BinaryNode(Power, [b, ^, c])])
#[test]
fn test_associativity_power_right() {
    assert_eq!(
        parse_cst("a ^ b ^ c", &Default::default()).syntax,
        assert_src!(1:1-10 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Power,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Caret, "^", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Power,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Caret, "^", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "c", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Rule is right-associative:
/// `a -> b -> c` → BinaryNode(Rule, [a, ->, BinaryNode(Rule, [b, ->, c])])
#[test]
fn test_associativity_rule_right() {
    assert_eq!(
        parse_cst("a -> b -> c", &Default::default()).syntax,
        assert_src!(1:1-12 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Rule,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(MinusGreater, "->", 1:3-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                assert_src!(1:6-12 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Rule,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:6-7)),
                        Token(token!(Whitespace, " ", 1:7-8)),
                        Token(token!(MinusGreater, "->", 1:8-10)),
                        Token(token!(Whitespace, " ", 1:10-11)),
                        Token(token!(Symbol, "c", 1:11-12)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Set (=) is right-associative:
/// `a = b = c` → BinaryNode(Set, [a, =, BinaryNode(Set, [b, =, c])])
#[test]
fn test_associativity_set_right() {
    assert_eq!(
        parse_cst("a = b = c", &Default::default()).syntax,
        assert_src!(1:1-10 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Set,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Equal, "=", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Set,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(Equal, "=", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "c", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Apply (@@) is right-associative:
/// `f @@ g @@ x` → BinaryNode(Apply, [f, @@, BinaryNode(Apply, [g, @@, x])])
#[test]
fn test_associativity_apply_right() {
    assert_eq!(
        parse_cst("f @@ g @@ x", &Default::default()).syntax,
        assert_src!(1:1-12 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::Apply,
            children: NodeSeq(vec![
                Token(token!(Symbol, "f", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(AtAt, "@@", 1:3-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                assert_src!(1:6-12 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Apply,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "g", 1:6-7)),
                        Token(token!(Whitespace, " ", 1:7-8)),
                        Token(token!(AtAt, "@@", 1:8-10)),
                        Token(token!(Whitespace, " ", 1:10-11)),
                        Token(token!(Symbol, "x", 1:11-12)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// And (&&) is flat (non-right associative):
/// `a && b && c` → InfixNode(And, [a, &&, b, &&, c])
#[test]
fn test_associativity_and_left_flat() {
    assert_eq!(
        parse_cst("a && b && c", &Default::default()).syntax,
        assert_src!(1:1-12 => Infix(InfixNode(OperatorNode {
            op: InfixOp::And,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(AmpAmp, "&&", 1:3-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
                Token(token!(Whitespace, " ", 1:7-8)),
                Token(token!(AmpAmp, "&&", 1:8-10)),
                Token(token!(Whitespace, " ", 1:10-11)),
                Token(token!(Symbol, "c", 1:11-12)),
            ]),
        })))
    );
}

/// Or (||) is flat (non-right associative):
/// `a || b || c` → InfixNode(Or, [a, ||, b, ||, c])
#[test]
fn test_associativity_or_left_flat() {
    assert_eq!(
        parse_cst("a || b || c", &Default::default()).syntax,
        assert_src!(1:1-12 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Or,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(BarBar, "||", 1:3-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
                Token(token!(Whitespace, " ", 1:7-8)),
                Token(token!(BarBar, "||", 1:8-10)),
                Token(token!(Whitespace, " ", 1:10-11)),
                Token(token!(Symbol, "c", 1:11-12)),
            ]),
        })))
    );
}

/// @ (application) is right-associative:
/// `f @ g @ x` → BinaryAt(f, BinaryAt(g, x))
#[test]
fn test_associativity_application_right() {
    assert_eq!(
        parse_cst("f @ g @ x", &Default::default()).syntax,
        assert_src!(1:1-10 => Binary(BinaryNode(OperatorNode {
            op: BinaryOp::CodeParser_BinaryAt,
            children: NodeSeq(vec![
                Token(token!(Symbol, "f", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(At, "@", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                assert_src!(1:5-10 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::CodeParser_BinaryAt,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "g", 1:5-6)),
                        Token(token!(Whitespace, " ", 1:6-7)),
                        Token(token!(At, "@", 1:7-8)),
                        Token(token!(Whitespace, " ", 1:8-9)),
                        Token(token!(Symbol, "x", 1:9-10)),
                    ]),
                }))),
            ]),
        })))
    );
}

// =========================================================================
// C. Prefix / Postfix interactions
// =========================================================================

/// Prefix minus has lower precedence than Plus:
/// `-a + b` → Plus(Minus(a), b)
#[test]
fn test_prefix_in_infix() {
    assert_eq!(
        parse_cst("-a + b", &Default::default()).syntax,
        assert_src!(1:1-7 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Prefix(PrefixNode(OperatorNode {
                    op: PrefixOp::Minus,
                    children: NodeSeq(vec![
                        Token(token!(Minus, "-", 1:1-2)),
                        Token(token!(Symbol, "a", 1:2-3)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(Plus, "+", 1:4-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
            ]),
        })))
    );
}

/// Postfix factorial has higher precedence than Plus:
/// `a! + b` → Plus(Factorial(a), b)
#[test]
fn test_postfix_in_infix() {
    assert_eq!(
        parse_cst("a! + b", &Default::default()).syntax,
        assert_src!(1:1-7 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Plus,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Postfix(PostfixNode(OperatorNode {
                    op: PostfixOp::Factorial,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Bang, "!", 1:2-3)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(Plus, "+", 1:4-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
            ]),
        })))
    );
}

/// Nested prefix minus: `- -a` → Minus(Minus(a))
/// Note: whitespace is included in the outer prefix's children.
#[test]
fn test_double_prefix() {
    assert_eq!(
        parse_cst("- -a", &Default::default()).syntax,
        assert_src!(1:1-5 => Prefix(PrefixNode(OperatorNode {
            op: PrefixOp::Minus,
            children: NodeSeq(vec![
                Token(token!(Minus, "-", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                assert_src!(1:3-5 => Prefix(PrefixNode(OperatorNode {
                    op: PrefixOp::Minus,
                    children: NodeSeq(vec![
                        Token(token!(Minus, "-", 1:3-4)),
                        Token(token!(Symbol, "a", 1:4-5)),
                    ]),
                }))),
            ]),
        })))
    );
}

/// Prefix minus in multiplication: `-a * b` → Times(Minus(a), b)
#[test]
fn test_prefix_minus_times() {
    assert_eq!(
        parse_cst("-a * b", &Default::default()).syntax,
        assert_src!(1:1-7 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                assert_src!(1:1-3 => Prefix(PrefixNode(OperatorNode {
                    op: PrefixOp::Minus,
                    children: NodeSeq(vec![
                        Token(token!(Minus, "-", 1:1-2)),
                        Token(token!(Symbol, "a", 1:2-3)),
                    ]),
                }))),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(Star, "*", 1:4-5)),
                Token(token!(Whitespace, " ", 1:5-6)),
                Token(token!(Symbol, "b", 1:6-7)),
            ]),
        })))
    );
}

/// PreIncrement wraps the entire call expression:
/// `++a[b]` → PreIncrement(Call(a, [b]))
#[test]
fn test_increment_vs_call() {
    assert_eq!(
        parse_cst("++a[b]", &Default::default()).syntax,
        assert_src!(1:1-7 => Prefix(PrefixNode(OperatorNode {
            op: PrefixOp::PreIncrement,
            children: NodeSeq(vec![
                Token(token!(PlusPlus, "++", 1:1-3)),
                assert_src!(1:3-7 => Call(CallNode {
                    head: CallHead::Concrete(NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:3-4)),
                    ])),
                    body: assert_src!(1:4-7 => CallBody::Group(GroupNode(OperatorNode {
                        op: CallOp::CodeParser_GroupSquare,
                        children: NodeSeq(vec![
                            Token(token!(OpenSquare, "[", 1:4-5)),
                            Token(token!(Symbol, "b", 1:5-6)),
                            Token(token!(CloseSquare, "]", 1:6-7)),
                        ]),
                    }))),
                })),
            ]),
        })))
    );
}

// =========================================================================
// D. Implicit multiplication
// =========================================================================

/// Adjacent symbols produce implicit Times with a Fake_ImplicitTimes token:
/// `a b` → InfixNode(Times, [a, Whitespace, ImplicitTimes, b])
#[test]
fn test_implicit_times_basic() {
    assert_eq!(
        parse_cst("a b", &Default::default()).syntax,
        assert_src!(1:1-4 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Fake_ImplicitTimes, "", 1:3-3)),
                Token(token!(Symbol, "b", 1:3-4)),
            ]),
        })))
    );
}

/// Implicit and explicit Times flatten together:
/// `a b * c` → InfixNode(Times, [a, ImplicitTimes, b, *, c])
#[test]
fn test_implicit_times_with_explicit() {
    assert_eq!(
        parse_cst("a b * c", &Default::default()).syntax,
        assert_src!(1:1-8 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Fake_ImplicitTimes, "", 1:3-3)),
                Token(token!(Symbol, "b", 1:3-4)),
                Token(token!(Whitespace, " ", 1:4-5)),
                Token(token!(Star, "*", 1:5-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Symbol, "c", 1:7-8)),
            ]),
        })))
    );
}

/// Number-symbol adjacency produces implicit Times:
/// `2 x` → InfixNode(Times, [2, ImplicitTimes, x])
#[test]
fn test_implicit_times_number_symbol() {
    assert_eq!(
        parse_cst("2 x", &Default::default()).syntax,
        assert_src!(1:1-4 => Infix(InfixNode(OperatorNode {
            op: InfixOp::Times,
            children: NodeSeq(vec![
                Token(token!(Integer, "2", 1:1-2)),
                Token(token!(Whitespace, " ", 1:2-3)),
                Token(token!(Fake_ImplicitTimes, "", 1:3-3)),
                Token(token!(Symbol, "x", 1:3-4)),
            ]),
        })))
    );
}

// =========================================================================
// E. Compound expressions
// =========================================================================

/// CompoundExpression (;) is flat:
/// `a; b; c` → InfixNode(CompoundExpression, [a, ;, b, ;, c])
#[test]
fn test_compound_semicolon_flat() {
    let result = parse_cst_seq("a; b; c", &Default::default());
    let NodeSeq(nodes) = result.syntax;
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0],
        assert_src!(1:1-8 => Infix(InfixNode(OperatorNode {
            op: InfixOp::CompoundExpression,
            children: NodeSeq(vec![
                Token(token!(Symbol, "a", 1:1-2)),
                Token(token!(Semi, ";", 1:2-3)),
                Token(token!(Whitespace, " ", 1:3-4)),
                Token(token!(Symbol, "b", 1:4-5)),
                Token(token!(Semi, ";", 1:5-6)),
                Token(token!(Whitespace, " ", 1:6-7)),
                Token(token!(Symbol, "c", 1:7-8)),
            ]),
        })))
    );
}

/// CompoundExpression is the loosest operator (after Comma):
/// `a = 1; b = 2` → CompoundExpression(Set(a, 1), Set(b, 2))
#[test]
fn test_compound_semicolon_lowest() {
    let result = parse_cst_seq("a = 1; b = 2", &Default::default());
    let NodeSeq(nodes) = result.syntax;
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0],
        assert_src!(1:1-13 => Infix(InfixNode(OperatorNode {
            op: InfixOp::CompoundExpression,
            children: NodeSeq(vec![
                assert_src!(1:1-6 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Set,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "a", 1:1-2)),
                        Token(token!(Whitespace, " ", 1:2-3)),
                        Token(token!(Equal, "=", 1:3-4)),
                        Token(token!(Whitespace, " ", 1:4-5)),
                        Token(token!(Integer, "1", 1:5-6)),
                    ]),
                }))),
                Token(token!(Semi, ";", 1:6-7)),
                Token(token!(Whitespace, " ", 1:7-8)),
                assert_src!(1:8-13 => Binary(BinaryNode(OperatorNode {
                    op: BinaryOp::Set,
                    children: NodeSeq(vec![
                        Token(token!(Symbol, "b", 1:8-9)),
                        Token(token!(Whitespace, " ", 1:9-10)),
                        Token(token!(Equal, "=", 1:10-11)),
                        Token(token!(Whitespace, " ", 1:11-12)),
                        Token(token!(Integer, "2", 1:12-13)),
                    ]),
                }))),
            ]),
        })))
    );
}
