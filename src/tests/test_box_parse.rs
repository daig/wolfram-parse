//! Tests for RowBox parsing (`box_parse` module).

use wolfram_expr::{Expr, Symbol};

use crate::{
    abstract_cst::{abstract_cst, aggregate_cst},
    ast::Ast,
    box_parse::parse_box_expr,
    cst::{
        BinaryNode, BoxKind, BoxNode, CallBody, CallHead, CallNode, Cst,
        GroupNode, InfixNode, OperatorNode, PostfixNode, PrefixNode,
    },
    parse::operators::{
        BinaryOperator, CallOperator, GroupOperator, InfixOperator,
        PostfixOperator, PrefixOperator,
    },
    quirks::QuirkSettings,
    source::Source,
    tokenize::{TokenKind as TK, TokenString},
    NodeSeq,
};

/// Helper: create a RowBox Expr from a list of string elements.
fn row_box(elements: Vec<Expr>) -> Expr {
    Expr::normal(
        Symbol::new("System`RowBox"),
        vec![Expr::normal(
            Symbol::new("System`List"),
            elements,
        )],
    )
}

/// Helper: create a string Expr.
fn s(val: &str) -> Expr {
    Expr::string(val)
}

/// Check that the resulting CST is an InfixNode with the given operator.
fn assert_infix_op(cst: &Cst<TokenString, Source>, expected: InfixOperator) {
    match cst {
        Cst::Infix(InfixNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, expected, "Expected infix op {:?}, got {:?}", expected, op);
        },
        other => panic!("Expected InfixNode, got: {:?}", other),
    }
}

/// Check that the resulting CST is a BinaryNode with the given operator.
fn assert_binary_op(cst: &Cst<TokenString, Source>, expected: BinaryOperator) {
    match cst {
        Cst::Binary(BinaryNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, expected, "Expected binary op {:?}, got {:?}", expected, op);
        },
        other => panic!("Expected BinaryNode, got: {:?}", other),
    }
}

/// Check that the resulting CST is a PrefixNode with the given operator.
fn assert_prefix_op(cst: &Cst<TokenString, Source>, expected: PrefixOperator) {
    match cst {
        Cst::Prefix(PrefixNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, expected, "Expected prefix op {:?}, got {:?}", expected, op);
        },
        other => panic!("Expected PrefixNode, got: {:?}", other),
    }
}

/// Check that the resulting CST is a PostfixNode with the given operator.
fn assert_postfix_op(
    cst: &Cst<TokenString, Source>,
    expected: PostfixOperator,
) {
    match cst {
        Cst::Postfix(PostfixNode(OperatorNode { op, .. })) => {
            assert_eq!(
                *op, expected,
                "Expected postfix op {:?}, got {:?}",
                expected, op
            );
        },
        other => panic!("Expected PostfixNode, got: {:?}", other),
    }
}

//==========================================================
// Tests
//==========================================================

#[test]
fn test_simple_plus() {
    // RowBox[{"2", "+", "3"}] → InfixNode[Plus, ...]
    let expr = row_box(vec![s("2"), s("+"), s("3")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Plus);

    // Check children include the operands and operator
    if let Cst::Infix(InfixNode(OperatorNode { children: NodeSeq(ref ch), .. })) = cst {
        // Should have 3 children (no whitespace)
        assert_eq!(ch.len(), 3);
        // First and last should be Integer tokens
        assert_eq!(token_kind(&ch[0]), Some(TK::Integer));
        assert_eq!(token_kind(&ch[1]), Some(TK::Plus));
        assert_eq!(token_kind(&ch[2]), Some(TK::Integer));
    }
}

#[test]
fn test_implicit_times() {
    // RowBox[{"2", " ", "x"}] → InfixNode[Times, ...{Fake_ImplicitTimes}...]
    let expr = row_box(vec![s("2"), s(" "), s("x")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Times);

    // Check that Fake_ImplicitTimes token is present
    if let Cst::Infix(InfixNode(OperatorNode { children: NodeSeq(ref ch), .. })) = cst {
        let has_implicit_times = ch.iter().any(|c| token_kind(c) == Some(TK::Fake_ImplicitTimes));
        assert!(has_implicit_times, "Should contain Fake_ImplicitTimes token");
    }
}

#[test]
fn test_function_call() {
    // RowBox[{"f", "[", "x", "]"}] → CallNode[{f}, GroupSquare[...]]
    let expr = row_box(vec![s("f"), s("["), s("x"), s("]")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Call(CallNode { head, body }) => {
            // Head should be the symbol "f"
            match head {
                CallHead::Aggregate(h) => {
                    assert_eq!(token_kind(h), Some(TK::Symbol));
                },
                CallHead::Concrete(NodeSeq(elts)) => {
                    assert_eq!(elts.len(), 1);
                    assert_eq!(token_kind(&elts[0]), Some(TK::Symbol));
                },
            }

            // Body should be GroupSquare
            match body {
                CallBody::Group(GroupNode(OperatorNode { op, .. })) => {
                    assert_eq!(*op, CallOperator::CodeParser_GroupSquare);
                },
                _ => panic!("Expected GroupSquare body"),
            }
        },
        other => panic!("Expected CallNode, got: {:?}", other),
    }
}

#[test]
fn test_group_list() {
    // RowBox[{"{", "a", ",", "b", "}"}] → GroupNode[List, ...]
    let expr = row_box(vec![s("{"), s("a"), s(","), s("b"), s("}")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Group(GroupNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, GroupOperator::List);
        },
        other => panic!("Expected GroupNode[List], got: {:?}", other),
    }
}

#[test]
fn test_group_paren() {
    // RowBox[{"(", "x", ")"}] → GroupNode[GroupParen, ...]
    let expr = row_box(vec![s("("), s("x"), s(")")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Group(GroupNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, GroupOperator::CodeParser_GroupParen);
        },
        other => panic!("Expected GroupNode[GroupParen], got: {:?}", other),
    }
}

#[test]
fn test_group_association() {
    // RowBox[{"<|", "a", "->", "1", "|>"}]
    let expr = row_box(vec![s("<|"), s("a"), s("->"), s("1"), s("|>")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Group(GroupNode(OperatorNode { op, .. })) => {
            assert_eq!(*op, GroupOperator::Association);
        },
        other => panic!("Expected GroupNode[Association], got: {:?}", other),
    }
}

#[test]
fn test_binary_power() {
    // RowBox[{"x", "^", "2"}] → BinaryNode[Power, ...]
    let expr = row_box(vec![s("x"), s("^"), s("2")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Power);
}

#[test]
fn test_binary_set() {
    // RowBox[{"x", "=", "5"}]
    let expr = row_box(vec![s("x"), s("="), s("5")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Set);
}

#[test]
fn test_binary_set_delayed() {
    // RowBox[{"f", ":=", "x"}]
    let expr = row_box(vec![s("f"), s(":="), s("x")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::SetDelayed);
}

#[test]
fn test_binary_rule() {
    // RowBox[{"x", "->", "y"}]
    let expr = row_box(vec![s("x"), s("->"), s("y")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Rule);
}

#[test]
fn test_binary_replace_all() {
    // RowBox[{"expr", "/.", "rules"}]
    let expr = row_box(vec![s("expr"), s("/."), s("rules")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::ReplaceAll);
}

#[test]
fn test_prefix_minus() {
    // RowBox[{"-", "x"}] → PrefixNode[Minus, ...]
    let expr = row_box(vec![s("-"), s("x")]);
    let cst = parse_box_expr(&expr);
    assert_prefix_op(&cst, PrefixOperator::Minus);
}

#[test]
fn test_prefix_not() {
    // RowBox[{"!", "x"}]
    let expr = row_box(vec![s("!"), s("x")]);
    let cst = parse_box_expr(&expr);
    assert_prefix_op(&cst, PrefixOperator::Not);
}

#[test]
fn test_postfix_factorial() {
    // RowBox[{"n", "!"}] → PostfixNode[Factorial, ...]
    let expr = row_box(vec![s("n"), s("!")]);
    let cst = parse_box_expr(&expr);
    assert_postfix_op(&cst, PostfixOperator::Factorial);
}

#[test]
fn test_postfix_function() {
    // RowBox[{"f", "&"}]
    let expr = row_box(vec![s("f"), s("&")]);
    let cst = parse_box_expr(&expr);
    assert_postfix_op(&cst, PostfixOperator::Function);
}

#[test]
fn test_postfix_derivative() {
    // RowBox[{"f", "'"}]
    let expr = row_box(vec![s("f"), s("'")]);
    let cst = parse_box_expr(&expr);
    assert_postfix_op(&cst, PostfixOperator::Derivative);
}

#[test]
fn test_single_element_splice() {
    // RowBox[{"x"}] → Just the `x` token (collapsed)
    let expr = row_box(vec![s("x")]);
    let cst = parse_box_expr(&expr);

    // Single element should be unwrapped, not wrapped in a RowBox
    assert_eq!(token_kind(&cst), Some(TK::Symbol));
    assert_eq!(token_input(&cst), Some("x"));
}

#[test]
fn test_nested_row_box() {
    // RowBox[{RowBox[{"a","+","b"}], "*", "c"}]
    let inner = row_box(vec![s("a"), s("+"), s("b")]);
    let expr = row_box(vec![inner, s("*"), s("c")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Times);

    // First agg child should be the nested InfixNode[Plus, ...]
    if let Cst::Infix(InfixNode(OperatorNode {
        children: NodeSeq(ref ch),
        ..
    })) = cst
    {
        // Find the first non-trivia child
        let first_sig = ch.iter().find(|c| !is_trivia(c)).unwrap();
        assert_infix_op(first_sig, InfixOperator::Plus);
    }
}

#[test]
fn test_with_superscript_box() {
    // RowBox[{SuperscriptBox["x","2"]}] → BoxNode[SuperscriptBox, ...]
    let sup_box = Expr::normal(
        Symbol::new("System`SuperscriptBox"),
        vec![Expr::string("x"), Expr::string("2")],
    );
    let expr = row_box(vec![sup_box]);
    let cst = parse_box_expr(&expr);

    // Single element gets unwrapped; should be a BoxNode[SuperscriptBox, ...]
    match &cst {
        Cst::Box(BoxNode { kind, .. }) => {
            assert_eq!(*kind, BoxKind::SuperscriptBox);
        },
        other => panic!("Expected BoxNode[SuperscriptBox], got: {:?}", other),
    }
}

#[test]
fn test_whitespace_preserved() {
    // RowBox[{"a"," ","+"," ","b"}] → InfixNode[Plus, ...]
    // with whitespace tokens included in children
    let expr = row_box(vec![s("a"), s(" "), s("+"), s(" "), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Plus);

    if let Cst::Infix(InfixNode(OperatorNode {
        children: NodeSeq(ref ch),
        ..
    })) = cst
    {
        // Should have 5 children including whitespace
        assert_eq!(ch.len(), 5);
        assert_eq!(token_kind(&ch[1]), Some(TK::Boxes_MultiWhitespace));
        assert_eq!(token_kind(&ch[3]), Some(TK::Boxes_MultiWhitespace));
    }
}

#[test]
fn test_compound_expr() {
    // RowBox[{"a", ";", "b"}] → InfixNode[CompoundExpression, ...]
    let expr = row_box(vec![s("a"), s(";"), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::CompoundExpression);
}

#[test]
fn test_comma_infix() {
    // RowBox[{"a", ",", "b", ",", "c"}] → InfixNode[Comma, ...]
    let expr = row_box(vec![s("a"), s(","), s("b"), s(","), s("c")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::CodeParser_Comma);
}

#[test]
fn test_alternatives() {
    // RowBox[{"a", "|", "b", "|", "c"}] → InfixNode[Alternatives, ...]
    let expr = row_box(vec![s("a"), s("|"), s("b"), s("|"), s("c")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Alternatives);
}

#[test]
fn test_string_join() {
    // RowBox[{"a", "<>", "b"}]
    let expr = row_box(vec![s("a"), s("<>"), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::StringJoin);
}

#[test]
fn test_binary_divide() {
    // RowBox[{"a", "/", "b"}]
    let expr = row_box(vec![s("a"), s("/"), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Divide);
}

#[test]
fn test_binary_map() {
    // RowBox[{"f", "/@", "list"}]
    let expr = row_box(vec![s("f"), s("/@"), s("list")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Map);
}

#[test]
fn test_binary_apply() {
    // RowBox[{"f", "@@", "list"}]
    let expr = row_box(vec![s("f"), s("@@"), s("list")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Apply);
}

#[test]
fn test_empty_row_box() {
    // RowBox[{}] → empty
    let expr = row_box(vec![]);
    let cst = parse_box_expr(&expr);

    // Empty RowBox should produce a Box node
    match &cst {
        Cst::Box(BoxNode { kind: BoxKind::RowBox, children: NodeSeq(ch), .. }) => {
            assert!(ch.is_empty());
        },
        other => panic!("Expected empty BoxNode[RowBox], got: {:?}", other),
    }
}

#[test]
fn test_integer_classification() {
    let expr = row_box(vec![s("42")]);
    let cst = parse_box_expr(&expr);
    assert_eq!(token_kind(&cst), Some(TK::Integer));
    assert_eq!(token_input(&cst), Some("42"));
}

#[test]
fn test_real_classification() {
    let expr = row_box(vec![s("3.14")]);
    let cst = parse_box_expr(&expr);
    assert_eq!(token_kind(&cst), Some(TK::Real));
    assert_eq!(token_input(&cst), Some("3.14"));
}

#[test]
fn test_string_classification() {
    let expr = row_box(vec![s("\"hello\"")]);
    let cst = parse_box_expr(&expr);
    assert_eq!(token_kind(&cst), Some(TK::String));
}

#[test]
fn test_underscore_classification() {
    // Single underscore → Under token
    let expr = row_box(vec![s("_")]);
    let cst = parse_box_expr(&expr);
    assert_eq!(token_kind(&cst), Some(TK::Under));
}

#[test]
fn test_double_bracket_part() {
    // RowBox[{"a", "\u{301a}", "1", "\u{301b}"}] → CallNode with Part
    // Uses Unicode LeftDoubleBracket / RightDoubleBracket characters
    let expr = row_box(vec![s("a"), s("\u{301a}"), s("1"), s("\u{301b}")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Call(CallNode { body, .. }) => {
            match body {
                CallBody::Group(GroupNode(OperatorNode { op, .. })) => {
                    assert_eq!(
                        *op,
                        CallOperator::CodeParser_GroupDoubleBracket
                    );
                },
                _ => panic!("Expected GroupDoubleBracket body"),
            }
        },
        other => panic!("Expected CallNode for Part, got: {:?}", other),
    }
}

#[test]
fn test_full_pipeline() {
    // Test that RowBox parsing produces valid CST that can be aggregated.
    // Note: full aggregate → abstract pipeline requires proper BoxPosition
    // spanning support, so we test a simpler case here: just verify that
    // aggregate_cst doesn't panic on simple cases.
    let expr = row_box(vec![s("42")]);
    let cst = parse_box_expr(&expr);

    // Single element: should be a leaf token
    let agg = match aggregate_cst(cst) {
        Some(agg) => agg,
        None => panic!("aggregate_cst returned None for leaf"),
    };

    let ast = abstract_cst(agg, QuirkSettings::default());

    match &ast {
        Ast::Leaf { kind: TK::Integer, input, .. } => {
            assert_eq!(input.to_str(), "42");
        },
        other => panic!("Expected Ast::Leaf Integer, got: {:?}", other),
    }
}

#[test]
fn test_nested_function_call() {
    // RowBox[{"f", "[", "g", "[", "x", "]", "]"}]
    // This should parse as f[g, [, x, ]] which is actually tricky
    // In practice, nested calls use nested RowBox:
    // RowBox[{"f", "[", RowBox[{"g", "[", "x", "]"}], "]"}]
    let inner = row_box(vec![s("g"), s("["), s("x"), s("]")]);
    let expr = row_box(vec![s("f"), s("["), inner, s("]")]);
    let cst = parse_box_expr(&expr);

    match &cst {
        Cst::Call(CallNode { head, body }) => {
            match head {
                CallHead::Aggregate(h) => {
                    assert_eq!(token_kind(h), Some(TK::Symbol));
                },
                CallHead::Concrete(NodeSeq(elts)) => {
                    assert_eq!(elts.len(), 1);
                    assert_eq!(token_kind(&elts[0]), Some(TK::Symbol));
                },
            }
            // Body should contain a nested call
            match body {
                CallBody::Group(GroupNode(OperatorNode {
                    children: NodeSeq(ch),
                    ..
                })) => {
                    let sig_children: Vec<_> =
                        ch.iter().filter(|c| !is_trivia(c)).collect();
                    // Should contain: OpenSquare, CallNode(g[x]), CloseSquare
                    assert!(sig_children.len() >= 2);
                },
                _ => panic!("Expected Group body"),
            }
        },
        other => panic!("Expected CallNode, got: {:?}", other),
    }
}

#[test]
fn test_comparison_operators() {
    // RowBox[{"a", "==", "b", "==", "c"}] → InfixNode[InfixInequality, ...]
    let expr = row_box(vec![s("a"), s("=="), s("b"), s("=="), s("c")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::CodeParser_InfixInequality);
}

#[test]
fn test_logical_and() {
    let expr = row_box(vec![s("a"), s("&&"), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::And);
}

#[test]
fn test_logical_or() {
    let expr = row_box(vec![s("a"), s("||"), s("b")]);
    let cst = parse_box_expr(&expr);
    assert_infix_op(&cst, InfixOperator::Or);
}

#[test]
fn test_binary_condition() {
    // RowBox[{"x", "/;", "cond"}]
    let expr = row_box(vec![s("x"), s("/;"), s("cond")]);
    let cst = parse_box_expr(&expr);
    assert_binary_op(&cst, BinaryOperator::Condition);
}

#[test]
fn test_prefix_pre_increment() {
    // RowBox[{"++", "x"}]
    let expr = row_box(vec![s("++"), s("x")]);
    let cst = parse_box_expr(&expr);
    assert_prefix_op(&cst, PrefixOperator::PreIncrement);
}

#[test]
fn test_postfix_increment() {
    // RowBox[{"x", "++"}]
    let expr = row_box(vec![s("x"), s("++")]);
    let cst = parse_box_expr(&expr);
    assert_postfix_op(&cst, PostfixOperator::Increment);
}

//==========================================================
// Helpers
//==========================================================

fn token_kind(cst: &Cst<TokenString, Source>) -> Option<TK> {
    match cst {
        Cst::Token(tok) => Some(tok.tok),
        _ => None,
    }
}

fn token_input(cst: &Cst<TokenString, Source>) -> Option<&str> {
    match cst {
        Cst::Token(tok) => Some(tok.input.to_str()),
        _ => None,
    }
}

fn is_trivia(cst: &Cst<TokenString, Source>) -> bool {
    match cst {
        Cst::Token(tok) => tok.tok.is_trivia(),
        _ => false,
    }
}
