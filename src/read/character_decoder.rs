use crate::{
    generated::long_names::CODEPOINT_TO_LONGNAME_MAP,
    issue::{CodeAction, IssueTag, Severity, syntax_issue},
    long_names::{self as LongNames, self},
    read::{
        code_point::{CodePoint::*, *},
        wl_character::{Escape, WLCharacter},
        Reader,
    },
    source::{
        BufferAndLength, Location, NextPolicy,
        NextPolicyBits::{
            ENABLE_CHARACTER_DECODING_ISSUES, SCAN_FOR_UNRECOGNIZEDLONGNAMES,
        },
        SourceCharacter, Span, STRING_OR_COMMENT,
    },
    utils,
};

use super::InputMark;

//
// CharacterDecoder is given a stream of integers that represent Unicode code points and decodes
// sequences of Source Characters such as \[Alpha] into a single WL character
//


/// Precondition: buffer is pointing to current WLCharacter
/// Postcondition: buffer is pointing to next WLCharacter
///
/// Example:
/// memory: 1+\[Alpha]-2
///           ^
///           buffer
///
/// after calling nextWLCharacter:
/// memory: 1+\[Alpha]-2
///                   ^
///                   buffer
/// return \[Alpha]
///
impl Reader<'_> {
    pub(crate) fn next_wl_character(
    &mut self,
    policy: NextPolicy,
) -> WLCharacter {
    let mut point = self.next_source_char(policy);

    if point != '\\' {
        incr_diagnostic!(CharacterDecoder_UnescapedCount);

        return WLCharacter::new(point);
    }

    //
    // Handle \
    //
    // handle escapes like special characters
    //

    //
    // There was a \
    //

    let escaped = self.mark();

    point = self.peek_source_char(policy);

    if !point.is_ascii() {
        return self.handle_uncommon(escaped, policy);
    }

    match point.as_i32() as u8 {
        b'"'  => self.handle_string_meta_double_quote(escaped, policy),
        b'<'  => self.handle_string_meta_open(escaped, policy),
        b'>'  => self.handle_string_meta_close(escaped, policy),
        b'\\' => self.handle_string_meta_backslash(escaped, policy),
        _     => self.handle_uncommon(escaped, policy),
    }
}

fn handle_string_meta_double_quote(
    &mut self,
    _: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    incr_diagnostic!(CharacterDecoder_StringMetaDoubleQuoteCount);

    self.next_source_char(policy);

    return WLCharacter::new_with_escape(
        StringMeta_DoubleQuote,
        Escape::Single,
    );
}

//
// \\ \" \< \>
//
// String meta characters
// What are \< and \> ?
// https://mathematica.stackexchange.com/questions/105018/what-are-and-delimiters-in-box-expressions
// https://stackoverflow.com/q/6065887
//
fn handle_string_meta_open(
    &mut self,
    escaped: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    incr_diagnostic!(CharacterDecoder_StringMetaOpenCount);

    self.next_source_char(policy);

    let c = WLCharacter::new_with_escape(StringMeta_Open, Escape::Single);

    if self.check_issues {
        let graphical_str = c.graphical_string();

        let current_wl_character_start_loc = escaped.src_loc.previous();

        let current_wl_character_end_loc = self.src_loc;

        let src_span =
            Span::new(current_wl_character_start_loc, current_wl_character_end_loc);

        //
        // matched reduced severity of unexpected characters inside strings or comments
        //

        let issue = syntax_issue(
            IssueTag::UnexpectedCharacter,
            format!("Unexpected string meta character: ``{}``.", graphical_str),
            Severity::Remark,
            src_span,
            0.95,
            vec![],
            vec![format!(
                "The kernel parses ``\"{graphical_str}\"`` as an empty string."
            )],
        );

        self.add_issue(issue);
    }

    return c;
}

fn handle_string_meta_close(
    &mut self,
    escaped: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    incr_diagnostic!(CharacterDecoder_StringMetaCloseCount);

    self.next_source_char(policy);

    let c = WLCharacter::new_with_escape(StringMeta_Close, Escape::Single);

    if self.check_issues {
        let graphical_str = c.graphical_string();

        let current_wl_character_start_loc = escaped.src_loc.previous();

        let current_wl_character_end_loc = self.src_loc;

        let src_span =
            Span::new(current_wl_character_start_loc, current_wl_character_end_loc);

        //
        // matched reduced severity of unexpected characters inside strings or comments
        //

        let issue = syntax_issue(
            IssueTag::UnexpectedCharacter,
            format!("Unexpected string meta character: ``{graphical_str}``."),
            Severity::Remark,
            src_span,
            0.95,
            vec![],
            vec![format!(
                "The kernel parses ``\"{graphical_str}\"`` as an empty string."
            )],
        );

        self.add_issue(issue);
    }

    return c;
}

fn handle_string_meta_backslash(
    &mut self,
    _: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    incr_diagnostic!(CharacterDecoder_StringMetaBackslashCount);

    self.next_source_char(policy);

    return self.handle_backslash(policy);
}

fn handle_long_name(
    &mut self,
    open_square: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    // assert!(openSquareBuf[0] == b'[');
    assert!(self.input[open_square.offset] == b'[');

    incr_diagnostic!(CharacterDecoder_LongNameCount);

    //
    // Do not write leading \[ or trailing ] to LongName
    //
    let long_name_start_buf = self.buffer();

    let mut cur_source = self.peek_source_char(policy);

    let mut well_formed = false;

    let mut atleast_1_digit_or_alpha = false;

    //
    // Read at least 1 alnum before entering loop
    //
    // Must start with upper
    //
    if cur_source.is_upper() {
        atleast_1_digit_or_alpha = true;

        self.next_source_char(policy);

        cur_source = self.peek_source_char(policy);

        loop {
            if cur_source.is_alpha_or_digit() {
                self.next_source_char(policy);

                cur_source = self.peek_source_char(policy);
            } else if cur_source == ']' {
                well_formed = true;

                break;
            } else {
                //
                // Something like \[A!] which is not a long name
                //

                break;
            }
        }
    } else if cur_source == ']' {
        //
        // Handle \[]
        //

        self.next_source_char(policy);

        cur_source = self.peek_source_char(policy);
    }

    if !well_formed {
        //
        // Not well-formed
        //

        if self.check_issues
            && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
        {
            let current_wl_character_start_loc = open_square.src_loc.previous();

            let current_wl_character_end_buf = self.buffer();
            let current_wl_character_end_loc = self.src_loc;

            let long_name_end_buf = current_wl_character_end_buf;

            // let long_name_buf_and_len = BufferAndLength(long_name_start_buf, long_name_end_buf - long_name_start_buf);
            let long_name_buf_and_len =
                BufferAndLength::between(long_name_start_buf, long_name_end_buf);
            let long_name_str = long_name_buf_and_len.as_str();

            if atleast_1_digit_or_alpha {
                //
                // Something like \[Alpha
                //
                // Make the warning message a little more relevant
                //

                let suggestion =
                    long_name_suggestion(long_name_str);

                let mut actions: Vec<CodeAction> = Vec::new();

                let found =
                    long_names::longname_to_codepoint(long_name_str).is_some();

                if found {
                    actions.push(CodeAction::insert_text(
                        format!("Insert ``]`` to form ``\\[{suggestion}]``"),
                        Span::at(current_wl_character_end_loc),
                        "]".into(),
                    ));
                }

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\[{long_name_str}``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            } else {
                //
                // Malformed some other way
                //
                // Something like \[!
                // Something like \[*
                //

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\[{long_name_str}``"),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\[{long_name_str}"),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\[{}``.", long_name_str),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }
        }

        self.seek(open_square);

        return WLCharacter::new('\\');
    }

    //
    // Well-formed
    //

    //
    // if unlikelyEscapeChecking, then make sure to append all of the Source characters again
    //

    let long_name_end_buf = self.buffer();

    let long_name_buf_and_len =
        BufferAndLength::between(long_name_start_buf, long_name_end_buf);
    // let long_name_buf_and_len = BufferAndLength {
    //     buf: long_name_start_buf,
    //     len: long_name_end_buf.as_ptr().addr() - long_name_start_buf.as_ptr().addr(),
    // };

    // let long_name_str = std::string(reinterpret_cast::<*const i8>(long_name_buf_and_len.Buf), long_name_buf_and_len.length());
    let long_name_str = long_name_buf_and_len.as_str();

    let found: Option<CodePoint> =
        long_names::longname_to_codepoint(long_name_str);

    if found == None {
        //
        // Name not found
        //

        if self.check_issues
            && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
        {
            let long_name_end_loc = self.src_loc;

            let current_wl_character_start_loc = open_square.src_loc.previous();

            //
            // Accomodate the ] character
            //
            let current_wl_character_end_loc = long_name_end_loc.next();

            let suggestion = long_name_suggestion(long_name_str);

            let mut actions: Vec<CodeAction> = Vec::new();

            // if !suggestion.is_empty() {
            //     actions.push(CodeAction::replace_text(
            //         format!("Replace with ``\\[{suggestion}]``"),
            //         Span::new(current_wl_character_start_loc, current_wl_character_end_loc),
            //         format!("\\[{suggestion}]"),
            //     ));
            // }

            // //
            // // More specifically: Unrecognized
            // //
            // let issue = syntax_issue(
            //     STRING_UNHANDLEDCHARACTER,
            //     format!("Unhandled character: ``\\[{long_name_str}]``."),
            //     STRING_FATAL,
            //     Span::new(current_wl_character_start_loc, current_wl_character_end_loc),
            //     1.0,
            //     Actions,
            //     vec![format!("``{long_name_str}`` is not a recognized long name.")],
            // );

            // self.add_issue(issue);

            if policy.contains(SCAN_FOR_UNRECOGNIZEDLONGNAMES) {
                let current_unrecognized_start_loc =
                    current_wl_character_start_loc.previous();

                if !suggestion.is_empty() {
                    actions.push(CodeAction::replace_text(
                        format!("Replace with ``\\\\[{suggestion}]``"),
                        Span::new(
                            current_unrecognized_start_loc,
                            current_wl_character_end_loc,
                        ),
                        format!("\\\\[{suggestion}]"),
                    ));
                }

                let issue = syntax_issue(
                    IssueTag::UnrecognizedLongName,
                    format!("Unrecognized longname: ``\\\\[{long_name_str}]``."),
                    Severity::Error,
                    Span::new(
                        current_unrecognized_start_loc,
                        current_wl_character_end_loc,
                    ),
                    0.75,
                    actions,
                    vec![format!(
                        "``{long_name_str}`` is not a valid long name."
                    )],
                );

                self.add_issue(issue);
            } else {
                if !suggestion.is_empty() {
                    actions.push(CodeAction::replace_text(
                        format!("Replace with ``\\[{suggestion}]``"),
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        format!("\\[{suggestion}]"),
                    ));
                }

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\[{long_name_str}]``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![format!(
                        "``{long_name_str}`` is not a valid long name."
                    )],
                );

                self.add_issue(issue);
            }
        }

        self.seek(open_square);

        return WLCharacter::new('\\');
    }

    //
    // Success!
    //

    let point: CodePoint = found.unwrap();

    self.next_source_char(policy);

    if self.check_issues && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
    {
        // let long_name_buf_and_len = BufferAndLength(long_name_start_buf, long_name_end_buf - long_name_start_buf);
        let long_name_buf_and_len =
            BufferAndLength::between(long_name_start_buf, long_name_end_buf);
        let long_name_str = long_name_buf_and_len.as_str();

        self.check_strange_syntax_issue(
            policy,
            point,
            open_square.src_loc,
            if LongNames::is_raw(long_name_str) {
                Escape::Raw
            } else {
                Escape::LongName
            },
        );
    }

    if LongNames::is_raw(long_name_str) {
        return WLCharacter::new_with_escape(point, Escape::Raw);
    }

    return WLCharacter::new_with_escape(point, Escape::LongName);
}

fn handle_4hex(
    &mut self,
    colon: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    assert!(self.input[colon.offset] == b':');

    incr_diagnostic!(CharacterDecoder_4HexCount);

    let hex_start_buf = self.buffer();

    for _ in 0..4 {
        let cur_source = self.peek_source_char(policy);

        if cur_source.is_hex() {
            self.next_source_char(policy);
        } else {
            //
            // Not well-formed
            //
            // Something like \:z
            //

            if self.check_issues
                && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
            {
                let current_wl_character_start_loc = colon.src_loc.previous();

                let current_wl_character_end_buf = self.buffer();
                let current_wl_character_end_loc = self.src_loc;

                let hex_end_buf = current_wl_character_end_buf;

                // let hex_buf_and_len = BufferAndLength(hex_start_buf, hex_end_buf - hex_start_buf);
                let hex_buf_and_len =
                    BufferAndLength::between(hex_start_buf, hex_end_buf);
                let hex_str = hex_buf_and_len.as_str();

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\:{hex_str}``"),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\:{hex_str}"),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\:{hex_str}``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }

            self.seek(colon);

            return WLCharacter::new('\\');
        }
    }

    //
    // Success!
    //

    let d3 = u32::from(utils::to_digit(hex_start_buf[0]));
    let d2 = u32::from(utils::to_digit(hex_start_buf[1]));
    let d1 = u32::from(utils::to_digit(hex_start_buf[2]));
    let d0 = u32::from(utils::to_digit(hex_start_buf[3]));
    let point: u32 = d3 << 12 | d2 << 8 | d1 << 4 | d0;
    let mut point = CodePoint::from_u32(point).unwrap();

    match point {
        CodePoint::Char(CODEPOINT_ACTUAL_DOUBLEQUOTE) => {
            point = CodePoint::StringMeta_DoubleQuote;
        },
        CodePoint::Char(CODEPOINT_ACTUAL_BACKSLASH) => {
            point = CodePoint::StringMeta_Backslash;
        },
        _ => (),
    }

    self.check_strange_syntax_issue(
        policy,
        point,
        colon.src_loc,
        Escape::Hex4,
    );

    return WLCharacter::new_with_escape(point, Escape::Hex4);
}

fn handle_2hex(
    &mut self,
    dot: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    assert!(self.input[dot.offset] == b'.');

    incr_diagnostic!(CharacterDecoder_2HexCount);

    let hex_start_buf = self.buffer();

    for _ in 0..2 {
        let cur_source = self.peek_source_char(policy);

        if cur_source.is_hex() {
            self.next_source_char(policy);
        } else {
            //
            // Not well-formed
            //
            // Something like \.z
            //

            if self.check_issues
                && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
            {
                let current_wl_character_start_loc = dot.src_loc.previous();

                let current_wl_character_end_buf = self.buffer();
                let current_wl_character_end_loc = self.src_loc;

                let hex_end_buf = current_wl_character_end_buf;

                // let hex_buf_and_len = BufferAndLength(hex_start_buf, hex_end_buf - hex_start_buf);
                let hex_buf_and_len =
                    BufferAndLength::between(hex_start_buf, hex_end_buf);
                let hex_str = hex_buf_and_len.as_str();

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\.{hex_str}``"),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\.{hex_str}"),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\.{hex_str}``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }

            self.seek(dot);

            return WLCharacter::new('\\');
        }
    }

    //
    // Success!
    //

    let d1 = utils::to_digit(hex_start_buf[0]);
    let d0 = utils::to_digit(hex_start_buf[1]);
    let mut point: CodePoint = CodePoint::Char(char::from(d1 << 4 | d0));

    match point {
        CodePoint::Char(CODEPOINT_ACTUAL_DOUBLEQUOTE) => {
            point = CodePoint::StringMeta_DoubleQuote;
        },
        CodePoint::Char(CODEPOINT_ACTUAL_BACKSLASH) => {
            point = CodePoint::StringMeta_Backslash;
        },
        _ => (),
    }

    self.check_strange_syntax_issue(
        policy,
        point,
        dot.src_loc,
        Escape::Hex2,
    );

    return WLCharacter::new_with_escape(point, Escape::Hex2);
}

fn handle_octal(
    &mut self,
    first_octal: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    assert!(SourceCharacter::from(char::from(
        self.input[first_octal.offset]
    ))
    .is_octal());

    incr_diagnostic!(CharacterDecoder_OctalCount);

    let octal_start_buf = self.buffer_at(first_octal.offset);

    for _ in 0..3 - 1 {
        let cur_source = self.peek_source_char(policy);

        if cur_source.is_octal() {
            self.next_source_char(policy);
        } else {
            //
            // Not well-formed
            //
            // Something like \1z
            //

            if self.check_issues
                && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
            {
                let current_wl_character_start_loc = first_octal.src_loc.previous();

                let current_wl_character_end_buf = self.buffer();
                let current_wl_character_end_loc = self.src_loc;

                let octal_end_buf = current_wl_character_end_buf;

                // let octal_buf_and_len = BufferAndLength(octal_start_buf, octal_end_buf - octal_start_buf);
                let octal_buf_and_len =
                    BufferAndLength::between(octal_start_buf, octal_end_buf);
                let octal_str = octal_buf_and_len.as_str();

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\{octal_str}``"),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\{octal_str}"),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\{octal_str}``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }

            self.seek(first_octal);

            // FIXME: Why return a backslash if the character is not well-formed?
            return WLCharacter::new('\\');
        }
    }

    //
    // Success!
    //

    let d2 = utils::to_digit(octal_start_buf[0]);
    let d1 = utils::to_digit(octal_start_buf[1]);
    let d0 = utils::to_digit(octal_start_buf[2]);
    let point: u32 = u32::from(d2) << 6 | u32::from(d1) << 3 | u32::from(d0);
    let mut point = CodePoint::from_u32(point).unwrap();

    // FIXME: This match to do canonicalization is repeated in several places.
    //        Refactor this into a method on CodePoint. Perhaps this should even
    //        be done automatically in CodePoint::from_u32?
    match point {
        CodePoint::Char(CODEPOINT_ACTUAL_DOUBLEQUOTE) => {
            point = CodePoint::StringMeta_DoubleQuote;
        },
        CodePoint::Char(CODEPOINT_ACTUAL_BACKSLASH) => {
            point = CodePoint::StringMeta_Backslash;
        },
        _ => (),
    }

    self.check_strange_syntax_issue(
        policy,
        point,
        first_octal.src_loc,
        Escape::Octal,
    );

    return WLCharacter::new_with_escape(point, Escape::Octal);
}

fn handle_6hex(
    &mut self,
    bar: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    assert!(self.input[bar.offset] == b'|');

    incr_diagnostic!(CharacterDecoder_6HexCount);

    let hex_start_buf = self.buffer();

    for _ in 0..6 {
        let cur_source = self.peek_source_char(policy);

        if cur_source.is_hex() {
            self.next_source_char(policy);
        } else {
            //
            // Not well-formed
            //
            // Something like \|z
            //

            if self.check_issues
                && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
            {
                let current_wl_character_start_loc = bar.src_loc.previous();

                let current_wl_character_end_buf = self.buffer();
                let current_wl_character_end_loc = self.src_loc;

                let hex_end_buf = current_wl_character_end_buf;

                // let hex_buf_and_len = BufferAndLength(hex_start_buf, hex_end_buf - hex_start_buf);
                let hex_buf_and_len =
                    BufferAndLength::between(hex_start_buf, hex_end_buf);
                let hex_str = hex_buf_and_len.as_str();

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\|{hex_str}``"),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\|{hex_str}"),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!("Unhandled character: ``\\|{hex_str}``."),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }

            self.seek(bar);

            return WLCharacter::new('\\');
        }
    }

    //
    // Well-formed
    //

    let d5 = utils::to_digit(hex_start_buf[0]);
    let d4 = utils::to_digit(hex_start_buf[1]);
    let d3 = utils::to_digit(hex_start_buf[2]);
    let d2 = utils::to_digit(hex_start_buf[3]);
    let d1 = utils::to_digit(hex_start_buf[4]);
    let d0 = utils::to_digit(hex_start_buf[5]);
    // let point: u32 = d5 << 20 | d4 << 16 | d3 << 12 | d2 << 8 | d1 << 4 | d0;
    let point: u32 =
        u32::from_be_bytes([0, d5 << 4 | d4, d3 << 4 | d2, d1 << 4 | d0]);

    // TODO: Is this logic here correct? Why always return a \ if point is out
    //       of range?
    if point > 0x10ffff {
        self.seek(bar);

        return WLCharacter::new('\\');
    }

    let mut point = CodePoint::from_u32(point).unwrap();

    //
    // Success!
    //

    match point {
        CodePoint::Char(CODEPOINT_ACTUAL_DOUBLEQUOTE) => {
            point = CodePoint::StringMeta_DoubleQuote;
        },
        CodePoint::Char(CODEPOINT_ACTUAL_BACKSLASH) => {
            point = CodePoint::StringMeta_Backslash;
        },
        _ => (),
    }

    self.check_strange_syntax_issue(
        policy,
        point,
        bar.src_loc,
        Escape::Hex6,
    );

    return WLCharacter::new_with_escape(point, Escape::Hex6);
}

fn handle_backslash(
    &mut self,
    policy: NextPolicy,
) -> WLCharacter {
    //
    // test whether this \ is the result of the "feature" of
    // converting "\[Alpa]" into "\\[Alpa]", copying that, and then never giving any further warnings
    // when dealing with "\\[Alpa]"
    //
    if self.check_issues {
        let reset_mark = self.mark();

        //
        // will be resetting any way, so just use nextSourceCharacter here
        //
        let mut c = self.next_source_char(policy);

        if c == '[' {
            //
            // Try to reconstruct \[XXX]
            //
            // If well-formed, then warn
            //

            let long_name_start_buf = self.offset;
            let long_name_start_loc = self.src_loc;

            c = self.next_source_char(policy);

            let mut well_formed = false;

            if c.is_upper() {
                c = self.next_source_char(policy);

                loop {
                    if c.is_alpha_or_digit() {
                        c = self.next_source_char(policy);

                        continue;
                    }

                    if c == ']' {
                        well_formed = true;
                    }

                    break;
                }
            }

            if well_formed {
                let mut tmp_policy = policy;

                tmp_policy |= ENABLE_CHARACTER_DECODING_ISSUES;
                tmp_policy |= SCAN_FOR_UNRECOGNIZEDLONGNAMES;

                self.offset = long_name_start_buf;
                self.src_loc = long_name_start_loc;

                self.handle_long_name(reset_mark, tmp_policy);
            }
        }

        self.seek(reset_mark);
    }

    return WLCharacter::new_with_escape(StringMeta_Backslash, Escape::Single);
}

fn handle_unhandled_escape(
    &mut self,
    unhandled: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    //
    // Anything else
    //
    // Something like  \A
    //

    let escaped_char = self.peek_source_char(policy);

    self.next_source_char(policy);

    //
    // Make the warnings a little more relevant
    //

    if self.check_issues && policy.contains(ENABLE_CHARACTER_DECODING_ISSUES)
    {
        let current_wl_character_start_loc = unhandled.src_loc.previous();

        let mut current_wl_character_end_loc = self.src_loc;

        if escaped_char.is_upper() {
            //
            // Attempt to read \Alpha] and report a missing [
            //

            let mut alnumRun: String = String::new();

            alnumRun.push(escaped_char.as_char().unwrap());

            let mut cur_source = self.peek_source_char(policy);

            let mut well_formed = false;

            loop {
                if cur_source.is_alpha_or_digit() {
                    alnumRun.push(cur_source.as_char().unwrap());

                    self.next_source_char(policy);

                    cur_source = self.peek_source_char(policy);
                } else if cur_source == ']' {
                    self.next_source_char(policy);

                    well_formed = true;

                    break;
                } else {
                    //
                    // Unrecognized
                    //
                    // Something like \A!] which is not a long name
                    //

                    break;
                }
            }

            let mut well_formedAndFound = false;

            if well_formed {
                well_formedAndFound =
                    long_names::longname_to_codepoint(alnumRun.as_str())
                        .is_some();
            }

            if well_formedAndFound {
                //
                // Something like \Alpha]
                //

                current_wl_character_end_loc = self.src_loc;

                let cur_source_graphical_str = format!("{alnumRun}]");

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::insert_text(
                    format!("Insert ``[`` to form ``\\[{alnumRun}]``"),
                    Span::at(current_wl_character_start_loc.next()),
                    "[".into(),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!(
                        "Unhandled character ``\\{cur_source_graphical_str}``."
                    ),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            } else {
                if escaped_char.is_hex() {
                    let cur_source_graphical_str =
                        WLCharacter::new(escaped_char).graphical_string();

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::replace_text(
                        format!(
                            "Replace with ``\\[{}XXX]``",
                            cur_source_graphical_str
                        ),
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        format!("\\[{}XXX]", cur_source_graphical_str),
                    ));

                    actions.push(CodeAction::replace_text(
                        format!(
                            "Replace with ``\\:{}XXX``",
                            cur_source_graphical_str
                        ),
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        format!("\\:{}XXX", cur_source_graphical_str),
                    ));

                    let issue = syntax_issue(
                        IssueTag::UnhandledCharacter,
                        format!(
                            "Unhandled character ``\\{}``.",
                            cur_source_graphical_str
                        ),
                        Severity::Fatal,
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        1.0,
                        actions,
                        vec![],
                    );

                    self.add_issue(issue);
                } else {
                    let cur_source_graphical_str =
                        WLCharacter::new(escaped_char).graphical_string();

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::replace_text(
                        format!(
                            "Replace with ``\\[{}XXX]``",
                            cur_source_graphical_str
                        ),
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        format!("\\[{}XXX]", cur_source_graphical_str),
                    ));

                    let issue = syntax_issue(
                        IssueTag::UnhandledCharacter,
                        format!(
                            "Unhandled character ``\\{}``.",
                            cur_source_graphical_str
                        ),
                        Severity::Fatal,
                        Span::new(
                            current_wl_character_start_loc,
                            current_wl_character_end_loc,
                        ),
                        1.0,
                        actions,
                        vec![],
                    );

                    self.add_issue(issue);
                }
            }
        } else if escaped_char.is_hex() {
            let cur_source_graphical_str =
                WLCharacter::new(escaped_char).graphical_string();

            let mut actions: Vec<CodeAction> = Vec::new();

            actions.push(CodeAction::replace_text(
                format!("Replace with ``\\:{}xxx``", cur_source_graphical_str),
                Span::new(current_wl_character_start_loc, current_wl_character_end_loc),
                format!("\\:{}xxx", cur_source_graphical_str),
            ));

            let issue = syntax_issue(
                IssueTag::UnhandledCharacter,
                format!("Unhandled character ``\\{}``.", cur_source_graphical_str),
                Severity::Fatal,
                Span::new(current_wl_character_start_loc, current_wl_character_end_loc),
                1.0,
                actions,
                vec![],
            );

            self.add_issue(issue);
        } else if escaped_char.is_end_of_file() {

            //
            // Do not know what a good suggestion would be for \<EOF>
            //
        } else if escaped_char.is_mb_unsafe_utf8_sequence() {

            //
            // Do not know what a good suggestion would be for \<0xa9>
            //
        } else {
            //
            // Anything else
            //

            let cur_source_graphical_str =
                WLCharacter::new(escaped_char).graphical_string();

            if cur_source_graphical_str.len() > 1 {
                //
                // Something like \<tab>
                //
                // cur_source_graphical_str is now the 2 characters '\' 't'
                //
                // This can now be confusing when reporting the issue.
                // The correct number of backslashes is required.
                //
                // Do the simple thing: No actions, and report the character with all escaped backslashes now
                //

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!(
                        "Unhandled character ``\\\\{}``.",
                        cur_source_graphical_str
                    ),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    vec![],
                    vec![],
                );

                self.add_issue(issue);
            } else {
                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::replace_text(
                    format!("Replace with ``\\\\{}``", cur_source_graphical_str),
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    format!("\\\\{}", cur_source_graphical_str),
                ));

                let issue = syntax_issue(
                    IssueTag::UnhandledCharacter,
                    format!(
                        "Unhandled character ``\\{}``.",
                        cur_source_graphical_str
                    ),
                    Severity::Fatal,
                    Span::new(
                        current_wl_character_start_loc,
                        current_wl_character_end_loc,
                    ),
                    1.0,
                    actions,
                    vec![],
                );

                self.add_issue(issue);
            }
        }
    }

    //
    // Keep these treated as 2 characters. This is how bad escapes are handled in WL strings.
    // And has the nice benefit of the single \ still giving an error at top-level
    //
    // Currently, past the bad character
    //
    // Must remember to reset to the bad character
    //
    // The tokenizer will use the bad character to decide what to do
    //

    self.seek(unhandled);

    return WLCharacter::new('\\');
}

fn handle_uncommon(
    &mut self,
    escaped: InputMark,
    policy: NextPolicy,
) -> WLCharacter {
    let cur_source = self.peek_source_char(policy);

    match cur_source {
        Char('\n') => {
            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                LineContinuation_LineFeed,
                Escape::Single,
            );
        },
        Char('\r') => {
            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                LineContinuation_CarriageReturn,
                Escape::Single,
            );
        },
        CodePoint::CRLF => {
            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                LineContinuation_CRLF,
                Escape::Single,
            );
        },
        Char('[') => {
            self.next_source_char(policy);

            return self.handle_long_name(escaped, policy);
        },
        Char(':') => {
            self.next_source_char(policy);

            return self.handle_4hex(escaped, policy);
        },
        Char('.') => {
            self.next_source_char(policy);

            return self.handle_2hex(escaped, policy);
        },
        Char('|') => {
            self.next_source_char(policy);

            return self.handle_6hex(escaped, policy);
        },
        Char('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7') => {
            self.next_source_char(policy);

            return self.handle_octal(escaped, policy);
        },

        //
        // Simple escaped characters
        // \b \f \n \r \t
        //
        Char('b') => {
            incr_diagnostic!(CharacterDecoder_StringMetaBackspaceCount);

            self.next_source_char(policy);

            let c = WLCharacter::new_with_escape(
                StringMeta_Backspace,
                Escape::Single,
            );

            if self.check_issues {
                let graphical_str = c.graphical_string();

                let current_wl_character_start_loc = escaped.src_loc.previous();

                let current_wl_character_end_loc = self.src_loc;

                let src_span = Span::new(
                    current_wl_character_start_loc,
                    current_wl_character_end_loc,
                );

                //
                // matched reduced severity of unexpected characters inside strings or comments
                //

                let issue = syntax_issue(
                    IssueTag::UnexpectedCharacter,
                    format!("Unexpected character: ``{graphical_str}``."),
                    Severity::Remark,
                    src_span,
                    0.95,
                    vec![],
                    vec![],
                );

                self.add_issue(issue);
            }

            return c;
        },

        Char('f') => {
            //
            // \f is NOT a space character (but inside of strings, it does have special meaning)
            //

            incr_diagnostic!(CharacterDecoder_StringMetaFormFeedCount);

            self.next_source_char(policy);

            let c = WLCharacter::new_with_escape(
                StringMeta_FormFeed,
                Escape::Single,
            );

            if self.check_issues {
                let graphical_str = c.graphical_string();

                let current_wl_character_start_loc = escaped.src_loc.previous();

                let current_wl_character_end_loc = self.src_loc;

                let src_span = Span::new(
                    current_wl_character_start_loc,
                    current_wl_character_end_loc,
                );

                //
                // matched reduced severity of unexpected characters inside strings or comments
                //

                let issue = syntax_issue(
                    IssueTag::UnexpectedCharacter,
                    format!("Unexpected character: ``{graphical_str}``."),
                    Severity::Remark,
                    src_span,
                    0.95,
                    vec![],
                    vec![],
                );

                self.add_issue(issue);
            }

            return c;
        },

        Char('n') => {
            //
            // \n is NOT a newline character (but inside of strings, it does have special meaning)
            //

            incr_diagnostic!(CharacterDecoder_StringMetaLineFeedCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                StringMeta_LineFeed,
                Escape::Single,
            );
        },

        Char('r') => {
            //
            // \r is NOT a newline character (but inside of strings, it does have special meaning)
            //

            incr_diagnostic!(CharacterDecoder_StringMetaCarriageReturnCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                StringMeta_CarriageReturn,
                Escape::Single,
            );
        },

        Char('t') => {
            //
            // \t is NOT a space character (but inside of strings, it does have special meaning)
            //

            incr_diagnostic!(CharacterDecoder_StringMetaTabCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                StringMeta_Tab,
                Escape::Single,
            );
        },
        //
        // Linear syntax characters
        // \! \% \& \( \) \* \+ \/ \@ \^ \_ \` \<space>
        //
        Char('!') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxBangCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_BANG,
                Escape::Single,
            );
        },
        Char('%') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxPercentCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_PERCENT,
                Escape::Single,
            );
        },
        Char('&') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxAmpCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_AMP,
                Escape::Single,
            );
        },
        Char('(') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxOpenParenCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_OPENPAREN,
                Escape::Single,
            );
        },
        Char(')') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxCloseParenCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_CLOSEPAREN,
                Escape::Single,
            );
        },
        Char('*') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxStarCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_STAR,
                Escape::Single,
            );
        },
        Char('+') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxPlusCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_PLUS,
                Escape::Single,
            );
        },
        Char('/') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxSlashCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_SLASH,
                Escape::Single,
            );
        },
        Char('@') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxAtCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_AT,
                Escape::Single,
            );
        },
        Char('^') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxCaretCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_CARET,
                Escape::Single,
            );
        },
        Char('_') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxUnderscoreCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_UNDER,
                Escape::Single,
            );
        },
        Char('`') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxBacktickCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                CODEPOINT_LINEARSYNTAX_BACKTICK,
                Escape::Single,
            );
        },
        Char(' ') => {
            incr_diagnostic!(CharacterDecoder_LinearSyntaxSpaceCount);

            self.next_source_char(policy);

            return WLCharacter::new_with_escape(
                LinearSyntax_Space,
                Escape::Single,
            );
        },
        _ => (),
    } // switch

    //
    // Anything else
    //
    // Something like \A or \{
    //
    incr_diagnostic!(CharacterDecoder_UnhandledCount);

    return self.handle_unhandled_escape(escaped, policy);
}

//
// example:
// input: Alpa
// return Alpha
//
// Return empty string if no suggestion.
//
}

#[cfg(feature = "USE_EXPR_LIB")]
fn long_name_suggestion(input: String) -> String {
    let InputExpr = Expr_UTF8BytesToStringExpr(input.c_str(), input.len());

    let e = Expr_LongNameSuggestion(InputExpr);

    let buffer: Buffer;
    let len: usize;

    Expr_StringExprToUTF8Bytes(e, &buffer, reinterpret_cast::<*mut mint>(&len));

    let suggestion = std::string(reinterpret_cast::<*const i8>(buffer), len);

    Expr_Release(e);

    return suggestion;
}

fn long_name_suggestion(input: &str) -> String {
    use edit_distance::edit_distance;

    let closest: Option<&&str> = CODEPOINT_TO_LONGNAME_MAP
        .iter()
        .map(|(_, longname): &(CodePoint, &str)| longname)
        .min_by_key(|name| edit_distance(input, name));

    match closest {
        Some(closest) => {
            // If the `closest` long name isn't within an edit distance of 2,
            // then we aren't confident enough that it might have been the users
            // intent to write `closest`, so don't suggest it.
            if edit_distance(input, closest) <= 2 {
                closest.to_string()
            } else {
                String::new()
            }
        },
        // TODO: Return None?
        _ => String::new(),
    }
}

impl Reader<'_> {
/// Add an [`Issue`][crate::issue::Issue] if the specified [`CodePoint`] is
/// a "strange" character.
pub(crate) fn check_strange_syntax_issue(
    &mut self,
    policy: NextPolicy,
    point: CodePoint,
    start_loc: Location,
    escape_style: Escape,
) {
    if !self.check_issues {
        // Don't add any issues
        return;
    }

    let c = WLCharacter::new_with_escape(point, escape_style);

    let issue_value: f64 = if utils::is_strange(point) {
        if c.is_strange_whitespace() {
            return;
        } else {
            0.95
        }
    } else if utils::is_mb_strange(point) {
        if c.is_mb_strange_whitespace() {
            return;
        } else {
            0.85
        }
    } else {
        return;
    };

    //
    // Just generally strange character is in the code
    //

    let current_wl_character_start_loc = start_loc.previous();

    let current_source_character_end_loc = self.src_loc;

    let graphical_str = c.graphical_string();

    let src_span =
        Span::new(current_wl_character_start_loc, current_source_character_end_loc);

    let mut actions: Vec<CodeAction> = Vec::new();

    for a in utils::certain_character_replacement_actions(c, src_span) {
        actions.push(a);
    }

    //
    // do not recommend replacing graphical character with literal version
    //

    //
    // any ASCII replacements
    //
    for r in LongNames::ascii_replacements(point) {
        actions.push(CodeAction::replace_text(
            format!(
                "Replace with ``{}``",
                LongNames::replacement_graphical(r.clone())
            ),
            src_span,
            r,
        ));
    }

    //
    // reduce severity of unexpected characters inside strings or comments
    //
    let severity = if policy.contains(STRING_OR_COMMENT) {
        Severity::Remark
    } else {
        Severity::Warning
    };

    let issue = syntax_issue(
        IssueTag::UnexpectedCharacter,
        format!("Unexpected character: ``{graphical_str}``."),
        severity,
        src_span,
        issue_value,
        actions,
        vec![],
    );

    self.add_issue(issue);
}
}
