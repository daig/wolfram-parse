use crate::read::WLCharacter;

#[test]
fn SourceCharacterTest_Graphical1() {
    assert_eq!(WLCharacter::new('\t').graphical_string(), "\\t");

    assert_eq!(WLCharacter::new(0x1b).graphical_string(), "\\[RawEscape]");

    assert_eq!(WLCharacter::new(0xb0).graphical_string(), "\\[Degree]");

    assert_eq!(WLCharacter::new('\u{abcd}').graphical_string(), "\\:abcd");
}
