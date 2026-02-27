//! RowBox parsing: convert box expressions (from .nb files) into CST nodes.
//!
//! This module interprets `RowBox[{"2", "+", "x"}]` and similar box structures
//! into concrete syntax tree nodes that can be fed through `aggregate_cst()` →
//! `abstract_cst()`.
//!
//! Based on the WolframResearch/codeparser reference implementation in
//! `CodeParser/Kernel/Boxes.wl`.

use wolfram_expr::{Expr, ExprKind, Symbol};

use crate::{
    cst::{
        BinaryNode, BoxKind, BoxNode, CallBody, CallHead, CallNode, CodeNode,
        Cst, GroupMissingCloserNode, GroupMissingOpenerNode, GroupNode,
        InfixNode, OperatorNode, PostfixNode, PrefixNode,
    },
    parse::operators::{
        BinaryOperator, CallOperator, GroupOperator, InfixOperator,
        PostfixOperator, PrefixOperator,
    },
    source::{BoxPosition, Source},
    tokenize::{Token, TokenKind as TK, TokenString},
    NodeSeq,
};

/// Result of parsing a RowBox: either a single node, or children to be spliced
/// into the parent.
enum ParsedBox {
    /// A single CST node (the common case).
    Node(Cst<TokenString, Source>),
    /// Children that should be spliced into the parent's child list.
    /// This happens when a RowBox contains a single non-trivia element.
    ToBeSpliced(Vec<Cst<TokenString, Source>>),
}

//==========================================================
// Public API
//==========================================================

/// Parse a box expression (e.g., from a .nb notebook cell) into a CST node.
///
/// Input: an `Expr` like `RowBox[{"2", "+", "x"}]`
/// Output: a `Cst<TokenString, Source>` with `Source::Box(BoxPosition)` positions
///
/// The result can be fed into `aggregate_cst()` → `abstract_cst()`.
pub fn parse_box_expr(expr: &Expr) -> Cst<TokenString, Source> {
    let pos = Source::Box(BoxPosition::At(vec![]));
    expr_to_cst(expr, &[], pos)
}

//==========================================================
// Step 1: String-to-Token Classification
//==========================================================

/// Classify a string from a RowBox element into a CST Token node.
///
/// Maps operator strings like "+", "-", "==", etc. to their TokenKind,
/// and classifies identifiers, integers, strings, and whitespace.
fn classify_string(
    s: &str,
    pos: Source,
) -> Cst<TokenString, Source> {
    let kind = match s {
        // Arithmetic
        "+" => TK::Plus,
        "-" => TK::Minus,
        "*" => TK::Star,
        "/" => TK::Slash,
        "^" => TK::Caret,
        "." => TK::Dot,

        // Comparison
        "==" => TK::EqualEqual,
        "!=" => TK::BangEqual,
        "<" => TK::Less,
        ">" => TK::Greater,
        "<=" => TK::LessEqual,
        ">=" => TK::GreaterEqual,
        "===" => TK::EqualEqualEqual,
        "=!=" => TK::EqualBangEqual,

        // Assignment
        "=" => TK::Equal,
        ":=" => TK::ColonEqual,
        "+=" => TK::PlusEqual,
        "-=" => TK::MinusEqual,
        "*=" => TK::StarEqual,
        "/=" => TK::SlashEqual,
        "^=" => TK::CaretEqual,
        "^:=" => TK::CaretColonEqual,

        // Logical
        "&&" => TK::AmpAmp,
        "||" => TK::BarBar,
        "!" => TK::Bang,
        "!!" => TK::BangBang,

        // Patterns / Rules
        "->" => TK::MinusGreater,
        ":>" => TK::ColonGreater,
        "/." => TK::SlashDot,
        "//." => TK::SlashSlashDot,
        "/;" => TK::SlashSemi,
        "?" => TK::Question,
        "??" => TK::QuestionQuestion,
        "|" => TK::Bar,
        ".." => TK::DotDot,
        "..." => TK::DotDotDot,

        // Application / Mapping
        "@" => TK::At,
        "@@" => TK::AtAt,
        "@@@" => TK::AtAtAt,
        "/@" => TK::SlashAt,
        "//@" => TK::SlashSlashAt,
        "//" => TK::SlashSlash,
        "~" => TK::Tilde,
        "&" => TK::Amp,
        "<<" => TK::LessLess,
        ">>" => TK::GreaterGreater,
        ">>>" => TK::GreaterGreaterGreater,

        // String
        "<>" => TK::LessGreater,
        "~~" => TK::TildeTilde,

        // Span / Increment
        ";;" => TK::SemiSemi,
        "++" => TK::PlusPlus,
        "--" => TK::MinusMinus,

        // Other operators
        "/:" => TK::SlashColon,
        "::" => TK::ColonColon,
        "**" => TK::StarStar,
        "@*" => TK::AtStar,
        "/*" => TK::SlashStar,

        // Delimiters
        "[" => TK::OpenSquare,
        "]" => TK::CloseSquare,
        "{" => TK::OpenCurly,
        "}" => TK::CloseCurly,
        "(" => TK::OpenParen,
        ")" => TK::CloseParen,
        "\u{301a}" => TK::LongName_LeftDoubleBracket,
        "\u{301b}" => TK::LongName_RightDoubleBracket,
        "<|" => TK::LessBar,
        "|>" => TK::BarGreater,

        // Separators
        "," => TK::Comma,
        ";" => TK::Semi,

        // Underscore patterns
        "_" => TK::Under,
        "__" => TK::UnderUnder,
        "___" => TK::UnderUnderUnder,
        "_." => TK::UnderDot,

        // Hash/Slot
        "#" => TK::Hash,
        "##" => TK::HashHash,

        // Percent/Out
        "%" => TK::Percent,
        "%%" => TK::PercentPercent,

        // Newlines
        "\n" => TK::ToplevelNewline,

        // Other specific tokens
        "\u{f3c7}" => TK::LongName_LeftDoubleBracket,
        "\u{f3c8}" => TK::LongName_RightDoubleBracket,
        "\u{f4a1}" => TK::LongName_InvisibleApplication,

        _ => {
            return classify_string_fallback(s, pos);
        },
    };

    make_token(kind, s, pos)
}

/// Fallback classification for strings that don't match exact operators.
fn classify_string_fallback(
    s: &str,
    pos: Source,
) -> Cst<TokenString, Source> {
    // Check for whitespace (spaces/tabs)
    if !s.is_empty() && s.chars().all(|c| c == ' ' || c == '\t') {
        return make_token(TK::Boxes_MultiWhitespace, s, pos);
    }

    // Check for newlines
    if s == "\n" || s == "\r\n" || s == "\r" {
        return make_token(TK::ToplevelNewline, s, pos);
    }

    // Multi single-quote (for Derivative)
    if !s.is_empty() && s.chars().all(|c| c == '\'') {
        return make_token(TK::Boxes_MultiSingleQuote, s, pos);
    }

    // Simple identifier: [a-zA-Z$][a-zA-Z$0-9]*
    if is_simple_symbol(s) {
        return make_token(TK::Symbol, s, pos);
    }

    // Simple integer: [0-9]+
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        return make_token(TK::Integer, s, pos);
    }

    // Simple real number: digits with at most one dot
    if is_simple_real(s) {
        return make_token(TK::Real, s, pos);
    }

    // Quoted string: starts and ends with "
    if s.starts_with('"') {
        return make_token(TK::String, s, pos);
    }

    // Comment
    if s.starts_with("(*") {
        return make_token(TK::Comment, s, pos);
    }

    // Fallthrough: tokenize with text parser, take first token
    tokenize_string_fallback(s, pos)
}

/// Check if a string is a simple symbol identifier.
fn is_simple_symbol(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '$' || first == '\u{f39b}') {
        // \[FormalA]-style characters start at \uf800 range
        if !(first >= '\u{f800}' && first <= '\u{f8ff}') {
            return false;
        }
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '$' || c == '`')
}

/// Check if a string is a simple real number like "3.14".
fn is_simple_real(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut dot_count = 0;
    let mut digit_count = 0;
    for c in s.chars() {
        if c == '.' {
            dot_count += 1;
            if dot_count > 1 {
                return false;
            }
        } else if c.is_ascii_digit() {
            digit_count += 1;
        } else {
            return false;
        }
    }
    dot_count == 1 && digit_count > 0
}

/// Last-resort: use the text tokenizer to classify an unknown string.
fn tokenize_string_fallback(
    s: &str,
    pos: Source,
) -> Cst<TokenString, Source> {
    use crate::{tokenize, ParseOptions};

    let opts = ParseOptions::default();
    let NodeSeq(tokens) = tokenize(s, &opts);

    if let Some(first) = tokens.into_iter().next() {
        make_token(first.tok, s, pos)
    } else {
        // If tokenization produces nothing, treat as a symbol
        make_token(TK::Symbol, s, pos)
    }
}

/// Helper: create a `Cst::Token` with `TokenString` input and `Source` position.
fn make_token(
    kind: TK,
    input: &str,
    src: Source,
) -> Cst<TokenString, Source> {
    Cst::Token(Token {
        tok: kind,
        input: TokenString::from_string(input.to_owned()),
        src,
    })
}

//==========================================================
// Step 2: Expr-to-CST Conversion
//==========================================================

/// Convert a box expression element into a CST node.
///
/// Dispatches on the Expr variant: String → classify_string,
/// Normal with RowBox head → recurse, other Normal → BoxNode, etc.
fn expr_to_cst(
    expr: &Expr,
    pos_prefix: &[usize],
    pos: Source,
) -> Cst<TokenString, Source> {
    match expr.kind() {
        ExprKind::String(s) => classify_string(s, pos),

        ExprKind::Normal(normal) => {
            // Check if head is a box symbol
            if let ExprKind::Symbol(head_sym) = normal.head().kind() {
                let name = head_sym.as_str();

                if name == "System`RowBox" {
                    // RowBox[{children...}]
                    if let Some(list_expr) = normal.elements().first() {
                        if let ExprKind::Normal(list_normal) = list_expr.kind() {
                            if list_normal.has_head(&Symbol::new("System`List")) {
                                match parse_row_box(list_normal.elements(), pos_prefix) {
                                    ParsedBox::Node(cst) => return cst,
                                    ParsedBox::ToBeSpliced(mut nodes) => {
                                        if nodes.len() == 1 {
                                            return nodes.pop().unwrap();
                                        }
                                        // Multiple nodes: wrap in a RowBox BoxNode
                                        return Cst::Box(BoxNode {
                                            kind: BoxKind::RowBox,
                                            children: NodeSeq(nodes),
                                            src: pos,
                                        });
                                    },
                                }
                            }
                        }
                        // RowBox with a List arg that's just a raw list of exprs
                        if let ExprKind::Normal(_) = list_expr.kind() {
                            // Not a List - try treating elements directly
                        } else {
                            // Single non-list element
                            return expr_to_cst(list_expr, pos_prefix, pos);
                        }
                    }
                    // Fallback: empty RowBox or unexpected structure
                    return Cst::Box(BoxNode {
                        kind: BoxKind::RowBox,
                        children: NodeSeq(vec![]),
                        src: pos,
                    });
                }

                // Other box types: wrap as BoxNode pass-through
                if let Some(box_kind) = box_kind_from_symbol_name(name) {
                    let children: Vec<Cst<TokenString, Source>> = normal
                        .elements()
                        .iter()
                        .enumerate()
                        .map(|(i, child_expr)| {
                            let mut child_prefix = pos_prefix.to_vec();
                            child_prefix.push(i + 1);
                            let child_pos =
                                Source::Box(BoxPosition::At(child_prefix.clone()));
                            expr_to_cst(child_expr, &child_prefix, child_pos)
                        })
                        .collect();

                    return Cst::Box(BoxNode {
                        kind: box_kind,
                        children: NodeSeq(children),
                        src: pos,
                    });
                }
            }

            // Not a recognized box: wrap as CodeNode
            Cst::Code(CodeNode {
                first: expr.clone(),
                second: Expr::string("Null"),
                src: pos,
            })
        },

        ExprKind::Integer(n) => {
            make_token(TK::Integer, &n.to_string(), pos)
        },

        ExprKind::Real(r) => {
            make_token(TK::Real, &r.to_string(), pos)
        },

        ExprKind::Symbol(sym) => {
            let name = sym.symbol_name().as_str();
            make_token(TK::Symbol, name, pos)
        },
    }
}

/// Map a full symbol name like "System`RowBox" to a BoxKind variant.
fn box_kind_from_symbol_name(name: &str) -> Option<BoxKind> {
    match name {
        "System`RowBox" => Some(BoxKind::RowBox),
        "System`GridBox" => Some(BoxKind::GridBox),
        "System`TagBox" => Some(BoxKind::TagBox),
        "System`TemplateBox" => Some(BoxKind::TemplateBox),
        "System`InterpretationBox" => Some(BoxKind::InterpretationBox),
        "System`SubscriptBox" => Some(BoxKind::SubscriptBox),
        "System`SuperscriptBox" => Some(BoxKind::SuperscriptBox),
        "System`StyleBox" => Some(BoxKind::StyleBox),
        "System`NamespaceBox" => Some(BoxKind::NamespaceBox),
        "System`OverscriptBox" => Some(BoxKind::OverscriptBox),
        "System`SubsuperscriptBox" => Some(BoxKind::SubsuperscriptBox),
        "System`GraphicsBox" => Some(BoxKind::GraphicsBox),
        _ => {
            if name.starts_with("System`") && name.ends_with("Box") {
                Some(BoxKind::Other(Symbol::new(name)))
            } else {
                None
            }
        },
    }
}

//==========================================================
// Step 3: parse_row_box — Core RowBox Parsing
//==========================================================

/// Parse the children of a RowBox into a CST node.
///
/// Takes the elements of `RowBox[{children...}]` and produces structured CST.
fn parse_row_box(
    children: &[Expr],
    pos_prefix: &[usize],
) -> ParsedBox {
    if children.is_empty() {
        return ParsedBox::ToBeSpliced(vec![]);
    }

    // Convert each child Expr to a CST node
    let handled_children: Vec<Cst<TokenString, Source>> = children
        .iter()
        .enumerate()
        .map(|(i, child_expr)| {
            let mut child_pos_prefix = pos_prefix.to_vec();
            child_pos_prefix.push(i + 1);
            let child_pos =
                Source::Box(BoxPosition::At(child_pos_prefix.clone()));
            expr_to_cst(child_expr, &child_pos_prefix, child_pos)
        })
        .collect();

    // Filter to non-trivia children for structural analysis
    let agg_indices: Vec<usize> = handled_children
        .iter()
        .enumerate()
        .filter(|(_, c)| !is_trivia_cst(c))
        .map(|(i, _)| i)
        .collect();

    // If single non-trivia child, splice it (unwrap the RowBox)
    if agg_indices.len() == 1 {
        return ParsedBox::ToBeSpliced(handled_children);
    }

    // If no non-trivia children, just return as-is
    if agg_indices.is_empty() {
        return ParsedBox::ToBeSpliced(handled_children);
    }

    let pos = Source::Box(BoxPosition::At(pos_prefix.to_vec()));
    prb_dispatch(handled_children, &agg_indices, pos)
}

//==========================================================
// Step 4: prb_dispatch — Pattern Matching
//==========================================================

/// Dispatch on the structure of non-trivia children to determine
/// the CST node type.
fn prb_dispatch(
    all: Vec<Cst<TokenString, Source>>,
    agg_indices: &[usize],
    pos: Source,
) -> ParsedBox {
    let agg_count = agg_indices.len();

    // Collect references to the "significant" (non-trivia) children
    let agg: Vec<&Cst<TokenString, Source>> =
        agg_indices.iter().map(|&i| &all[i]).collect();

    // 1. Calls: {head, OpenSquare, ..., CloseSquare}
    if agg_count >= 2 {
        if let Some(result) = try_call(&all, &agg, agg_indices, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 2. Groups: matched brackets
    if agg_count >= 2 {
        if let Some(result) = try_group(&all, &agg, agg_indices, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 3. Infix: odd-indexed agg children are the same operator
    if agg_count >= 3 && agg_count % 2 == 1 {
        if let Some(result) = try_infix(&all, &agg, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 4. Binary: exactly 3 agg children, middle is binary op
    if agg_count == 3 {
        if let Some(result) = try_binary(&all, &agg, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 5. Prefix: exactly 2 agg children, first is prefix op
    if agg_count == 2 {
        if let Some(result) = try_prefix(&all, &agg, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 6. Postfix: exactly 2 agg children, second is postfix op
    if agg_count == 2 {
        if let Some(result) = try_postfix(&all, &agg, &pos) {
            return ParsedBox::Node(result);
        }
    }

    // 7. Fallthrough: implicit times
    ParsedBox::Node(make_implicit_times(all, pos))
}

//==========================================================
// Pattern Matchers
//==========================================================

/// Try to match a call pattern: `{head, OpenSquare, ..., CloseSquare}`
fn try_call(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    agg_indices: &[usize],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    let agg_count = agg.len();
    if agg_count < 3 {
        return None;
    }

    // Check if second agg child is OpenSquare and last is CloseSquare
    let second_tok = token_kind_of(agg[1])?;
    let last_tok = token_kind_of(agg[agg_count - 1])?;

    let call_op = match (second_tok, last_tok) {
        (TK::OpenSquare, TK::CloseSquare) => {
            CallOperator::CodeParser_GroupSquare
        },
        (TK::LongName_LeftDoubleBracket, TK::LongName_RightDoubleBracket) => {
            CallOperator::CodeParser_GroupDoubleBracket
        },
        _ => return None,
    };

    // Split: head is everything up to the bracket opener (in original indices),
    // body is the bracket group
    let opener_idx = agg_indices[1]; // index into `all`

    let head_children: Vec<Cst<TokenString, Source>> =
        all[..opener_idx].to_vec();
    let body_children: Vec<Cst<TokenString, Source>> =
        all[opener_idx..].to_vec();

    let head = if head_children.len() == 1 {
        CallHead::Aggregate(Box::new(head_children.into_iter().next().unwrap()))
    } else {
        CallHead::Concrete(NodeSeq(head_children))
    };

    let body = CallBody::Group(GroupNode(OperatorNode {
        op: call_op,
        children: NodeSeq(body_children),
    }));

    Some(Cst::Call(CallNode { head, body }))
}

/// Try to match a group pattern: `{OpenCurly, ..., CloseCurly}` etc.
fn try_group(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    _agg_indices: &[usize],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    let first_tok = token_kind_of(agg[0])?;
    let last_tok = token_kind_of(agg[agg.len() - 1])?;

    let group_op = match (first_tok, last_tok) {
        (TK::OpenCurly, TK::CloseCurly) => GroupOperator::List,
        (TK::OpenParen, TK::CloseParen) => GroupOperator::CodeParser_GroupParen,
        (TK::LessBar, TK::BarGreater) => GroupOperator::Association,
        (TK::OpenSquare, TK::CloseSquare) => {
            GroupOperator::CodeParser_GroupSquare
        },
        // Missing closer
        (TK::OpenCurly, _) => {
            return Some(Cst::GroupMissingCloser(GroupMissingCloserNode(
                OperatorNode {
                    op: GroupOperator::List,
                    children: NodeSeq(all.to_vec()),
                },
            )));
        },
        (TK::OpenParen, _) => {
            return Some(Cst::GroupMissingCloser(GroupMissingCloserNode(
                OperatorNode {
                    op: GroupOperator::CodeParser_GroupParen,
                    children: NodeSeq(all.to_vec()),
                },
            )));
        },
        // Missing opener
        (_, TK::CloseCurly) => {
            return Some(Cst::GroupMissingOpener(GroupMissingOpenerNode(
                OperatorNode {
                    op: GroupOperator::List,
                    children: NodeSeq(all.to_vec()),
                },
            )));
        },
        (_, TK::CloseParen) => {
            return Some(Cst::GroupMissingOpener(GroupMissingOpenerNode(
                OperatorNode {
                    op: GroupOperator::CodeParser_GroupParen,
                    children: NodeSeq(all.to_vec()),
                },
            )));
        },
        _ => return None,
    };

    Some(Cst::Group(GroupNode(OperatorNode {
        op: group_op,
        children: NodeSeq(all.to_vec()),
    })))
}

/// Try to match an infix pattern: odd-indexed agg children are the same
/// operator token.
fn try_infix(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    // Check that all odd-indexed agg children are the same operator token
    let first_op_tok = token_kind_of(agg[1])?;
    let infix_op = infix_op_for(first_op_tok)?;

    for i in (1..agg.len()).step_by(2) {
        let tok = token_kind_of(agg[i])?;
        if infix_op_for(tok)? != infix_op {
            return None;
        }
    }

    // Check that even-indexed children are NOT operators
    // (they should be operands)
    for i in (0..agg.len()).step_by(2) {
        if let Some(tok) = token_kind_of(agg[i]) {
            if is_pure_infix_operator(tok) {
                return None;
            }
        }
    }

    Some(Cst::Infix(InfixNode(OperatorNode {
        op: infix_op,
        children: NodeSeq(all.to_vec()),
    })))
}

/// Try to match a binary pattern: exactly 3 agg children, middle is binary op.
fn try_binary(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    let mid_tok = token_kind_of(agg[1])?;
    let binary_op = binary_op_for(mid_tok)?;

    Some(Cst::Binary(BinaryNode(OperatorNode {
        op: binary_op,
        children: NodeSeq(all.to_vec()),
    })))
}

/// Try to match a prefix pattern: exactly 2 agg children, first is prefix op.
fn try_prefix(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    let first_tok = token_kind_of(agg[0])?;
    let prefix_op = prefix_op_for(first_tok)?;

    Some(Cst::Prefix(PrefixNode(OperatorNode {
        op: prefix_op,
        children: NodeSeq(all.to_vec()),
    })))
}

/// Try to match a postfix pattern: exactly 2 agg children, second is postfix op.
fn try_postfix(
    all: &[Cst<TokenString, Source>],
    agg: &[&Cst<TokenString, Source>],
    _pos: &Source,
) -> Option<Cst<TokenString, Source>> {
    let second_tok = token_kind_of(agg[1])?;
    let postfix_op = postfix_op_for(second_tok)?;

    Some(Cst::Postfix(PostfixNode(OperatorNode {
        op: postfix_op,
        children: NodeSeq(all.to_vec()),
    })))
}

//==========================================================
// Step 5: Operator Classification Helpers
//==========================================================

/// Map a TokenKind to an InfixOperator (if it can act as infix).
fn infix_op_for(tok: TK) -> Option<InfixOperator> {
    let op = match tok {
        TK::Plus | TK::Minus => InfixOperator::Plus,
        TK::Star => InfixOperator::Times,
        TK::Dot => InfixOperator::Dot,
        TK::StarStar => InfixOperator::NonCommutativeMultiply,
        TK::AmpAmp => InfixOperator::And,
        TK::BarBar => InfixOperator::Or,
        TK::Bar => InfixOperator::Alternatives,
        TK::LessGreater => InfixOperator::StringJoin,
        TK::TildeTilde => InfixOperator::StringExpression,
        TK::Comma => InfixOperator::CodeParser_Comma,
        TK::Semi => InfixOperator::CompoundExpression,
        TK::Fake_ImplicitTimes => InfixOperator::Times,

        // Comparison operators form InfixInequality chains
        TK::EqualEqual | TK::BangEqual | TK::Less | TK::Greater
        | TK::LessEqual | TK::GreaterEqual | TK::EqualEqualEqual
        | TK::EqualBangEqual => InfixOperator::CodeParser_InfixInequality,

        // Long-name operators
        TK::LongName_And => InfixOperator::And,
        TK::LongName_Or => InfixOperator::Or,
        TK::LongName_Xor => InfixOperator::Xor,
        TK::LongName_Nand => InfixOperator::Nand,
        TK::LongName_Nor => InfixOperator::Nor,
        TK::LongName_Xnor => InfixOperator::Xnor,
        TK::LongName_Element => InfixOperator::Element,
        TK::LongName_Subset => InfixOperator::Subset,
        TK::LongName_Superset => InfixOperator::Superset,
        TK::LongName_Union => InfixOperator::Union,
        TK::LongName_Intersection => InfixOperator::Intersection,
        TK::LongName_LeftArrow => InfixOperator::LeftArrow,
        TK::LongName_RightArrow => InfixOperator::RightArrow,
        TK::LongName_LeftRightArrow => InfixOperator::LeftRightArrow,
        TK::LongName_Times | TK::LongName_InvisibleTimes => {
            InfixOperator::Times
        },
        TK::LongName_CenterDot => InfixOperator::CenterDot,
        TK::LongName_CirclePlus => InfixOperator::CirclePlus,
        TK::LongName_CircleTimes => InfixOperator::CircleTimes,
        TK::LongName_Equivalent => InfixOperator::Equivalent,

        _ => return None,
    };
    Some(op)
}

/// Map a TokenKind to a BinaryOperator.
fn binary_op_for(tok: TK) -> Option<BinaryOperator> {
    let op = match tok {
        TK::Slash => BinaryOperator::Divide,
        TK::Caret => BinaryOperator::Power,
        TK::SemiSemi => BinaryOperator::Span,
        TK::Equal => BinaryOperator::Set,
        TK::ColonEqual => BinaryOperator::SetDelayed,
        TK::MinusGreater => BinaryOperator::Rule,
        TK::ColonGreater => BinaryOperator::RuleDelayed,
        TK::SlashDot => BinaryOperator::ReplaceAll,
        TK::SlashSlashDot => BinaryOperator::ReplaceRepeated,
        TK::SlashColon => BinaryOperator::Pattern,
        TK::SlashSemi => BinaryOperator::Condition,
        TK::At => BinaryOperator::CodeParser_BinaryAt,
        TK::SlashSlash => BinaryOperator::CodeParser_BinarySlashSlash,
        TK::SlashAt => BinaryOperator::Map,
        TK::SlashSlashAt => BinaryOperator::MapAll,
        TK::AtAt => BinaryOperator::Apply,
        TK::AtAtAt => BinaryOperator::MapApply,
        TK::PlusEqual => BinaryOperator::AddTo,
        TK::MinusEqual => BinaryOperator::SubtractFrom,
        TK::StarEqual => BinaryOperator::TimesBy,
        TK::SlashEqual => BinaryOperator::DivideBy,
        TK::CaretEqual => BinaryOperator::UpSet,
        TK::CaretColonEqual => BinaryOperator::UpSetDelayed,
        TK::Question => BinaryOperator::PatternTest,
        TK::LessLess => BinaryOperator::Put,
        TK::GreaterGreater => BinaryOperator::Put,
        TK::GreaterGreaterGreater => BinaryOperator::PutAppend,

        TK::LongName_Rule => BinaryOperator::Rule,
        TK::LongName_RuleDelayed => BinaryOperator::RuleDelayed,
        TK::LongName_DirectedEdge => BinaryOperator::DirectedEdge,
        TK::LongName_UndirectedEdge => BinaryOperator::UndirectedEdge,
        TK::LongName_TwoWayRule => BinaryOperator::TwoWayRule,
        TK::LongName_Implies => BinaryOperator::Implies,
        TK::LongName_PlusMinus => BinaryOperator::PlusMinus,
        TK::LongName_MinusPlus => BinaryOperator::MinusPlus,
        TK::LongName_CircleMinus => BinaryOperator::CircleMinus,
        TK::LongName_Function => BinaryOperator::Function,

        _ => return None,
    };
    Some(op)
}

/// Map a TokenKind to a PrefixOperator.
fn prefix_op_for(tok: TK) -> Option<PrefixOperator> {
    let op = match tok {
        TK::Minus => PrefixOperator::Minus,
        TK::Plus => PrefixOperator::Plus,
        TK::Bang => PrefixOperator::Not,
        TK::PlusPlus => PrefixOperator::PreIncrement,
        TK::MinusMinus => PrefixOperator::PreDecrement,
        TK::BangBang => PrefixOperator::CodeParser_PrefixNot2,
        TK::LongName_PlusMinus => PrefixOperator::PlusMinus,
        TK::LongName_MinusPlus => PrefixOperator::MinusPlus,
        TK::LongName_Sqrt => PrefixOperator::Sqrt,
        TK::LongName_Not => PrefixOperator::Not,
        TK::LongName_Del => PrefixOperator::Del,
        TK::LongName_Square => PrefixOperator::Square,
        TK::LongName_ForAll => PrefixOperator::ForAll,
        TK::LongName_Exists => PrefixOperator::Exists,
        TK::LongName_NotExists => PrefixOperator::NotExists,
        TK::LongName_CircleTimes => PrefixOperator::CircleTimes,
        _ => return None,
    };
    Some(op)
}

/// Map a TokenKind to a PostfixOperator.
fn postfix_op_for(tok: TK) -> Option<PostfixOperator> {
    let op = match tok {
        TK::Amp => PostfixOperator::Function,
        TK::Bang => PostfixOperator::Factorial,
        TK::BangBang => PostfixOperator::Factorial2,
        TK::PlusPlus => PostfixOperator::Increment,
        TK::MinusMinus => PostfixOperator::Decrement,
        TK::DotDot => PostfixOperator::Repeated,
        TK::DotDotDot => PostfixOperator::RepeatedNull,
        TK::Boxes_MultiSingleQuote => PostfixOperator::Derivative,
        TK::LongName_Transpose => PostfixOperator::Transpose,
        TK::LongName_Conjugate => PostfixOperator::Conjugate,
        TK::LongName_ConjugateTranspose => {
            PostfixOperator::ConjugateTranspose
        },
        TK::LongName_HermitianConjugate => {
            PostfixOperator::HermitianConjugate
        },
        _ => return None,
    };
    Some(op)
}

/// Check if a token kind is a "pure" infix operator (i.e., it can ONLY be infix,
/// not also prefix or postfix). Used to reject invalid infix patterns.
fn is_pure_infix_operator(tok: TK) -> bool {
    matches!(
        tok,
        TK::Star
            | TK::StarStar
            | TK::AmpAmp
            | TK::BarBar
            | TK::LessGreater
            | TK::TildeTilde
            | TK::Comma
    )
}

//==========================================================
// Step 6: Implicit Times Insertion
//==========================================================

/// Insert `Fake_ImplicitTimes` tokens between adjacent non-trivia nodes
/// and wrap in an InfixNode[Times, ...].
fn make_implicit_times(
    children: Vec<Cst<TokenString, Source>>,
    _pos: Source,
) -> Cst<TokenString, Source> {
    let mut result: Vec<Cst<TokenString, Source>> = Vec::new();
    let mut last_was_non_trivia = false;

    for child in children {
        let is_trivia = is_trivia_cst(&child);

        if !is_trivia && last_was_non_trivia {
            // Insert implicit times between adjacent non-trivia nodes
            let fake_pos = child.get_source();
            result.push(Cst::Token(Token {
                tok: TK::Fake_ImplicitTimes,
                input: TokenString::from_string(String::new()),
                src: fake_pos,
            }));
        }

        if !is_trivia {
            last_was_non_trivia = true;
        }

        result.push(child);
    }

    Cst::Infix(InfixNode(OperatorNode {
        op: InfixOperator::Times,
        children: NodeSeq(result),
    }))
}

//==========================================================
// Helpers
//==========================================================

/// Check if a CST node is trivia (whitespace, comment, newline).
///
/// Note: `TokenKind::is_trivia()` doesn't cover `Boxes_MultiWhitespace`
/// because it has a different bit pattern, so we check explicitly.
fn is_trivia_cst(cst: &Cst<TokenString, Source>) -> bool {
    match cst {
        Cst::Token(tok) => matches!(
            tok.tok,
            TK::Whitespace
                | TK::Boxes_MultiWhitespace
                | TK::Comment
                | TK::InternalNewline
                | TK::ToplevelNewline
        ),
        _ => false,
    }
}

/// Extract the TokenKind from a CST node if it's a Token.
fn token_kind_of(cst: &Cst<TokenString, Source>) -> Option<TK> {
    match cst {
        Cst::Token(tok) => Some(tok.tok),
        _ => None,
    }
}
