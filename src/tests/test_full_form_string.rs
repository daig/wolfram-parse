use crate::{ast::ToFullFormError, ParseOptions};

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
