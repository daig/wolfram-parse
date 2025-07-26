//! Mapping [`TokenKind`] variants to parselet implementations

#![allow(non_upper_case_globals)]

use crate::{
    parse::{operators::{CompoundOperator, PrefixOperator, GroupOperator, PrefixBinaryOperator}, parselet::*},
    precedence::Precedence,
    tokenize::TokenKind,
};


pub(crate) const under1Parselet: UnderParselet = UnderParselet::new(
    CompoundOperator::Blank,
    CompoundOperator::CodeParser_PatternBlank,
);
pub(crate) const under2Parselet: UnderParselet = UnderParselet::new(
    CompoundOperator::BlankSequence,
    CompoundOperator::CodeParser_PatternBlankSequence,
);
pub(crate) const under3Parselet: UnderParselet = UnderParselet::new(
    CompoundOperator::BlankNullSequence,
    CompoundOperator::CodeParser_PatternBlankNullSequence,
);

// Prefix operator parselets
pub(crate) const PREFIX_MINUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_MINUS, PrefixOperator::Minus);
pub(crate) const PREFIX_PLUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_PLUS, PrefixOperator::Plus);
pub(crate) const PREFIX_BANG: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_BANG, PrefixOperator::Not);
pub(crate) const PREFIX_PLUSPLUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_PLUSPLUS, PrefixOperator::PreIncrement);
pub(crate) const PREFIX_MINUSMINUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_MINUSMINUS, PrefixOperator::PreDecrement);
pub(crate) const PREFIX_BANGBANG: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::FAKE_PREFIX_BANGBANG, PrefixOperator::CodeParser_PrefixNot2);

pub(crate) const PREFIX_LONGNAME_PLUSMINUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_LONGNAME_PLUSMINUS, PrefixOperator::PlusMinus);
pub(crate) const PREFIX_LONGNAME_SUM: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_SUM, PrefixOperator::Sum);
pub(crate) const PREFIX_LONGNAME_NOT: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_NOT, PrefixOperator::Not);
pub(crate) const PREFIX_LONGNAME_SQRT: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_SQRT, PrefixOperator::Sqrt);
pub(crate) const PREFIX_LONGNAME_MINUSPLUS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_LONGNAME_MINUSPLUS, PrefixOperator::MinusPlus);
pub(crate) const PREFIX_LONGNAME_DIFFERENTIALD: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_DIFFERENTIALD, PrefixOperator::DifferentialD);
pub(crate) const PREFIX_LONGNAME_CAPITALDIFFERENTIALD: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_CAPITALDIFFERENTIALD, PrefixOperator::CapitalDifferentialD);
pub(crate) const PREFIX_LONGNAME_MINUS_OP: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_LONGNAME_MINUS, PrefixOperator::Minus);
pub(crate) const PREFIX_LONGNAME_DEL: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_DEL, PrefixOperator::Del);
pub(crate) const PREFIX_LONGNAME_SQUARE: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_SQUARE, PrefixOperator::Square);

pub(crate) const PREFIX_LONGNAME_PRODUCT: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_PRODUCT, PrefixOperator::Product);
pub(crate) const PREFIX_LONGNAME_CONTINUEDFRACTIONK: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_CONTINUEDFRACTIONK, PrefixOperator::ContinuedFractionK);
pub(crate) const PREFIX_LONGNAME_CIRCLETIMES: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_LONGNAME_CIRCLETIMES, PrefixOperator::CircleTimes);
pub(crate) const PREFIX_LONGNAME_FORALL: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_FORALL, PrefixOperator::ForAll);
pub(crate) const PREFIX_LONGNAME_EXISTS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_EXISTS, PrefixOperator::Exists);
pub(crate) const PREFIX_LONGNAME_NOTEXISTS: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_NOTEXISTS, PrefixOperator::NotExists);
pub(crate) const PREFIX_LONGNAME_COPRODUCT: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::PREFIX_LONGNAME_COPRODUCT, PrefixOperator::Coproduct);
pub(crate) const PREFIX_LONGNAME_PIECEWISE: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_PIECEWISE, PrefixOperator::Piecewise);
pub(crate) const PREFIX_LONGNAME_INVISIBLEPREFIXSCRIPTBASE: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_INVISIBLEPREFIXSCRIPTBASE, PrefixOperator::InvisiblePrefixScriptBase);
pub(crate) const PREFIX_LONGNAME_EXPECTATIONE: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_EXPECTATIONE, PrefixOperator::ExpectationE);
pub(crate) const PREFIX_LONGNAME_CUBEROOT: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_CUBEROOT, PrefixOperator::CubeRoot);
pub(crate) const PREFIX_LONGNAME_PROBABILITYPR: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LONGNAME_PROBABILITYPR, PrefixOperator::ProbabilityPr);

pub(crate) const PREFIX_LINEARSYNTAX_BANG: PrefixOperatorParselet = PrefixOperatorParselet::new(Precedence::LINEARSYNTAX_BANG, PrefixOperator::CodeParser_PrefixLinearSyntaxBang);

// Group parselets
pub(crate) const GROUP_PAREN: GroupParselet = GroupParselet::new(TokenKind::OpenParen, GroupOperator::CodeParser_GroupParen);
pub(crate) const GROUP_SQUARE: GroupParselet = GroupParselet::new(TokenKind::OpenSquare, GroupOperator::CodeParser_GroupSquare);
pub(crate) const GROUP_CURLY: GroupParselet = GroupParselet::new(TokenKind::OpenCurly, GroupOperator::List);
pub(crate) const GROUP_LESSBAR: GroupParselet = GroupParselet::new(TokenKind::LessBar, GroupOperator::Association);
pub(crate) const GROUP_COLONCOLONOPENSQUARE: GroupParselet = GroupParselet::new(TokenKind::ColonColonOpenSquare, GroupOperator::CodeParser_GroupTypeSpecifier);
pub(crate) const GROUP_LEFTANGLEBRACKET: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftAngleBracket, GroupOperator::AngleBracket);
pub(crate) const GROUP_LEFTCEILING: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftCeiling, GroupOperator::Ceiling);
pub(crate) const GROUP_LEFTFLOOR: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftFloor, GroupOperator::Floor);
pub(crate) const GROUP_LEFTDOUBLEBRACKET: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftDoubleBracket, GroupOperator::CodeParser_GroupDoubleBracket);
pub(crate) const GROUP_LEFTBRACKETINGBAR: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftBracketingBar, GroupOperator::BracketingBar);
pub(crate) const GROUP_LEFTDOUBLEBRACKETINGBAR: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftDoubleBracketingBar, GroupOperator::DoubleBracketingBar);
pub(crate) const GROUP_LEFTASSOCIATION: GroupParselet = GroupParselet::new(TokenKind::LongName_LeftAssociation, GroupOperator::Association);
pub(crate) const GROUP_OPENCURLYQUOTE: GroupParselet = GroupParselet::new(TokenKind::LongName_OpenCurlyQuote, GroupOperator::CurlyQuote);
pub(crate) const GROUP_OPENCURLYDOUBLEQUOTE: GroupParselet = GroupParselet::new(TokenKind::LongName_OpenCurlyDoubleQuote, GroupOperator::CurlyDoubleQuote);

// Integral parselets
pub(crate) const INTEGRAL_INTEGRATE: IntegralParselet = IntegralParselet::new(PrefixBinaryOperator::Integrate, PrefixOperator::Integral);
pub(crate) const INTEGRAL_CONTOUR: IntegralParselet = IntegralParselet::new(PrefixBinaryOperator::ContourIntegral, PrefixOperator::ContourIntegral);
pub(crate) const INTEGRAL_DOUBLECONTOUR: IntegralParselet = IntegralParselet::new(PrefixBinaryOperator::DoubleContourIntegral, PrefixOperator::DoubleContourIntegral);
pub(crate) const INTEGRAL_CLOCKWISECONTOUR: IntegralParselet = IntegralParselet::new(PrefixBinaryOperator::ClockwiseContourIntegral, PrefixOperator::ClockwiseContourIntegral);
pub(crate) const INTEGRAL_COUNTERCLOCKWISECONTOUR: IntegralParselet = IntegralParselet::new(PrefixBinaryOperator::CounterClockwiseContourIntegral, PrefixOperator::CounterClockwiseContourIntegral);

// Call parselets  
pub(crate) const CALL_SQUARE: CallParselet = CallParselet::new(GROUP_SQUARE);
pub(crate) const CALL_LEFTDOUBLEBRACKET: CallParselet = CallParselet::new(GROUP_LEFTDOUBLEBRACKET);
pub(crate) const CALL_COLONCOLONOPENSQUARE: CallParselet = CallParselet::new(GROUP_COLONCOLONOPENSQUARE);

// Special parselets
pub(crate) const EQUAL_PARSELET: EqualParselet = EqualParselet::new();
pub(crate) const COLONEQUAL_PARSELET: ColonEqualParselet = ColonEqualParselet::new();

macro_rules! token_kind_to_prefix_parselet {
    ($ty:ty; $kind:ident) => {{
        use crate::{
            tokenize::TokenKind as TK,
            parse::parselet::*,
        };

        // Helper macro for creating static parselets in match arms
        macro_rules! static_parselet {
            ($parselet_type:ty) => {{
                static PARSELET: $parselet_type = <$parselet_type>::new();
                &PARSELET
            }};
            ($parselet_type:ty, $arg1:expr) => {{
                static PARSELET: $parselet_type = <$parselet_type>::new($arg1);
                &PARSELET
            }};
            ($parselet_type:ty, $arg1:expr, $arg2:expr) => {{
                static PARSELET: $parselet_type = <$parselet_type>::new($arg1, $arg2);
                &PARSELET
            }};
            ($parselet_type:ty, $arg1:expr, $arg2:expr, $arg3:expr) => {{
                static PARSELET: $parselet_type = <$parselet_type>::new($arg1, $arg2, $arg3);
                &PARSELET
            }}
        }

    match $kind {
        TK::EndOfFile => &PrefixEndOfFileParselet {},

        TK::String
        | TK::Integer
        | TK::Real
        | TK::Rational
        | TK::LinearSyntaxBlob => &LeafParselet {},

        TK::Unknown
        | TK::Whitespace
        | TK::InternalNewline
        | TK::Comment => &PrefixErrorParselet {},


        TK::Error_ExpectedEqual
        | TK::Error_Number
        | TK::Error_UnhandledCharacter
        | TK::Error_ExpectedLetterlike
        | TK::Error_Aborted
        | TK::Error_ExpectedOperand
        | TK::Error_ExpectedTag
        | TK::Error_ExpectedFile
        | TK::Error_UnterminatedComment
        | TK::Error_UnterminatedString
        | TK::Error_UnterminatedFileString
        | TK::Error_UnterminatedLinearSyntaxBlob
        | TK::Error_UnsupportedToken
        | TK::Error_UnexpectedCloser
        | TK::Error_UnsafeCharacterEncoding
        | TK::Error_UnexpectedCommentCloser => &PrefixErrorParselet {},


        TK::BarGreater
        | TK::CloseCurly
        | TK::CloseParen
        | TK::CloseSquare
        | TK::LongName_CloseCurlyDoubleQuote
        | TK::LongName_CloseCurlyQuote
        | TK::LongName_RightAngleBracket
        | TK::LongName_RightAssociation
        | TK::LongName_RightBracketingBar
        | TK::LongName_RightCeiling
        | TK::LongName_RightDoubleBracket
        | TK::LongName_RightDoubleBracketingBar
        | TK::LongName_RightFloor => &PrefixCloserParselet {},


        TK::Minus      => &crate::parse::token_parselets::PREFIX_MINUS,
        TK::Plus       => &crate::parse::token_parselets::PREFIX_PLUS,
        TK::Bang       => &crate::parse::token_parselets::PREFIX_BANG,
        TK::PlusPlus   => &crate::parse::token_parselets::PREFIX_PLUSPLUS,
        TK::MinusMinus => &crate::parse::token_parselets::PREFIX_MINUSMINUS,

        TK::BangBang => &crate::parse::token_parselets::PREFIX_BANGBANG,

        TK::LongName_PlusMinus            => &crate::parse::token_parselets::PREFIX_LONGNAME_PLUSMINUS,
        TK::LongName_Sum                  => &crate::parse::token_parselets::PREFIX_LONGNAME_SUM,
        TK::LongName_Not                  => &crate::parse::token_parselets::PREFIX_LONGNAME_NOT,
        TK::LongName_Sqrt                 => &crate::parse::token_parselets::PREFIX_LONGNAME_SQRT,
        TK::LongName_MinusPlus            => &crate::parse::token_parselets::PREFIX_LONGNAME_MINUSPLUS,
        TK::LongName_DifferentialD        => &crate::parse::token_parselets::PREFIX_LONGNAME_DIFFERENTIALD,
        TK::LongName_CapitalDifferentialD => &crate::parse::token_parselets::PREFIX_LONGNAME_CAPITALDIFFERENTIALD,
        TK::LongName_Minus                => &crate::parse::token_parselets::PREFIX_LONGNAME_MINUS_OP,
        TK::LongName_Del                  => &crate::parse::token_parselets::PREFIX_LONGNAME_DEL,
        TK::LongName_Square               => &crate::parse::token_parselets::PREFIX_LONGNAME_SQUARE,


        TK::Comma
        | TK::LongName_InvisibleComma => &PrefixCommaParselet {},


        TK::LongName_Product                   => &crate::parse::token_parselets::PREFIX_LONGNAME_PRODUCT,
        TK::LongName_ContinuedFractionK        => &crate::parse::token_parselets::PREFIX_LONGNAME_CONTINUEDFRACTIONK,
        TK::LongName_CircleTimes               => &crate::parse::token_parselets::PREFIX_LONGNAME_CIRCLETIMES,
        TK::LongName_ForAll                    => &crate::parse::token_parselets::PREFIX_LONGNAME_FORALL,
        TK::LongName_Exists                    => &crate::parse::token_parselets::PREFIX_LONGNAME_EXISTS,
        TK::LongName_NotExists                 => &crate::parse::token_parselets::PREFIX_LONGNAME_NOTEXISTS,
        TK::LongName_Coproduct                 => &crate::parse::token_parselets::PREFIX_LONGNAME_COPRODUCT,
        TK::LongName_Piecewise                 => &crate::parse::token_parselets::PREFIX_LONGNAME_PIECEWISE,
        TK::LongName_InvisiblePrefixScriptBase => &crate::parse::token_parselets::PREFIX_LONGNAME_INVISIBLEPREFIXSCRIPTBASE,
        TK::LongName_ExpectationE              => &crate::parse::token_parselets::PREFIX_LONGNAME_EXPECTATIONE,
        TK::LongName_CubeRoot                  => &crate::parse::token_parselets::PREFIX_LONGNAME_CUBEROOT,
        TK::LongName_ProbabilityPr             => &crate::parse::token_parselets::PREFIX_LONGNAME_PROBABILITYPR,

        TK::LinearSyntax_Bang => &crate::parse::token_parselets::PREFIX_LINEARSYNTAX_BANG,
        | TK::LinearSyntax_At
        | TK::LinearSyntax_Amp
        | TK::LinearSyntax_Star
        | TK::LinearSyntax_Under
        | TK::LinearSyntax_Caret
        | TK::LinearSyntax_Space
        | TK::LinearSyntax_Percent
        | TK::LinearSyntax_Plus
        | TK::LinearSyntax_Slash
        | TK::LinearSyntax_BackTick
        | TK::LinearSyntax_CloseParen => &PrefixUnsupportedTokenParselet {},


        //
        // Groups
        //
        TK::OpenParen                        => &crate::parse::token_parselets::GROUP_PAREN,
        TK::OpenSquare                       => &crate::parse::token_parselets::GROUP_SQUARE,
        TK::OpenCurly                        => &crate::parse::token_parselets::GROUP_CURLY,
        TK::LessBar                          => &crate::parse::token_parselets::GROUP_LESSBAR,
        TK::ColonColonOpenSquare             => &crate::parse::token_parselets::GROUP_COLONCOLONOPENSQUARE,
        TK::LongName_LeftAngleBracket        => &crate::parse::token_parselets::GROUP_LEFTANGLEBRACKET,
        TK::LongName_LeftCeiling             => &crate::parse::token_parselets::GROUP_LEFTCEILING,
        TK::LongName_LeftFloor               => &crate::parse::token_parselets::GROUP_LEFTFLOOR,
        TK::LongName_LeftDoubleBracket       => &crate::parse::token_parselets::GROUP_LEFTDOUBLEBRACKET,
        TK::LongName_LeftBracketingBar       => &crate::parse::token_parselets::GROUP_LEFTBRACKETINGBAR,
        TK::LongName_LeftDoubleBracketingBar => &crate::parse::token_parselets::GROUP_LEFTDOUBLEBRACKETINGBAR,
        TK::LongName_LeftAssociation         => &crate::parse::token_parselets::GROUP_LEFTASSOCIATION,
        TK::LongName_OpenCurlyQuote          => &crate::parse::token_parselets::GROUP_OPENCURLYQUOTE,
        TK::LongName_OpenCurlyDoubleQuote    => &crate::parse::token_parselets::GROUP_OPENCURLYDOUBLEQUOTE,

        //----------------------------
        // Special
        //----------------------------

        //
        // context sensitive parsing of  x_
        //
        TK::Symbol => &SymbolParselet {},

        //
        // context sensitive parsing of _x
        //
        TK::Under           => &crate::parse::token_parselets::under1Parselet,
        TK::UnderUnder      => &crate::parse::token_parselets::under2Parselet,
        TK::UnderUnderUnder => &crate::parse::token_parselets::under3Parselet,

        TK::UnderDot => &UnderDotParselet {},


        TK::Hash => &HashParselet {},
        TK::HashHash => &HashHashParselet {},

        TK::Percent => &PercentParselet {},
        TK::PercentPercent => &LeafParselet {},

        // prefix, infix, postfix
        TK::SemiSemi => &SemiSemiParselet {},

        //
        // Has to handle \[Integral] f \[DifferentialD] x
        //
        TK::LongName_Integral                        => &crate::parse::token_parselets::INTEGRAL_INTEGRATE,
        TK::LongName_ContourIntegral                 => &crate::parse::token_parselets::INTEGRAL_CONTOUR,
        TK::LongName_DoubleContourIntegral           => &crate::parse::token_parselets::INTEGRAL_DOUBLECONTOUR,
        TK::LongName_ClockwiseContourIntegral        => &crate::parse::token_parselets::INTEGRAL_CLOCKWISECONTOUR,
        TK::LongName_CounterClockwiseContourIntegral => &crate::parse::token_parselets::INTEGRAL_COUNTERCLOCKWISECONTOUR,

        // stringify next token (as a file]
        TK::LessLess => &LessLessParselet {},


        TK::QuestionQuestion => &PrefixUnsupportedTokenParselet {},

        // Also use for operators that are only valid in StandardForm.
        // e.g., \[Limit] does not have an interpretation in InputForm
        //
        // \[Limit] is not letterlike, so it needs some kind of categorization,
        // but it also needs to be prevented from making any valid parses.
        TK::LongName_Limit
        | TK::LongName_MaxLimit
        | TK::LongName_MinLimit => &PrefixUnsupportedTokenParselet {},

        // technically, \[AutoLeftMatch] foo \[AutoRightMatch] does parse as
        // AutoMatch[foo] in InputForm but this is not documented,
        // and I'm not going to support it
        TK::LongName_AutoLeftMatch
        | TK::LongName_AutoRightMatch
        | TK::LongName_DiscreteShift
        | TK::LongName_DifferenceDelta
        | TK::LongName_DiscreteRatio
        | TK::LongName_PartialD => &PrefixUnsupportedTokenParselet {},


        _ => &PrefixUnhandledParselet {},
    } }}
}

//======================================
// Infix Parselets
//======================================

macro_rules! token_kind_to_infix_parselet {
    ($ty:ty; $kind:ident) => {{

    use crate::{
        tokenize::TokenKind as TK,
        parse::{parselet::*, operators::{BinaryOperator, InfixOperator, PostfixOperator}},
        precedence::Precedence
    };

    // Helper macro for creating static parselets in match arms
    macro_rules! static_parselet {
        ($parselet_type:ty) => {{
            static PARSELET: $parselet_type = <$parselet_type>::new();
            &PARSELET
        }};
        ($parselet_type:ty, $arg1:expr) => {{
            static PARSELET: $parselet_type = <$parselet_type>::new($arg1);
            &PARSELET
        }};
        ($parselet_type:ty, $arg1:expr, $arg2:expr) => {{
            static PARSELET: $parselet_type = <$parselet_type>::new($arg1, $arg2);
            &PARSELET
        }};
        ($parselet_type:ty, $arg1:expr, $arg2:expr, $arg3:expr) => {{
            static PARSELET: $parselet_type = <$parselet_type>::new($arg1, $arg2, $arg3);
            &PARSELET
        }}
    }

    match $kind {
        TK::EndOfFile => &InfixAssertFalseParselet {},

        TK::Unknown
        | TK::Whitespace
        | TK::InternalNewline
        | TK::Comment => &InfixAssertFalseParselet {},

        TK::ToplevelNewline => &InfixToplevelNewlineParselet {},


        TK::Error_ExpectedEqual
        | TK::Error_Number
        | TK::Error_UnhandledCharacter
        | TK::Error_ExpectedLetterlike
        | TK::Error_Aborted
        | TK::Error_ExpectedOperand
        | TK::Error_ExpectedTag
        | TK::Error_ExpectedFile
        | TK::Error_UnterminatedComment
        | TK::Error_UnterminatedString
        | TK::Error_UnterminatedFileString
        | TK::Error_UnterminatedLinearSyntaxBlob
        | TK::Error_UnsupportedToken
        | TK::Error_UnexpectedCloser
        | TK::Error_UnsafeCharacterEncoding
        | TK::Error_UnexpectedCommentCloser => &InfixAssertFalseParselet {},

        TK::BarGreater
        | TK::CloseCurly
        | TK::CloseParen
        | TK::CloseSquare
        | TK::LongName_CloseCurlyDoubleQuote
        | TK::LongName_CloseCurlyQuote
        | TK::LongName_RightAngleBracket
        | TK::LongName_RightAssociation
        | TK::LongName_RightBracketingBar
        | TK::LongName_RightCeiling
        | TK::LongName_RightDoubleBracket
        | TK::LongName_RightDoubleBracketingBar
        | TK::LongName_RightFloor => &InfixAssertFalseParselet {},

        TK::LongName_DifferentialD
        | TK::LongName_CapitalDifferentialD => &InfixDifferentialDParselet {},


        //
        // Binary
        //

        TK::Slash            => static_parselet!(BinaryOperatorParselet, Precedence::SLASH, BinaryOperator::Divide),

        TK::Caret            => static_parselet!(BinaryOperatorParselet, Precedence::CARET, BinaryOperator::Power),

        TK::CaretEqual       => static_parselet!(BinaryOperatorParselet, Precedence::CARETEQUAL, BinaryOperator::UpSet),

        TK::CaretColonEqual  => static_parselet!(BinaryOperatorParselet, Precedence::CARETCOLONEQUAL, BinaryOperator::UpSetDelayed),

        TK::SlashAt          => static_parselet!(BinaryOperatorParselet, Precedence::SLASHAT, BinaryOperator::Map),

        TK::MinusGreater     => static_parselet!(BinaryOperatorParselet, Precedence::MINUSGREATER, BinaryOperator::Rule),
        TK::AtAt             => static_parselet!(BinaryOperatorParselet, Precedence::ATAT, BinaryOperator::Apply),
        TK::SlashSemi        => static_parselet!(BinaryOperatorParselet, Precedence::SLASHSEMI, BinaryOperator::Condition),
        TK::SlashDot         => static_parselet!(BinaryOperatorParselet, Precedence::SLASHDOT, BinaryOperator::ReplaceAll),
        TK::ColonGreater     => static_parselet!(BinaryOperatorParselet, Precedence::COLONGREATER, BinaryOperator::RuleDelayed),
        TK::SlashSlashDot    => static_parselet!(BinaryOperatorParselet, Precedence::SLASHSLASHDOT, BinaryOperator::ReplaceRepeated),
        TK::PlusEqual        => static_parselet!(BinaryOperatorParselet, Precedence::PLUSEQUAL, BinaryOperator::AddTo),
        TK::StarEqual        => static_parselet!(BinaryOperatorParselet, Precedence::STAREQUAL, BinaryOperator::TimesBy),
        TK::MinusEqual       => static_parselet!(BinaryOperatorParselet, Precedence::MINUSEQUAL, BinaryOperator::SubtractFrom),
        TK::SlashEqual       => static_parselet!(BinaryOperatorParselet, Precedence::SLASHEQUAL, BinaryOperator::DivideBy),
        TK::LessMinusGreater => static_parselet!(BinaryOperatorParselet, Precedence::LESSMINUSGREATER, BinaryOperator::TwoWayRule),
        TK::SlashSlashAt     => static_parselet!(BinaryOperatorParselet, Precedence::SLASHSLASHAT, BinaryOperator::MapAll),
        TK::At               => static_parselet!(BinaryOperatorParselet, Precedence::AT, BinaryOperator::CodeParser_BinaryAt),
        TK::AtAtAt           => static_parselet!(BinaryOperatorParselet, Precedence::ATATAT, BinaryOperator::MapApply),
        TK::SlashSlash       => static_parselet!(BinaryOperatorParselet, Precedence::SLASHSLASH, BinaryOperator::CodeParser_BinarySlashSlash),
        TK::Question         => static_parselet!(BinaryOperatorParselet, Precedence::INFIX_QUESTION, BinaryOperator::PatternTest),
        TK::BarMinusGreater  => static_parselet!(BinaryOperatorParselet, Precedence::BARMINUSGREATER, BinaryOperator::Function),
        TK::SlashSlashEqual  => static_parselet!(BinaryOperatorParselet, Precedence::SLASHSLASHEQUAL, BinaryOperator::ApplyTo),

        TK::LongName_Divide               => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DIVIDE, BinaryOperator::Divide),
        TK::LongName_DivisionSlash        => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DIVISIONSLASH, BinaryOperator::Divide),
        TK::LongName_Implies              => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_IMPLIES, BinaryOperator::Implies),
        TK::LongName_RoundImplies         => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_ROUNDIMPLIES, BinaryOperator::RoundImplies),
        TK::LongName_PlusMinus            => static_parselet!(BinaryOperatorParselet, Precedence::INFIX_LONGNAME_PLUSMINUS, BinaryOperator::PlusMinus),
        TK::LongName_DirectedEdge         => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DIRECTEDEDGE, BinaryOperator::DirectedEdge),
        TK::LongName_Rule                 => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_RULE, BinaryOperator::Rule),
        TK::LongName_RuleDelayed          => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_RULEDELAYED, BinaryOperator::RuleDelayed),
        TK::LongName_UndirectedEdge       => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_UNDIRECTEDEDGE, BinaryOperator::UndirectedEdge),
        TK::LongName_Function             => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_FUNCTION, BinaryOperator::Function),
        TK::LongName_MinusPlus            => static_parselet!(BinaryOperatorParselet, Precedence::INFIX_LONGNAME_MINUSPLUS, BinaryOperator::MinusPlus),
        TK::LongName_TwoWayRule           => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_TWOWAYRULE, BinaryOperator::TwoWayRule),
        TK::LongName_InvisibleApplication => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_INVISIBLEAPPLICATION, BinaryOperator::CodeParser_BinaryAt),
        TK::LongName_CircleMinus          => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_CIRCLEMINUS, BinaryOperator::CircleMinus),
        TK::LongName_SuchThat             => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_SUCHTHAT, BinaryOperator::SuchThat),
        TK::LongName_Perpendicular        => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_PERPENDICULAR, BinaryOperator::Perpendicular),
        TK::LongName_Because              => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_BECAUSE, BinaryOperator::Because),
        TK::LongName_Therefore            => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_THEREFORE, BinaryOperator::Therefore),
        TK::LongName_RightTee             => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_RIGHTTEE, BinaryOperator::RightTee),
        TK::LongName_LeftTee              => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_LEFTTEE, BinaryOperator::LeftTee),
        TK::LongName_DoubleRightTee       => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DOUBLERIGHTTEE, BinaryOperator::DoubleRightTee),
        TK::LongName_DoubleLeftTee        => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DOUBLELEFTTEE, BinaryOperator::DoubleLeftTee),
        TK::LongName_UpTee                => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_UPTEE, BinaryOperator::UpTee),
        TK::LongName_DownTee              => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_DOWNTEE, BinaryOperator::DownTee),
        TK::LongName_Application          => static_parselet!(BinaryOperatorParselet, Precedence::LONGNAME_APPLICATION, BinaryOperator::Application),

        //
        // Infix
        //
        // Note that these are the operators that make sense to be infix in WL source code.
        //
        // These may not necessarily correspond to Flat functions in WL.
        //
        TK::Minus           => static_parselet!(InfixOperatorParselet, Precedence::INFIX_MINUS, InfixOperator::Plus),
        TK::EqualEqualEqual => static_parselet!(InfixOperatorParselet, Precedence::EQUALEQUALEQUAL, InfixOperator::SameQ),
        TK::EqualBangEqual  => static_parselet!(InfixOperatorParselet, Precedence::EQUALBANGEQUAL, InfixOperator::UnsameQ),
        TK::Plus            => static_parselet!(InfixOperatorParselet, Precedence::INFIX_PLUS, InfixOperator::Plus),
        TK::Dot             => static_parselet!(InfixOperatorParselet, Precedence::DOT, InfixOperator::Dot),
        TK::StarStar        => static_parselet!(InfixOperatorParselet, Precedence::STARSTAR, InfixOperator::NonCommutativeMultiply),
        TK::AmpAmp          => static_parselet!(InfixOperatorParselet, Precedence::AMPAMP, InfixOperator::And),
        TK::BarBar          => static_parselet!(InfixOperatorParselet, Precedence::BARBAR, InfixOperator::Or),
        TK::Bar             => static_parselet!(InfixOperatorParselet, Precedence::BAR, InfixOperator::Alternatives),
        TK::LessGreater     => static_parselet!(InfixOperatorParselet, Precedence::LESSGREATER, InfixOperator::StringJoin),
        TK::TildeTilde      => static_parselet!(InfixOperatorParselet, Precedence::TILDETILDE, InfixOperator::StringExpression),
        TK::AtStar          => static_parselet!(InfixOperatorParselet, Precedence::ATSTAR, InfixOperator::Composition),
        TK::SlashStar       => static_parselet!(InfixOperatorParselet, Precedence::SLASHSTAR, InfixOperator::RightComposition),
        //
        // Times
        //
        TK::Star
        | TK::LongName_Times
        | TK::LongName_InvisibleTimes
        | TK::Fake_ImplicitTimes => &TimesParselet {},


        //
        // Set relations
        //
        TK::LongName_Element                => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::Element),
        TK::LongName_Subset                 => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::Subset),
        TK::LongName_Superset               => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::Superset),
        TK::LongName_SubsetEqual            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SubsetEqual),
        TK::LongName_SupersetEqual          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SupersetEqual),
        TK::LongName_NotElement             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotElement),
        TK::LongName_NotSubset              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSubset),
        TK::LongName_NotSuperset            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSuperset),
        TK::LongName_NotSubsetEqual         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSubsetEqual),
        TK::LongName_NotSupersetEqual       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSupersetEqual),
        TK::LongName_SquareSubset           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SquareSubset),
        TK::LongName_SquareSuperset         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SquareSuperset),
        TK::LongName_NotSquareSubset        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSquareSubset),
        TK::LongName_NotSquareSuperset      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSquareSuperset),
        TK::LongName_SquareSubsetEqual      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SquareSubsetEqual),
        TK::LongName_SquareSupersetEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::SquareSupersetEqual),
        TK::LongName_NotSquareSubsetEqual   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSquareSubsetEqual),
        TK::LongName_NotSquareSupersetEqual => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotSquareSupersetEqual),
        TK::LongName_ReverseElement         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::ReverseElement),
        TK::LongName_NotReverseElement      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::NotReverseElement),
        TK::LongName_Distributed            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_SETRELATIONS, InfixOperator::Distributed),

        TK::LongName_ImplicitPlus => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_IMPLICITPLUS, InfixOperator::Plus),
        TK::LongName_And          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_AND, InfixOperator::And),
        TK::LongName_Or           => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_OR, InfixOperator::Or),
        TK::LongName_Xor          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_XOR, InfixOperator::Xor),
        TK::LongName_Nand         => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_NAND, InfixOperator::Nand),
        TK::LongName_Nor          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_NOR, InfixOperator::Nor),
        //
        // Horizontal arrows
        //
        TK::LongName_LeftArrow            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::LeftArrow),
        TK::LongName_RightArrow           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::RightArrow),
        TK::LongName_LeftRightArrow       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::LeftRightArrow),
        TK::LongName_LeftTeeArrow         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::LeftTeeArrow),
        TK::LongName_RightTeeArrow        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::RightTeeArrow),
        TK::LongName_RightArrowLeftArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::RightArrowLeftArrow),
        TK::LongName_LeftArrowRightArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::LeftArrowRightArrow),
        TK::LongName_DoubleLeftArrow      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::DoubleLeftArrow),
        TK::LongName_DoubleRightArrow     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::DoubleRightArrow),
        TK::LongName_DoubleLeftRightArrow => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::DoubleLeftRightArrow),
        TK::LongName_LeftArrowBar         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::LeftArrowBar),
        TK::LongName_RightArrowBar        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::RightArrowBar),
        TK::LongName_ShortRightArrow      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::ShortRightArrow),
        TK::LongName_ShortLeftArrow       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_HORIZONTALARROWS, InfixOperator::ShortLeftArrow),
        //
        // Diagonal arrow operators
        //
        TK::LongName_UpperLeftArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_DIAGONALARROWOPERATORS, InfixOperator::UpperLeftArrow),
        TK::LongName_UpperRightArrow => static_parselet!(InfixOperatorParselet, Precedence::CLASS_DIAGONALARROWOPERATORS, InfixOperator::UpperRightArrow),
        TK::LongName_LowerRightArrow => static_parselet!(InfixOperatorParselet, Precedence::CLASS_DIAGONALARROWOPERATORS, InfixOperator::LowerRightArrow),
        TK::LongName_LowerLeftArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_DIAGONALARROWOPERATORS, InfixOperator::LowerLeftArrow),
        //
        // Vector operators
        //
        TK::LongName_LeftVector          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::LeftVector),
        TK::LongName_RightVector         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::RightVector),
        TK::LongName_LeftRightVector     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::LeftRightVector),
        TK::LongName_LeftVectorBar       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::LeftVectorBar),
        TK::LongName_RightVectorBar      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::RightVectorBar),
        TK::LongName_LeftTeeVector       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::LeftTeeVector),
        TK::LongName_RightTeeVector      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::RightTeeVector),
        TK::LongName_DownLeftVector      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownLeftVector),
        TK::LongName_DownRightVector     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownRightVector),
        TK::LongName_DownLeftRightVector => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownLeftRightVector),
        TK::LongName_DownLeftVectorBar   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownLeftVectorBar),
        TK::LongName_DownRightVectorBar  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownRightVectorBar),
        TK::LongName_DownLeftTeeVector   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownLeftTeeVector),
        TK::LongName_DownRightTeeVector  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VECTOROPERATORS, InfixOperator::DownRightTeeVector),
        //
        // Vertical arrow operators
        //
        TK::LongName_UpArrow           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::UpArrow),
        TK::LongName_DownArrow         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DownArrow),
        TK::LongName_UpDownArrow       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::UpDownArrow),
        TK::LongName_UpTeeArrow        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::UpTeeArrow),
        TK::LongName_DownTeeArrow      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DownTeeArrow),
        TK::LongName_UpArrowDownArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::UpArrowDownArrow),
        TK::LongName_DoubleUpArrow     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleUpArrow),
        TK::LongName_DoubleDownArrow   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleDownArrow),
        TK::LongName_DoubleUpDownArrow => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleUpDownArrow),
        TK::LongName_DownArrowUpArrow  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DownArrowUpArrow),        //
        // itai asking about precedence of "long" arrows:
        // https://mail-archive.wolfram.com/archive/l-typeset/2021/Jul00/0000.html
        //
        TK::LongName_LongLeftArrow            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::LongLeftArrow),
        TK::LongName_LongRightArrow           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::LongRightArrow),
        TK::LongName_LongLeftRightArrow       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::LongLeftRightArrow),
        TK::LongName_DoubleLongLeftArrow      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleLongLeftArrow),
        TK::LongName_DoubleLongRightArrow     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleLongRightArrow),
        TK::LongName_DoubleLongLeftRightArrow => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DoubleLongLeftRightArrow),
        TK::LongName_UpArrowBar               => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::UpArrowBar),
        TK::LongName_DownArrowBar             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::DownArrowBar),
        TK::LongName_ShortUpArrow             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::ShortUpArrow),
        TK::LongName_ShortDownArrow           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALARROWOPERATORS, InfixOperator::ShortDownArrow),

        //
        // Vertical vector operators
        //
        TK::LongName_RightUpVector        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightUpVector),
        TK::LongName_LeftUpVector         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftUpVector),
        TK::LongName_RightDownVector      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightDownVector),
        TK::LongName_LeftDownVector       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftDownVector),
        TK::LongName_RightUpDownVector    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightUpDownVector),
        TK::LongName_LeftUpDownVector     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftUpDownVector),
        TK::LongName_RightUpVectorBar     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightUpVectorBar),
        TK::LongName_RightDownVectorBar   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightDownVectorBar),
        TK::LongName_LeftUpVectorBar      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftUpVectorBar),
        TK::LongName_LeftDownVectorBar    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftDownVectorBar),
        TK::LongName_RightUpTeeVector     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightUpTeeVector),
        TK::LongName_RightDownTeeVector   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::RightDownTeeVector),
        TK::LongName_LeftUpTeeVector      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftUpTeeVector),
        TK::LongName_LeftDownTeeVector    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::LeftDownTeeVector),
        TK::LongName_UpEquilibrium        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::UpEquilibrium),
        TK::LongName_ReverseUpEquilibrium => static_parselet!(InfixOperatorParselet, Precedence::CLASS_VERTICALVECTOROPERATORS, InfixOperator::ReverseUpEquilibrium),


        TK::LongName_CenterDot   => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CENTERDOT, InfixOperator::CenterDot),
        TK::LongName_Equivalent  => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_EQUIVALENT, InfixOperator::Equivalent),
        TK::LongName_CircleDot   => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CIRCLEDOT, InfixOperator::CircleDot),
        TK::LongName_Conditioned => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CONDITIONED, InfixOperator::Conditioned),
        //
        // Union operators
        //
        TK::LongName_Union       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_UNIONOPERATORS, InfixOperator::Union),
        TK::LongName_SquareUnion => static_parselet!(InfixOperatorParselet, Precedence::CLASS_UNIONOPERATORS, InfixOperator::SquareUnion),
        TK::LongName_UnionPlus   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_UNIONOPERATORS, InfixOperator::UnionPlus),
        //
        // Intersection operators
        //
        TK::LongName_Intersection       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INTERSECTIONOPERATORS, InfixOperator::Intersection),
        TK::LongName_SquareIntersection => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INTERSECTIONOPERATORS, InfixOperator::SquareIntersection),


        TK::LongName_TensorWedge          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_TENSORWEDGE, InfixOperator::TensorWedge),
        TK::LongName_TensorProduct        => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_TENSORPRODUCT, InfixOperator::TensorProduct),
        TK::LongName_Cross                => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CROSS, InfixOperator::Cross),
        TK::LongName_SmallCircle          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_SMALLCIRCLE, InfixOperator::SmallCircle),
        TK::LongName_Divides              => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_DIVIDES, InfixOperator::Divisible),
        TK::LongName_VerticalSeparator    => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_VERTICALSEPARATOR, InfixOperator::VerticalSeparator),
        TK::LongName_Backslash            => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_BACKSLASH, InfixOperator::Backslash),
        TK::LongName_Diamond              => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_DIAMOND, InfixOperator::Diamond),
        TK::LongName_Wedge                => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_WEDGE, InfixOperator::Wedge),
        TK::LongName_Vee                  => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_VEE, InfixOperator::Vee),
        TK::LongName_CircleTimes          => static_parselet!(InfixOperatorParselet, Precedence::INFIX_LONGNAME_CIRCLETIMES, InfixOperator::CircleTimes),
        TK::LongName_Star                 => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_STAR, InfixOperator::Star),
        TK::LongName_VerticalTilde        => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_VERTICALTILDE, InfixOperator::VerticalTilde),
        TK::LongName_Coproduct            => static_parselet!(InfixOperatorParselet, Precedence::INFIX_LONGNAME_COPRODUCT, InfixOperator::Coproduct),
        TK::LongName_Cap                  => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CAP, InfixOperator::Cap),
        TK::LongName_Cup                  => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CUP, InfixOperator::Cup),
        TK::LongName_CirclePlus           => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_CIRCLEPLUS, InfixOperator::CirclePlus),
        TK::LongName_VerticalBar          => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_VERTICALBAR, InfixOperator::VerticalBar),
        TK::LongName_DoubleVerticalBar    => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_DOUBLEVERTICALBAR, InfixOperator::DoubleVerticalBar),
        TK::LongName_NotVerticalBar       => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_NOTVERTICALBAR, InfixOperator::NotVerticalBar),
        TK::LongName_NotDoubleVerticalBar => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_NOTDOUBLEVERTICALBAR, InfixOperator::NotDoubleVerticalBar),
        //
        // Ordering operators
        //
        TK::LongName_LeftTriangle          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::LeftTriangle),
        TK::LongName_RightTriangle         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::RightTriangle),
        TK::LongName_NotLeftTriangle       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotLeftTriangle),
        TK::LongName_NotRightTriangle      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotRightTriangle),
        TK::LongName_LeftTriangleEqual     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::LeftTriangleEqual),
        TK::LongName_RightTriangleEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::RightTriangleEqual),
        TK::LongName_NotLeftTriangleEqual  => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotLeftTriangleEqual),
        TK::LongName_NotRightTriangleEqual => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotRightTriangleEqual),
        TK::LongName_LeftTriangleBar       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::LeftTriangleBar),
        TK::LongName_RightTriangleBar      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::RightTriangleBar),
        TK::LongName_NotLeftTriangleBar    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotLeftTriangleBar),
        TK::LongName_NotRightTriangleBar   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotRightTriangleBar),
        TK::LongName_TildeEqual            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::TildeEqual),
        TK::LongName_NotTildeEqual         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotTildeEqual),
        TK::LongName_TildeFullEqual        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::TildeFullEqual),
        TK::LongName_NotTildeFullEqual     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotTildeFullEqual),
        TK::LongName_Tilde                 => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Tilde),
        TK::LongName_NotTilde              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotTilde),
        TK::LongName_EqualTilde            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::EqualTilde),
        TK::LongName_NotEqualTilde         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotEqualTilde),
        TK::LongName_TildeTilde            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::TildeTilde),
        TK::LongName_NotTildeTilde         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotTildeTilde),
        TK::LongName_Proportional          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Proportional),
        TK::LongName_Proportion            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Proportion),
        TK::LongName_Congruent             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Congruent),
        TK::LongName_NotCongruent          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotCongruent),
        TK::LongName_Equilibrium           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Equilibrium),
        TK::LongName_ReverseEquilibrium    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::ReverseEquilibrium),
        TK::LongName_DotEqual              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::DotEqual),
        TK::LongName_Precedes              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Precedes),
        TK::LongName_Succeeds              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::Succeeds),
        TK::LongName_PrecedesEqual         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::PrecedesEqual),
        TK::LongName_SucceedsEqual         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::SucceedsEqual),
        TK::LongName_PrecedesTilde         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::PrecedesTilde),
        TK::LongName_SucceedsTilde         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::SucceedsTilde),
        TK::LongName_PrecedesSlantEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::PrecedesSlantEqual),
        TK::LongName_SucceedsSlantEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::SucceedsSlantEqual),
        TK::LongName_NotPrecedes           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotPrecedes),
        TK::LongName_NotSucceeds           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotSucceeds),
        TK::LongName_NotPrecedesEqual      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotPrecedesEqual),
        TK::LongName_NotSucceedsEqual      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotSucceedsEqual),
        TK::LongName_NotPrecedesTilde      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotPrecedesTilde),
        TK::LongName_NotSucceedsTilde      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotSucceedsTilde),
        TK::LongName_NotPrecedesSlantEqual => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotPrecedesSlantEqual),
        TK::LongName_NotSucceedsSlantEqual => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotSucceedsSlantEqual),
        TK::LongName_CupCap                => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::CupCap),
        TK::LongName_NotCupCap             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotCupCap),
        TK::LongName_HumpEqual             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::HumpEqual),
        TK::LongName_HumpDownHump          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::HumpDownHump),
        TK::LongName_NotHumpEqual          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotHumpEqual),
        TK::LongName_NotHumpDownHump       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_ORDERINGOPERATORS, InfixOperator::NotHumpDownHump),
        //
        // special Inequality
        //
        TK::BangEqual                        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::EqualEqual                       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::Greater                          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::GreaterEqual                     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LessEqual                        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::Less                             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_Equal                   => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterEqual            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterEqualLess        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterFullEqual        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterGreater          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterLess             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterSlantEqual       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_GreaterTilde            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessEqual               => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessEqualGreater        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessFullEqual           => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessGreater             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessLess                => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessSlantEqual          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LessTilde               => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_LongEqual               => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NestedGreaterGreater    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NestedLessLess          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotEqual                => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreater              => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterEqual         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterFullEqual     => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterGreater       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterLess          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterSlantEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotGreaterTilde         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLess                 => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessEqual            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessFullEqual        => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessGreater          => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessLess             => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessSlantEqual       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotLessTilde            => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotNestedGreaterGreater => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_NotNestedLessLess       => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),        //
        // special VectorInequality
        //
        TK::LongName_VectorGreater      => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_VectorGreaterEqual => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_VectorLess         => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),
        TK::LongName_VectorLessEqual    => static_parselet!(InfixOperatorParselet, Precedence::CLASS_INEQUALITY, InfixOperator::CodeParser_InfixInequality),


        TK::LongName_PermutationProduct => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_PERMUTATIONPRODUCT, InfixOperator::PermutationProduct),
        TK::LongName_Colon              => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_COLON, InfixOperator::Colon),
        TK::LongName_Xnor               => static_parselet!(InfixOperatorParselet, Precedence::LONGNAME_XNOR, InfixOperator::Xnor),
        TK::LongName_Minus              => static_parselet!(InfixOperatorParselet, Precedence::INFIX_LONGNAME_MINUS, InfixOperator::Plus),

        //
        // Postfix
        //
        TK::Amp                                 => static_parselet!(PostfixOperatorParselet, Precedence::AMP, PostfixOperator::Function),
        TK::DotDot                              => static_parselet!(PostfixOperatorParselet, Precedence::DOTDOT, PostfixOperator::Repeated),
        TK::Bang                                => static_parselet!(PostfixOperatorParselet, Precedence::POSTFIX_BANG, PostfixOperator::Factorial),
        TK::MinusMinus                          => static_parselet!(PostfixOperatorParselet, Precedence::POSTFIX_MINUSMINUS, PostfixOperator::Decrement),
        TK::PlusPlus                            => static_parselet!(PostfixOperatorParselet, Precedence::POSTFIX_PLUSPLUS, PostfixOperator::Increment),
        TK::DotDotDot                           => static_parselet!(PostfixOperatorParselet, Precedence::DOTDOTDOT, PostfixOperator::RepeatedNull),
        TK::BangBang                            => static_parselet!(PostfixOperatorParselet, Precedence::POSTFIX_BANGBANG, PostfixOperator::Factorial2),
        TK::SingleQuote                         => static_parselet!(PostfixOperatorParselet, Precedence::SINGLEQUOTE, PostfixOperator::Derivative),
        TK::LongName_Transpose                  => static_parselet!(PostfixOperatorParselet, Precedence::LONGNAME_TRANSPOSE, PostfixOperator::Transpose),
        TK::LongName_Conjugate                  => static_parselet!(PostfixOperatorParselet, Precedence::LONGNAME_CONJUGATE, PostfixOperator::Conjugate),
        TK::LongName_ConjugateTranspose         => static_parselet!(PostfixOperatorParselet, Precedence::LONGNAME_CONJUGATETRANSPOSE, PostfixOperator::ConjugateTranspose),
        TK::LongName_HermitianConjugate         => static_parselet!(PostfixOperatorParselet, Precedence::LONGNAME_HERMITIANCONJUGATE, PostfixOperator::HermitianConjugate),
        TK::LongName_InvisiblePostfixScriptBase => static_parselet!(PostfixOperatorParselet, Precedence::LONGNAME_INVISIBLEPOSTFIXSCRIPTBASE, PostfixOperator::InvisiblePostfixScriptBase),

        //
        // Calls
        //
        TK::OpenSquare                 => &crate::parse::token_parselets::CALL_SQUARE,
        TK::LongName_LeftDoubleBracket => &crate::parse::token_parselets::CALL_LEFTDOUBLEBRACKET,
        TK::ColonColonOpenSquare       => &crate::parse::token_parselets::CALL_COLONCOLONOPENSQUARE,




        //
        // trailing ; and , is allowed
        //
        TK::Semi => &SemiParselet {},

        TK::Comma => &CommaParselet {},
        TK::LongName_InvisibleComma => &CommaParselet {},

        //
        // prefix, infix, postfix
        //
        TK::SemiSemi => &SemiSemiParselet {},

        //
        // ternary
        //
        TK::Tilde => &TildeParselet {},

        //
        // context sensitive parsing of sym:obj and pat:v
        //
        TK::Colon => &ColonParselet {},

        //
        // ternary, with different possibilities for second operator
        //
        TK::SlashColon => &SlashColonParselet {},

        //
        // Has to handle  a =.  and  a = .
        //
        TK::Equal => &crate::parse::token_parselets::EQUAL_PARSELET,
        TK::ColonEqual => &crate::parse::token_parselets::COLONEQUAL_PARSELET,

        //
        // stringify next token (as a symbol)
        //
        TK::ColonColon => &ColonColonParselet {},

        //
        // stringify next token (as a file)
        //
        TK::GreaterGreater => &GreaterGreaterParselet {},
        TK::GreaterGreaterGreater => &GreaterGreaterGreaterParselet {},


        TK::QuestionQuestion => &InfixAssertFalseParselet {},

        //
        // Also use for operators that are only valid in StandardForm.
        // e.g., \[Limit] does not have an interpretation in InputForm
        //
        // \[Limit] is not letterlike, so it needs some kind of categorization,
        // but it also needs to be prevented from making any valid parses.
        //
        TK::LongName_Limit
        | TK::LongName_MaxLimit
        | TK::LongName_MinLimit => &InfixAssertFalseParselet {},

        //
        // technically, \[AutoLeftMatch] foo \[AutoRightMatch] does parse as
        // AutoMatch[foo] in InputForm but this is not documented,
        // and I'm not going to support it
        //
        | TK::LongName_AutoLeftMatch
        | TK::LongName_AutoRightMatch
        | TK::LongName_DiscreteShift
        | TK::LongName_DifferenceDelta
        | TK::LongName_DiscreteRatio
        | TK::LongName_PartialD => &InfixAssertFalseParselet {},

        // TODO: Debug assert renaming variants are isPossibleBeginning()
        _ => {
            &InfixImplicitTimesParselet {}
        },
    }

    }}
}

pub(crate) use {token_kind_to_infix_parselet, token_kind_to_prefix_parselet};
