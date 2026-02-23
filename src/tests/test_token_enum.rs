use crate::tokenize::TokenKind;


#[test]
fn TokenEnumTest_Trivia() {
    assert!(TokenKind::Whitespace.is_trivia());
}

#[test]
fn TokenEnumTest_PossibleBeginning() {
    assert!(TokenKind::Symbol.is_possible_beginning());

    assert!(TokenKind::SemiSemi.is_possible_beginning());
}

#[test]
fn TokenEnumTest_Closer() {
    assert!(TokenKind::CloseSquare.is_closer());

    assert!(TokenKind::LongName_RightCeiling.is_closer());
}

#[test]
fn TokenEnumTest_Error() {
    assert!(TokenKind::Error_ExpectedTag.is_error());

    assert!(TokenKind::Error_UnsupportedToken.is_error());
}

#[test]
fn TokenEnumTest_Unterminated() {
    assert!(TokenKind::Error_UnterminatedString.is_unterminated());

    assert!(TokenKind::Error_UnterminatedComment.is_unterminated());
}

#[test]
fn TokenEnumTest_Empty() {
    assert!(TokenKind::EndOfFile.is_empty());

    assert!(TokenKind::Error_ExpectedOperand.is_empty());
}
