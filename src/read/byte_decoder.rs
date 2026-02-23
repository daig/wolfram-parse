//! Decode a sequence of UTF-8 encoded bytes into Source characters

//
// https://unicodebook.readthedocs.io/issues.html#strict-utf8-decoder
//
// Table 3.1B. Legal UTF-8 Byte Sequences
// http://www.unicode.org/versions/corrigendum1.html
//
// CODE POINTS         1ST BYTE    2ND BYTE    3RD BYTE    4TH BYTE
//   U+0000....U+007F    00..7F
//   U+0080....U+07FF    C2..DF      80..BF
//   U+0800....U+0FFF        E0      A0..BF      80..BF
//   U+1000....U+FFFF    E1..EF      80..BF      80..BF
//  U+10000...U+3FFFF        F0      90..BF      80..BF      80..BF
//  U+40000...U+FFFFF    F1..F3      80..BF      80..BF      80..BF
// U+100000..U+10FFFF        F4      80..8F      80..BF      80..BF
//
//
// stray surrogates:
//   U+D800....U+DFFF        ED      A0..BF      80..BF
//
// BOM:
//   U+FEFF                  EF      BB          BF
//

use std::num::NonZeroU32;

use crate::{
    issue::{CodeAction, encoding_issue, IssueTag, Severity},
    read::{Reader, SourceManager},
    read::{
        code_point::{CodePoint, CODEPOINT_BOM},
        wl_character::{Escape, WLCharacter},
    },
    source::{
        LineColumn, Location, NextPolicy, NextPolicyBits::*, SourceCharacter,
        SourceConvention, Span,
    },
    tokenize::tokenizer::UnsafeCharacterEncoding,
    utils::{self, non_zero_u32_incr},
    EncodingMode,
};

//
// Precondition: buffer is pointing to current SourceCharacter
// Postcondition: buffer is pointing to next SourceCharacter
//
// return the SourceCharacter that was current
//
// Decode UTF-8 byte sequences
//
// Also decode \r\n into a single SourceCharacter
//
// \r\n is akin to a 2-byte UTF-8 sequence
//
// Also warn about \r line endings
//
// Do not decode unsafe character encodings: incomplete sequences, stray surrogates, or BOM
//
impl Reader<'_> {
pub(crate) fn next_source_character(
    &mut self,
    policy: NextPolicy,
) -> SourceCharacter {
    let first_byte = self.current_byte();

    if 0x20 <= first_byte && first_byte <= 0x7e {
        //
        // Valid
        //

        incr_diagnostic!(ByteDecoder_PrintableCount);

        self.next_byte();

        self.src().increment();

        return SourceCharacter::from_u8(first_byte);
    }

    if first_byte == 0x0a {
        //
        // Handle LF specially
        //

        incr_diagnostic!(ByteDecoder_LineFeedCount);

        self.next_byte();

        self.src().newline();

        return SourceCharacter::from_u8(first_byte);
    }

    return self.next_source_character_uncommon(policy);
}

fn next_source_character_uncommon(
    &mut self,
    policy: NextPolicy,
) -> SourceCharacter {
    let first_byte = self.next_byte();

    match first_byte {
        b'\t' => {

            //
            // Handle TAB specially
            //

            incr_diagnostic!(ByteDecoder_TabCount);

            self.src().tab();

            return SourceCharacter::from_u8(first_byte);

        }
            //
            // Handle CR specially
            //
        b'\r' => {

            incr_diagnostic!(ByteDecoder_CarriageReturnCount);

            if self.current_byte() == b'\n' {

                self.next_byte();

                self.src().windows_newline();

                return SourceCharacter::from(CodePoint::CRLF);
            }

            self.src().newline();

            if self.check_issues && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES) {

                //
                // No CodeAction here
                //

                //
                // FIXME: no way to do endOfPreviousLine()
                //
                let current_source_character_start_loc = self.src_loc;

                let I = encoding_issue(
                    IssueTag::UnexpectedCarriageReturn,
                    format!("Unexpected ``\\r`` character."),
                    Severity::Warning,
                    Span::new(current_source_character_start_loc, self.src_loc),
                    1.0,
                    vec![],
                    vec![]
                );

                self.add_issue(I);
            }

            return SourceCharacter::from('\r');
        }
            //
            // 1 byte UTF-8 sequence
            //
        0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 |
        0x08 | /*   TAB*/ /*    LF*/ 0x0b | 0x0c | /*    CR*/ 0x0e | 0x0f |
        0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17 |
        0x18 | 0x19 | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e | 0x1f |
        0x7f => {

            incr_diagnostic!(ByteDecoder_1ByteCount);

            //
            // Valid
            //

            let first_byte = CodePoint::from(char::from(first_byte));

            return self.valid_strange(first_byte, policy);
        }

            //
            // 2 byte UTF-8 sequence
            //
        /*                 */ 0xc2 | 0xc3 | 0xc4 | 0xc5 | 0xc6 | 0xc7 |
        0xc8 | 0xc9 | 0xca | 0xcb | 0xcc | 0xcd | 0xce | 0xcf |
        0xd0 | 0xd1 | 0xd2 | 0xd3 | 0xd4 | 0xd5 | 0xd6 | 0xd7 |
        0xd8 | 0xd9 | 0xda | 0xdb | 0xdc | 0xdd | 0xde | 0xdf => {

            incr_diagnostic!(ByteDecoder_2ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mark = self.mark();

            let tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0x80 <= second_byte && second_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded = ((u32::from(first_byte) & 0x1f) << 6) | (u32::from(second_byte) & 0x3f);
            let decoded = CodePoint::from_u32(decoded).unwrap();

            return self.valid_mb(decoded, policy);
        }
            //
            // 3 byte UTF-8 sequence
            //
        0xe0 => {

            incr_diagnostic!(ByteDecoder_3ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mut mark = self.mark();

            let mut tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0xa0 <= second_byte && second_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let third_byte = tmp;

            if !(0x80 <= third_byte && third_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded =
                ((u32::from(first_byte) & 0x0f) << 12)
                | ((u32::from(second_byte) & 0x3f) << 6)
                | (u32::from(third_byte) & 0x3f);
            let decoded = CodePoint::from_u32(decoded).unwrap();

            return self.valid_mb(decoded, policy);
        }
            //
            // 3 byte UTF-8 sequence
            //
        /*      */ 0xe1 | 0xe2 | 0xe3 | 0xe4 | 0xe5 | 0xe6 | 0xe7 |
        0xe8 | 0xe9 | 0xea | 0xeb | 0xec | 0xed | 0xee | 0xef => {

            incr_diagnostic!(ByteDecoder_3ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mut mark = self.mark();

            let mut tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0x80 <= second_byte && second_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let third_byte = tmp;

            if !(0x80 <= third_byte && third_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded =
                ((u32::from(first_byte) & 0x0f) << 12)
                | ((u32::from(second_byte) & 0x3f) << 6)
                | (u32::from(third_byte) & 0x3f);

            if utils::is_stray_surrogate(decoded) {

                //
                // Stray surrogate
                //

                return self.stray_surrogate(mark.src_loc, policy);
            }

            let decoded = CodePoint::from_u32(decoded).unwrap();

            if decoded == CODEPOINT_BOM {

                //
                // BOM
                //

                return self.bom(mark.src_loc, policy);
            }

            return self.valid_mb(decoded, policy);
        }
            //
            // 4 byte UTF-8 sequence
            //
        0xf0 => {

            incr_diagnostic!(ByteDecoder_4ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mut mark = self.mark();

            let mut tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0x90 <= second_byte && second_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let third_byte = tmp;

            if !(0x80 <= third_byte && third_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let fourth_byte = tmp;

            if !(0x80 <= fourth_byte && fourth_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded =
                ((u32::from(first_byte) & 0x07) << 18)
                | ((u32::from(second_byte) & 0x3f) << 12)
                | ((u32::from(third_byte) & 0x3f) << 6)
                | ((u32::from(fourth_byte) & 0x3f));
            let decoded = CodePoint::from_u32(decoded).unwrap();

            return self.valid_mb(decoded, policy);
        }
            //
            // 4 byte UTF-8 sequence
            //
        0xf1 | 0xf2 | 0xf3 => {

            incr_diagnostic!(ByteDecoder_4ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mut mark = self.mark();

            let mut tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0x80 <= second_byte && second_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let third_byte = tmp;

            if !(0x80 <= third_byte && third_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let fourth_byte = tmp;

            if !(0x80 <= fourth_byte && fourth_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded: u32 =
                ( (u32::from(first_byte) & 0x07) << 18)
                | ((u32::from(second_byte) & 0x3f) << 12)
                | ((u32::from(third_byte) & 0x3f) << 6)
                | ((u32::from(fourth_byte) & 0x3f));
            let decoded = CodePoint::from_u32(decoded).unwrap();

            return self.valid_mb(decoded, policy);
        }
            //
            // 4 byte UTF-8 sequence
            //
        0xf4 => {

            incr_diagnostic!(ByteDecoder_4ByteCount);

            //
            // Buffer is possibly already pointing to EOF
            //

            let mut mark = self.mark();

            let mut tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let second_byte = tmp;

            if !(0x80 <= second_byte && second_byte <= 0x8f) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_1byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let third_byte = tmp;

            if !(0x80 <= third_byte && third_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_2byte_sequence(mark.src_loc, policy);
            }

            mark = self.mark();

            tmp = self.next_byte();

            if self.was_eof {

                //
                // EOF
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            // Continue

            let fourth_byte = tmp;

            if !(0x80 <= fourth_byte && fourth_byte <= 0xbf) {

                //
                // Incomplete
                //

                self.seek(mark);

                return self.incomplete_3byte_sequence(mark.src_loc, policy);
            }

            //
            // Valid
            //

            let decoded: u32 = ((u32::from(first_byte) & 0x07) << 18)
                | ((u32::from(second_byte) & 0x3f) << 12)
                | ((u32::from(third_byte) & 0x3f) << 6)
                | ((u32::from(fourth_byte) & 0x3f));
            let decoded = CodePoint::from_u32(decoded).unwrap();

            return self.valid_mb(decoded, policy);
        }
            //
            // Not a valid UTF-8 start, handle specially
            //
        0xff => {

            incr_diagnostic!(ByteDecoder_FFCount);

            if self.was_eof {

                //
                // Do not increment Column
                //

                return SourceCharacter::from(CodePoint::EndOfFile);
            }

            //
            // Incomplete
            //

            return self.incomplete_1byte_sequence(self.src_loc, policy);
        }
            //
            // Not a valid UTF-8 start
            //
        _ => {

            incr_diagnostic!(ByteDecoder_Incomplete1ByteCount);

            //
            // Incomplete
            //

            return self.incomplete_1byte_sequence(self.src_loc, policy);
        }
    }
}

fn strange_warning(
    &mut self,
    decoded: CodePoint,
    current_source_character_start_loc: Location,
    policy: NextPolicy,
) {
    let current_source_character_end_loc = self.src_loc;

    let safe_and_graphical_str = decoded.safe_and_graphical_string();
    let graphical_str = decoded.graphical_string();

    let src_span =
        Span::new(current_source_character_start_loc, current_source_character_end_loc);

    let mut actions: Vec<CodeAction> = Vec::new();

    let c = WLCharacter::new_with_escape(decoded, Escape::None);

    let certain_character_actions =
        utils::certain_character_replacement_actions(c, src_span);

    for A in certain_character_actions {
        actions.push(A);
    }

    //
    // graphical version
    //
    actions.push(CodeAction::replace_text(
        format!("Replace with ``{graphical_str}``"),
        src_span,
        graphical_str,
    ));

    //
    // any ASCII replacements
    //
    for r in crate::long_names::ascii_replacements(decoded) {
        actions.push(CodeAction::replace_text(
            format!(
                "Replace with ``{}``",
                crate::long_names::replacement_graphical(r.clone())
            ),
            src_span,
            r,
        ));
    }

    if policy.contains(STRING_OR_COMMENT) {
        //
        // reduce severity of unexpected characters inside strings or comments
        //

        let I = encoding_issue(
            IssueTag::UnexpectedCharacter,
            format!("Unexpected character: ``{safe_and_graphical_str}``."),
            Severity::Remark,
            src_span,
            0.95,
            actions,
            Vec::new(),
        );

        self.add_issue(I);
    } else if c.is_strange_whitespace() || c.is_mb_strange_whitespace() {

        // Do nothing.
    } else {
        let I = encoding_issue(
            IssueTag::UnexpectedCharacter,
            format!("Unexpected character: ``{safe_and_graphical_str}``."),
            Severity::Warning,
            src_span,
            0.95,
            actions,
            Vec::new(),
        );

        self.add_issue(I);
    }
}

fn non_ascii_warning(
    &mut self,
    decoded: CodePoint,
    current_source_character_start_loc: Location,
) {
    let current_source_character_end_loc = self.src_loc;

    let safe_and_graphical_str = decoded.safe_and_graphical_string();
    let graphical_str = decoded.graphical_string();

    let src_span =
        Span::new(current_source_character_start_loc, current_source_character_end_loc);

    let mut actions: Vec<CodeAction> = Vec::new();

    actions.push(CodeAction::replace_text(
        format!("Replace with ``{graphical_str}``"),
        src_span,
        graphical_str,
    ));

    for r in crate::long_names::ascii_replacements(decoded) {
        actions.push(CodeAction::replace_text(
            format!(
                "Replace with ``{}``",
                crate::long_names::replacement_graphical(r.clone())
            ),
            src_span,
            r,
        ));
    }

    let I = encoding_issue(
        IssueTag::NonASCIICharacter,
        format!("Non-ASCII character: ``{safe_and_graphical_str}``."),
        Severity::Remark,
        src_span,
        1.0,
        actions,
        Vec::new(),
    );

    self.add_issue(I);
}

fn valid_strange(
    &mut self,
    decoded: CodePoint,
    policy: NextPolicy,
) -> SourceCharacter {
    self.src().increment();

    if self.check_issues {
        let current_source_character_start_loc = self.src_loc.previous();

        self.strange_warning(
            decoded,
            current_source_character_start_loc,
            policy,
        );
    }

    return decoded;
}

fn valid_mb(
    &mut self,
    decoded: CodePoint,
    policy: NextPolicy,
) -> SourceCharacter {
    self.src().increment();

    if self.check_issues {
        let current_source_character_start_loc = self.src_loc.previous();

        if crate::utils::is_mb_strange(decoded) {
            self.strange_warning(
                decoded,
                current_source_character_start_loc,
                policy,
            );
        } else if self.encoding_mode == EncodingMode::Normal {
            self.non_ascii_warning(
                decoded,
                current_source_character_start_loc,
            );
        }
    }

    return decoded;
}

fn incomplete_1byte_sequence(
    &mut self,
    err_src_loc: Location,
    _policy: NextPolicy,
) -> SourceCharacter {
    self.incomplete_byte_sequence(
        err_src_loc,
        CodePoint::Unsafe1ByteUtf8Sequence,
    )
}

fn incomplete_2byte_sequence(
    &mut self,
    err_src_loc: Location,
    _policy: NextPolicy,
) -> SourceCharacter {
    self.incomplete_byte_sequence(
        err_src_loc,
        CodePoint::Unsafe2ByteUtf8Sequence,
    )
}

fn incomplete_3byte_sequence(
    &mut self,
    err_src_loc: Location,
    _policy: NextPolicy,
) -> SourceCharacter {
    self.incomplete_byte_sequence(
        err_src_loc,
        CodePoint::Unsafe3ByteUtf8Sequence,
    )
}

fn incomplete_byte_sequence(
    &mut self,
    err_src_loc: Location,
    err_char: SourceCharacter,
) -> SourceCharacter {
    self.src().increment();

    if self.check_issues {
        //
        // No CodeAction here
        //

        let I = encoding_issue(
            IssueTag::IncompleteUTF8Sequence,
            "Incomplete UTF-8 sequence.".into(),
            Severity::Fatal,
            Span::new(err_src_loc, err_src_loc.next()),
            1.0,
            vec![],
            vec![],
        );

        self.add_issue(I);
    }

    //
    // http://www.unicode.org/faq/utf_bom.html
    // Are there any byte sequences that are not generated by a UTF? How should I interpret them?
    //
    // Related bugs: 366106, 376155
    //

    self.set_unsafe_character_encoding_flag(
        UnsafeCharacterEncoding::IncompleteUTF8Sequence,
    );

    return err_char;
}

//
// Related bugs: 376155
//
fn stray_surrogate(
    &mut self,
    err_src_loc: Location,
    _policy: NextPolicy,
) -> SourceCharacter {
    self.src().increment();

    if self.check_issues {
        //
        // No CodeAction here
        //

        let I = encoding_issue(
            IssueTag::StraySurrogate,
            "Stray surrogate.".into(),
            Severity::Fatal,
            Span::new(err_src_loc, err_src_loc.next()),
            1.0,
            vec![],
            vec![],
        );

        self.add_issue(I);
    }

    //
    // http://www.unicode.org/faq/utf_bom.html
    // Are there any byte sequences that are not generated by a UTF? How should I interpret them?
    //
    // Related bugs: 366106, 376155
    //

    self.set_unsafe_character_encoding_flag(
        UnsafeCharacterEncoding::StraySurrogate,
    );

    return CodePoint::from(CodePoint::Unsafe3ByteUtf8Sequence);
}

fn bom(
    &mut self,
    err_src_loc: Location,
    _policy: NextPolicy,
) -> SourceCharacter {
    self.src().increment();

    if self.check_issues {
        //
        // No CodeAction here
        //

        let I = encoding_issue(
            IssueTag::BOM,
            "BOM.".into(),
            Severity::Fatal,
            Span::new(err_src_loc, err_src_loc.next()),
            1.0,
            vec![],
            vec![],
        );

        self.add_issue(I);
    }

    self.set_unsafe_character_encoding_flag(UnsafeCharacterEncoding::BOM);

    return SourceCharacter::from(CodePoint::Unsafe3ByteUtf8Sequence);
}
}

impl SourceConvention {
    pub fn new_source_location(&self) -> Location {
        match self {
            SourceConvention::LineColumn => Location::LineColumn(LineColumn(
                NonZeroU32::MIN,
                NonZeroU32::MIN,
            )),
            SourceConvention::CharacterIndex => Location::CharacterIndex(1),
        }
    }
}

/// How to manage advancing through [`Location`]s
impl<'t> SourceManager<'t> {
    fn newline(&mut self) {
        match self.loc {
            Location::LineColumn(LineColumn(line, column)) => {
                *line = non_zero_u32_incr(*line);
                *column = NonZeroU32::MIN;
            },
            Location::CharacterIndex(index) => {
                *index += 1;
            },
        }
    }

    fn windows_newline(&mut self) {
        match self.loc {
            Location::LineColumn(LineColumn(line, column)) => {
                *line = non_zero_u32_incr(*line);
                *column = NonZeroU32::MIN;
            },
            Location::CharacterIndex(index) => {
                *index += 2;
            },
        }
    }

    fn tab(&mut self) {
        match self.loc {
            Location::LineColumn(LineColumn(_, column)) => {
                let current_tab_stop =
                    self.tab_width * ((column.get() - 1) / self.tab_width) + 1;

                *column =
                    NonZeroU32::new(current_tab_stop + self.tab_width).unwrap();
            },
            Location::CharacterIndex(index) => {
                *index += 1;
            },
        }
    }

    fn increment(&mut self) {
        match self.loc {
            Location::LineColumn(LineColumn(_, column)) => {
                *column = non_zero_u32_incr(*column)
            },
            Location::CharacterIndex(index) => *index += 1,
        }
    }
}
