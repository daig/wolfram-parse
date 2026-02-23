//! Tokenizer takes a stream of WL characters and tokenizes them

use std::{collections::HashSet, os::raw::c_int};

use smallvec::SmallVec;

use crate::{
    feature,
    issue::{CodeAction, format_issue, IssueTag, Severity, syntax_issue},
    safe_expect,
    read::{
        code_point::{
            CodePoint::{Char, *},
            *,
        },
        Escape, InputMark, Reader, WLCharacter,
    },
    source::{
        Buffer, BufferAndLength, Location, NextPolicy, SourceCharacter, Span,
        INSIDE_SLOT, INSIDE_STRINGIFY_AS_FILE, INSIDE_STRINGIFY_AS_TAG,
        TOPLEVEL,
    },
    tokenize::{token_kind::Closer, Token, TokenKind, TokenRef},
    utils::{self, from_fn},
    FirstLineBehavior, ParseOptions,
};

#[cfg(feature = "string-interning")]
use crate::{
    string_interner::LocalInterner,
    tokenize::token::InterningTokenInput,
};

use crate::source::NextPolicyBits::*;

#[derive(Debug)]
pub(crate) struct Tokenizer<'i> {
    reader: Reader<'i>,

    first_line_behavior: FirstLineBehavior,

    pub(crate) group_stack: SmallVec<[Closer; 4]>,

    pub(crate) tracked: TrackedSourceLocations,

    #[cfg(feature = "string-interning")]
    interner: LocalInterner,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct TrackedSourceLocations {
    pub simple_line_continuations: HashSet<Location>,
    pub complex_line_continuations: HashSet<Location>,
    pub embedded_newlines: HashSet<Location>,
    pub embedded_tabs: HashSet<Location>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnsafeCharacterEncoding {
    // Ok = 0,
    IncompleteUTF8Sequence = 1,
    StraySurrogate = 2,
    ///
    BOM = 3,
}

/// Marker that records where a token started.
///
/// An instance of this type is created when the first character in a new
/// token is read by [`tokenizer_next_token()`]. When the last character in a
/// token is read, a call to
/// [`Tokenizer::token(kind, token_start)`][Tokenizer::token] is made to
/// construct a new [`Token`] whose source buffer and [`Span`] start at the specified
/// `TokenStart` instance and end at the current character in the input.
///
/// See also: [`InputMark`], which is a similar type that is used
/// for resetting the input cursor to an earlier state.
#[derive(Debug, Copy, Clone)]
struct TokenStart<'i> {
    buf: Buffer<'i>,
    loc: Location,
}

const _: () = assert!(std::mem::size_of::<TokenStart>() == 24);
const _: () = assert!(std::mem::size_of::<&TokenStart>() == 8);

impl UnsafeCharacterEncoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnsafeCharacterEncoding::IncompleteUTF8Sequence => {
                "IncompleteUTF8Sequence"
            },
            UnsafeCharacterEncoding::StraySurrogate => "StraySurrogate",
            UnsafeCharacterEncoding::BOM => "BOM",
            // NOTE: When adding a case here, also update from_str().
        }
    }

    #[doc(hidden)]
    pub fn from_str(string: &str) -> Option<Self> {
        let value = match string {
            "IncompleteUTF8Sequence" => {
                UnsafeCharacterEncoding::IncompleteUTF8Sequence
            },
            "StraySurrogate" => UnsafeCharacterEncoding::StraySurrogate,
            "BOM" => UnsafeCharacterEncoding::BOM,
            _ => return None,
        };

        Some(value)
    }

    #[doc(hidden)]
    pub fn from_missing_reason(missing_reason: &str) -> Option<Self> {
        if !missing_reason.starts_with("UnsafeCharacterEncoding_") {
            return None;
        }

        let variant_name: &str =
            missing_reason.trim_start_matches("UnsafeCharacterEncoding_");

        UnsafeCharacterEncoding::from_str(variant_name)
    }

    #[doc(hidden)]
    pub fn missing_reason(&self) -> String {
        let variant_name: &'static str = self.as_str();

        format!("UnsafeCharacterEncoding_{variant_name}")
    }
}

impl<'i> std::ops::Deref for Tokenizer<'i> {
    type Target = Reader<'i>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<'i> std::ops::DerefMut for Tokenizer<'i> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl<'i> Tokenizer<'i> {
    pub(crate) fn new(input: &'i [u8], opts: &ParseOptions) -> Self {
        let ParseOptions {
            first_line_behavior,
            src_convention: _,
            encoding_mode: _,
            tab_width: _,
            check_issues: _,
            compute_oob: _,
            quirk_settings: _,
        } = *opts;

        let mut tokenizer = Tokenizer {
            reader: Reader::new(input, opts),

            first_line_behavior,

            group_stack: SmallVec::new(),

            tracked: TrackedSourceLocations {
                simple_line_continuations: HashSet::new(),
                complex_line_continuations: HashSet::new(),
                embedded_newlines: HashSet::new(),
                embedded_tabs: HashSet::new(),
            },

            #[cfg(feature = "string-interning")]
            interner: LocalInterner::new(),
        };

        tokenizer.handle_first_line();

        tokenizer
    }

    fn handle_first_line(&mut self) {
        match self.first_line_behavior {
            FirstLineBehavior::NotScript => {
                return;
            },
            FirstLineBehavior::Check => {
                //
                // Handle the optional #! shebang
                //

                let mut peek = self.peek_token();

                if peek.tok != TokenKind::Hash {
                    // not #!

                    return;
                }

                peek.skip(self);

                peek = self.peek_token();

                if peek.tok != TokenKind::Bang {
                    // not #!

                    return;
                }

                //
                // Definitely a shebang
                //

                peek.skip(self);

                loop {
                    if feature::CHECK_ABORT && crate::should_abort() {
                        break;
                    }

                    let peek = self.peek_token();

                    if peek.tok == TokenKind::EndOfFile {
                        break;
                    }

                    if peek.tok == TokenKind::ToplevelNewline {
                        peek.skip(self);

                        break;
                    }

                    peek.skip(self);
                } // while (true)

                //
                // TODO: if anyone ever asks, then consider providing the shebang as a token
                // but only after BIGCODEMERGE!!
                //
            },
            FirstLineBehavior::Script => {
                //
                // Handle the #! shebang
                //

                let mut peek = self.peek_token();

                if peek.tok != TokenKind::Hash {
                    //
                    // TODO: add to Issues
                    //

                    return;
                }

                peek.skip(self);

                peek = self.peek_token();

                if peek.tok != TokenKind::Bang {
                    //
                    // TODO: add to Issues
                    //

                    return;
                }

                peek.skip(self);

                loop {
                    if feature::CHECK_ABORT && crate::should_abort() {
                        break;
                    }

                    let peek = self.peek_token();

                    if peek.tok == TokenKind::EndOfFile {
                        break;
                    }

                    if peek.tok == TokenKind::ToplevelNewline {
                        peek.skip(self);

                        break;
                    }

                    peek.skip(self);
                } // while (true)
            },
        }
    }

    //==================================
    // Read tokens
    //==================================

    /// Returns the next token in the input without advancing.
    ///
    /// Consecutive calls to `peek_token()` will always return the same token.
    ///
    /// Use [`Tokenizer::next_token()`] to return the current token and advance
    /// to the subsequent token.
    #[must_use]
    pub(crate) fn peek_token(&mut self) -> TokenRef<'i> {
        self.peek_token_with(TOPLEVEL)
    }

    /// Returns the next token in the input without advancing, using the specified
    /// policy settings.
    #[must_use]
    pub(crate) fn peek_token_with(
        &mut self,
        mut policy: NextPolicy,
    ) -> TokenRef<'i> {
        let inside_group: bool = !self.group_stack.is_empty();

        //
        // if inside_group:
        //   return_internal_newline_mask is 0b100
        // else:
        //   return_internal_newline_mask is 0b000
        //
        let return_internal_newline_mask = (inside_group as u8) << 2;

        policy.remove(NextPolicy::from_bits(return_internal_newline_mask));

        let mark = self.mark();

        let tok = tokenizer_next_token(self, policy);

        // Reset so it is as if we didn't advance.
        self.seek(mark);

        tok
    }

    /// Returns the next token in the input and advances.
    ///
    /// Precondition: buffer is pointing to current token
    ///
    /// Postcondition: buffer is pointing to next token
    ///
    /// Example:
    ///
    /// ```text
    /// memory: 1+\[Alpha]-2
    ///           ^
    ///           buffer
    /// ```
    ///
    /// after calling `next_token()`:
    ///
    /// ```text
    /// memory: 1+\[Alpha]-2
    ///                   ^
    ///                   buffer
    /// ```
    ///
    /// and `\[Alpha]` is returned.
    #[must_use]
    pub(crate) fn next_token(&mut self) -> TokenRef<'i> {
        tokenizer_next_token(self, crate::source::TOPLEVEL)
    }

    //==================================
    // Create tokens
    //==================================

    /// Construct a new token whose source buffer and location begin at `start`
    /// and end at the current buffer and source location.
    fn token<T: Into<TokenKind>>(
        &self,
        tok: T,
        start: &TokenStart<'i>,
    ) -> TokenRef<'i> {
        let tok = tok.into();

        let buf = self.get_token_buffer_and_length(start.buf);

        let span = self.get_token_span(start.loc);

        Token::new2(tok, buf, span)
    }

    /// Construct a new token whose source buffer and location are exactly the
    /// character located at `at`.
    fn error_token_at<T: Into<TokenKind>>(
        &self,
        tok: T,
        at: &TokenStart<'i>,
    ) -> TokenRef<'i> {
        let tok = tok.into();

        debug_assert!(tok.is_error());

        let buf = BufferAndLength::between(at.buf, at.buf);

        debug_assert_eq!(buf.buf.len(), 0);

        let span = Span::at(at.loc);

        Token::new2(tok, buf, span)
    }

    fn get_token_span(&self, tok_start_loc: Location) -> Span {
        debug_assert!(tok_start_loc <= self.src_loc);

        return Span::new(tok_start_loc, self.src_loc);
    }

    fn get_token_buffer_and_length(
        &self,
        tok_start_buf: Buffer<'i>,
    ) -> BufferAndLength<'i> {
        // return BufferAndLength::new(tokStartBuf, session.buffer - tokStartBuf);

        BufferAndLength::between(tok_start_buf, self.buffer())
    }

    /// Create a token with string interning support
    #[cfg(feature = "string-interning")]
    fn token_with_interning<T: Into<TokenKind>>(
        &mut self,
        tok: T,
        start: &TokenStart<'i>,
    ) -> Token<InterningTokenInput<'i>, Span> {
        let tok = tok.into();
        
        let buf = self.get_token_buffer_and_length(start.buf);
        let span = self.get_token_span(start.loc);
        
        // Convert buffer to string for interning check
        let input_str = std::str::from_utf8(buf.as_bytes())
            .unwrap_or("")
            .to_string();

        let input = if Token::should_intern(tok, &input_str) {
            InterningTokenInput::interned(self.interner.intern(&input_str))
        } else {
            InterningTokenInput::owned(input_str)
        };
        
        Token {
            tok,
            input,
            src: span,
        }
    }

    //==================================
    // Tracked locations
    //==================================

    fn add_simple_line_continuation(&mut self, loc: Location) {
        self.tracked.simple_line_continuations.insert(loc);
    }

    fn add_complex_line_continuation(&mut self, loc: Location) {
        self.tracked.complex_line_continuations.insert(loc);
    }

    fn add_embedded_newline(&mut self, loc: Location) {
        self.tracked.embedded_newlines.insert(loc);
    }

    fn add_embedded_tab(&mut self, loc: Location) {
        self.tracked.embedded_tabs.insert(loc);
    }
}

impl TrackedSourceLocations {
    #[allow(dead_code)]
    #[doc(hidden)]
    pub fn to_nodes(&self) -> [HashSet<Location>; 4] {
        let TrackedSourceLocations {
            simple_line_continuations,
            complex_line_continuations,
            embedded_newlines,
            embedded_tabs,
        } = self;

        [
            simple_line_continuations.clone(),
            complex_line_continuations.clone(),
            embedded_newlines.clone(),
            embedded_tabs.clone(),
        ]
    }
}

struct NumberTokenizationContext {
    InvalidBase: bool,

    UnrecognizedDigit: bool,

    NegativeExponent: bool,

    Real: bool,

    NonZeroExponentDigitCount: u32,
    //
    // Use the convention that base of 0 means the default, unspecified base
    //
    Base: c_int,
}

// TODO: Why does putting this type in the root module cause Rust Analyzer
//             to not work on this file?
// pub mod handler {
type HandlerFunction = for<'p, 'i> fn(
    session: &'p mut Tokenizer<'i>,
    // NOTE: Passing `token_start` by reference instead of by value is an
    //       optimization. At time of writing TokenStart is 32 bytes in size,
    //       but a reference is only 8.
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i>;
// }

/// Lookup table for handling ASCII characters that start a token.
///
/// This lookup table is equivalent to the match statement used in this `const`
/// definition to populate the table. However, using a lookup table instead of
/// a match is an optimization—doing an offset jump is more efficient than a
/// series of comparisions to find the right match arm.
///
/// See also `CHARACTER_DECODER_HANDLER_TABLE` in character_decoder.rs.
#[rustfmt::skip]
const TOKENIZER_HANDLER_TABLE: [HandlerFunction; 128] = from_fn!(
    [HandlerFunction, 128],
    |index: usize| {
        let index = index as u8;

        match index {
            0..=9 => tokenizer_next_token_uncommon,
            10 => tokenizer_handle_line_feed,
            11..=31 => tokenizer_next_token_uncommon,
            32 => tokenizer_handle_space,
            33 => tokenizer_next_token_uncommon,      // !
            34 => tokenizer_handle_string,            // "
            35 => tokenizer_next_token_uncommon,      // #
            36 => tokenizer_handle_symbol,            // $
            37..=43 => tokenizer_next_token_uncommon,
            44 => tokenizer_handle_comma,             // ,
            45 => tokenizer_handle_minus,             // -
            46..=47 => tokenizer_next_token_uncommon,
            48..=57 => tokenizer_handle_number,       // 0-9
            58..=64 => tokenizer_next_token_uncommon,
            65..=90 => tokenizer_handle_symbol,       // A-Z
            91 => tokenizer_handle_open_square,        // [
            92 => tokenizer_next_token_uncommon,      // \
            93 => tokenizer_handle_close_square,       // ]
            94..=96 => tokenizer_next_token_uncommon,
            97..=122 => tokenizer_handle_symbol,      // a-z
            123 => tokenizer_handle_open_curly,        // {
            124 => tokenizer_next_token_uncommon,     // |
            125 => tokenizer_handle_close_curly,       // }
            126..=127 => tokenizer_next_token_uncommon,
            // Not a valid ASCII character
            128..=255 => panic!(),
        }
    }
);

pub(crate) const ASCII_VTAB: char = '\x0B';
pub(crate) const ASCII_FORM_FEED: char = '\x0C';

fn tokenizer_next_token<'i>(
    session: &mut Tokenizer<'i>,
    policy: NextPolicy,
) -> TokenRef<'i> {
    let token_start = &TokenStart {
        buf: session.buffer(),
        loc: session.src_loc,
    };

    let c = tokenizer_next_wl_character(session, token_start, policy);

    let point: CodePoint = c.to_point();

    if !point.is_ascii() {
        return tokenizer_next_token_uncommon(session, token_start, c, policy);
    }

    let index = usize::try_from(point.as_i32()).unwrap();

    let func = TOKENIZER_HANDLER_TABLE[index];
    return func(session, token_start, c, policy);
}

fn tokenizer_next_token_uncommon<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    match c.to_point() {
        Char(_) => (),
        EndOfFile => {
            return session.token(TokenKind::EndOfFile, token_start);
        },
        Unsafe1ByteUtf8Sequence
        | Unsafe2ByteUtf8Sequence
        | Unsafe3ByteUtf8Sequence => {
            //
            // This will be disposed before the user sees it
            //

            return session
                .token(TokenKind::Error_UnsafeCharacterEncoding, token_start);
        },
        _ => (),
    }

    match c.to_point() {
        //
        // all single-byte characters
        //
        // most control characters are letterlike
        // jessef: There may be such a thing as *too* binary-safe...
        //
        Char('`' |
        'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h' | 'i' | 'j' | 'k' | 'l' | 'm' |
        'n' | 'o' | 'p' | 'q' | 'r' | 's' | 't' | 'u' | 'v' | 'w' | 'x' | 'y' | 'z' |
        '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | /*    \x07*/
        '\x08' | /*    \x09*/ /*    \x0a*/ /*    \x0b*/ /*    \x0c*/ /*    \x0d*/ '\x0e' | '\x0f' |
        '\x10' | '\x11' | '\x12' | '\x13' | '\x14' | '\x15' | '\x16' | '\x17' |
        '\x18' | '\x19' | '\x1a' | '\x1b' | '\x1c' | '\x1d' | '\x1e' | '\x1f') => {

            return tokenizer_handle_symbol(session, token_start, c, policy);
        }
        Char(CODEPOINT_BEL | CODEPOINT_DEL) => {
            return session.token(TokenKind::Error_UnhandledCharacter, token_start);
        }
        Char('\t') => {
            return session.token(TokenKind::Whitespace, token_start);
        }
        Char(ASCII_VTAB | ASCII_FORM_FEED) => {

            return tokenizer_handle_strange_whitespace(session, token_start, c, policy);
        }
        Char('\r') => {

            incr_diagnostic!(Tokenizer_NewlineCount);

            //
            // Return INTERNALNEWLINE or TOPLEVELNEWLINE, depending on policy
            //
            return session.token(
                TokenKind::newline_with_policy(policy), token_start);
        }
        Char('(') => {

            return tokenizer_handle_open_paren(session, token_start, c, policy);
        }
        Char(')') => {

            incr_diagnostic!(Tokenizer_CloseParenCount);

            return session.token(TokenKind::CloseParen, token_start);
        }
        Char('+') => {

            return tokenizer_handle_plus(session, token_start, c, policy);
        }
        Char('^') => {

            return tokenizer_handle_caret(session, token_start, c, policy);
        }
        Char('=') => {

            return tokenizer_handle_equal(session, token_start, c, policy);
        }
        Char(';') => {

            return tokenizer_handle_semi(session, token_start, c, policy);
        }
        Char(':') => {

            return tokenizer_handle_colon(session, token_start, c, policy);
        }
        Char('#') => {

            return tokenizer_handle_hash(session, token_start, c, policy);
        }
        Char('&') => {

            return tokenizer_handle_amp(session, token_start, c, policy);
        }
        Char('!') => {

            return tokenizer_handle_bang(session, token_start, c, policy);
        }
        Char('%') => {

            return tokenizer_handle_percent(session, token_start, c, policy);
        }
        Char('\'') => {

            return session.token(TokenKind::SingleQuote, token_start);
        }
        Char('*') => {

            return tokenizer_handle_star(session, token_start, c, policy);
        }
        Char('.') => {

            return tokenizer_handle_dot(session, token_start, c, policy);
        }
        Char('/') => {

            return tokenizer_handle_slash(session, token_start, c, policy);
        }
        Char('<') => {

            return tokenizer_handle_less(session, token_start, c, policy);
        }
        Char('>') => {

            return tokenizer_handle_greater(session, token_start, c, policy);
        }
        Char('?') => {

            return tokenizer_handle_question(session, token_start, c, policy);
        }
        Char('@') => {

            return tokenizer_handle_at(session, token_start, c, policy);
        }
        Char('\\') => {

            return tokenizer_handle_unhandled_backslash(session, token_start, c, policy);
        }
        Char('_') => {

            return tokenizer_handle_under(session, token_start, c, policy);
        }
        Char('|') => {

            return tokenizer_handle_bar(session, token_start, c, policy);
        }
        Char('~') => {
            return tokenizer_handle_tilde(session, token_start, c, policy);
        }
        Char(CODEPOINT_LINEARSYNTAX_BANG) => {
            return session.token(TokenKind::LinearSyntax_Bang, token_start);
        }
        Char(CODEPOINT_LINEARSYNTAX_OPENPAREN) => {
            return tokenizer_handle_mb_linear_syntax_blob(session, token_start, c, policy);
        }
        _ => ()
    }


    if c.is_mb_linear_syntax() {
        return tokenizer_handle_naked_mb_linear_syntax(
            session,
            token_start,
            c,
            policy,
        );
    }

    if c.is_mb_uninterpretable() {
        return session.token(TokenKind::Error_UnhandledCharacter, token_start);
    }

    if c.is_mb_strange_whitespace() {
        return tokenizer_handle_mb_strange_whitespace(
            session,
            token_start,
            c,
            policy,
        );
    }

    if c.is_mb_whitespace() {
        return session.token(TokenKind::Whitespace, token_start);
    }

    if c.is_mb_strange_newline() {
        return tokenizer_handle_mb_strange_newline(
            session,
            token_start,
            c,
            policy,
        );
    }

    if c.is_mb_newline() {
        //
        // Return INTERNALNEWLINE or TOPLEVELNEWLINE, depending on policy
        //
        return session
            .token(TokenKind::newline_with_policy(policy), token_start);
    }

    if c.is_mb_punctuation() {
        return tokenizer_handle_mb_punctuation(session, token_start, c, policy);
    }

    if c.is_mb_string_meta() {
        return session.token(TokenKind::Error_UnhandledCharacter, token_start);
    }

    //
    // if nothing else, then it is letterlike
    //

    assert!(c.is_mb_letterlike());

    return tokenizer_handle_symbol(session, token_start, c, policy);
}

pub(crate) fn tokenizer_next_token_stringify_as_tag<'i>(
    session: &mut Tokenizer<'i>,
) -> TokenRef<'i> {
    let token_start = &TokenStart {
        buf: session.buffer(),
        loc: session.src_loc,
    };

    let policy = INSIDE_STRINGIFY_AS_TAG;

    let c = tokenizer_next_wl_character(session, token_start, policy);

    match c.to_point() {
        EndOfFile => {
            //
            // EndOfFile is special, so invent source
            //

            return session
                .error_token_at(TokenKind::Error_ExpectedTag, token_start);
        },
        Char('\n' | '\r') | CRLF => {
            //
            // Newline is special, so invent source
            //

            return session
                .error_token_at(TokenKind::Error_ExpectedTag, token_start);
        },
        Char('"') => {
            return tokenizer_handle_string(session, token_start, c, policy);
        },
        //
        // Default
        //
        _ => {
            return tokenizer_handle_string_stringify_as_tag(
                session,
                token_start,
                c,
                policy,
            );
        },
    }
}

//
// Use SourceCharacters here, not WLCharacters
//
pub(crate) fn tokenizer_next_token_stringify_as_file<'i>(
    session: &mut Tokenizer<'i>,
) -> TokenRef<'i> {
    let token_start = &TokenStart {
        buf: session.buffer(),
        loc: session.src_loc,
    };

    let policy = INSIDE_STRINGIFY_AS_FILE;

    let c = session.next_source_char(policy);

    match c {
        EndOfFile => {
            return session
                .error_token_at(TokenKind::Error_ExpectedFile, token_start);
        },
        Char('\n' | '\r') | CRLF => {
            //
            // Stringifying as a file can span lines
            //
            // Something like  a >>
            //                    b
            //
            // should work
            //
            // Do not use TokenKind::Error_EMPTYSTRING here
            //

            //
            // Return INTERNALNEWLINE or TOPLEVELNEWLINE, depending on policy
            //
            return session
                .token(TokenKind::newline_with_policy(policy), token_start);
        },
        Char(' ' | '\t') => {
            //
            // There could be space, something like  << abc
            //
            // or something like:
            // a >>
            //   b
            //
            return session.token(TokenKind::Whitespace, token_start);
        },
        Char('"') => {
            return tokenizer_handle_string(
                session,
                token_start,
                WLCharacter::new(c),
                policy,
            );
        },
        //
        // Default case
        //
        _ => {
            return tokenizer_handle_string_stringify_as_file(
                session,
                token_start,
                c,
                policy,
            );
        },
    }
}

pub(crate) fn tokenizer_current_token_stringify_as_tag<'i>(
    session: &mut Tokenizer<'i>,
) -> TokenRef<'i> {
    let mark = session.mark();

    let tok = tokenizer_next_token_stringify_as_tag(session);

    session.seek(mark);

    return tok;
}

pub(crate) fn tokenizer_current_token_stringify_as_file<'i>(
    session: &mut Tokenizer<'i>,
) -> TokenRef<'i> {
    let mark = session.mark();

    let tok = tokenizer_next_token_stringify_as_file(session);

    session.seek(mark);

    return tok;
}

//
// Handling line continuations belongs in some layer strictly above CharacterDecoder and below Tokenizer.
//
// Some middle layer that deals with "parts" of a token.
//
fn tokenizer_next_wl_character<'i>(
    session: &mut Tokenizer<'i>,
    // TODO(cleanup): Just take a Location, token_start.buf is never used
    token_start: &TokenStart<'i>,
    policy: NextPolicy,
) -> WLCharacter {
    incr_diagnostic!(Tokenizer_LineContinuationCount);

    let mut c = session.next_wolfram_char(policy);

    loop {
        if !c.point.is_line_continuation() {
            return c;
        }

        c = session.peek_wolfram_char(policy);

        //
        // Even though strings preserve the whitespace after a line continuation, and
        // e.g., integers do NOT preserve the whitespace after a line continuation,
        // we do not need to worry about that here.
        //
        // There are no choices to be made here.
        // All whitespace after a line continuation can be ignored for the purposes of tokenization
        //
        while c.is_whitespace() {
            if session.compute_oob {
                if c.point == '\t' {
                    if policy.contains(STRING_OR_COMMENT) {
                        //
                        // It is possible to have e.g.:
                        //
                        //"a\
                        //<tab>b"
                        //
                        // where the embedded tab gets consumed by the whitespace loop after the line continuation.
                        //
                        // Must still count the embedded tab

                        session.add_embedded_tab(token_start.loc);
                    }
                }
            }

            session.next_wolfram_char(policy);

            c = session.peek_wolfram_char(policy);
        }

        if session.compute_oob {
            if policy.contains(TRACK_LC) {
                if policy.contains(STRING_OR_COMMENT) {
                    session.add_complex_line_continuation(token_start.loc);
                } else {
                    session.add_simple_line_continuation(token_start.loc);
                }
            }
        }

        session.next_wolfram_char(policy);
    } // loop
}

fn tokenizer_current_wl_character<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut policy: NextPolicy,
) -> WLCharacter {
    let mark = session.mark();

    //
    //
    //
    policy.remove(TRACK_LC);

    let c = tokenizer_next_wl_character(session, token_start, policy);

    session.seek(mark);

    return c;
}

fn tokenizer_handle_comma<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_CommaCount);

    return session.token(TokenKind::Comma, token_start);
}

fn tokenizer_handle_line_feed<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_NewlineCount);

    //
    // Return INTERNALNEWLINE or TOPLEVELNEWLINE, depending on policy
    //
    return session.token(TokenKind::newline_with_policy(policy), token_start);
}

fn tokenizer_handle_open_square<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_OpenSquareCount);

    return session.token(TokenKind::OpenSquare, token_start);
}

fn tokenizer_handle_open_curly<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_OpenCurlyCount);

    return session.token(TokenKind::OpenCurly, token_start);
}

fn tokenizer_handle_space<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_WhitespaceCount);

    return session.token(TokenKind::Whitespace, token_start);
}

fn tokenizer_handle_close_square<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_CloseSquareCount);

    return session.token(TokenKind::CloseSquare, token_start);
}

fn tokenizer_handle_close_curly<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _firstChar: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    incr_diagnostic!(Tokenizer_CloseCurlyCount);

    return session.token(TokenKind::CloseCurly, token_start);
}

fn tokenizer_handle_strange_whitespace<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_strange_whitespace());

    if session.check_issues {
        add_unexpected_char_issue(
            session,
            c,
            token_start.loc,
            IssueTag::UnexpectedSpaceCharacter,
            0.95,
        )
    }

    return session.token(TokenKind::Whitespace, token_start);
}

//
// Use SourceCharacters here, not WLCharacters
//
// Comments deal with (**) SourceCharacters
// Escaped characters do not work
//
// Important to process SourceCharacters here: (* \\.28\\.2a *)
//
fn tokenizer_handle_comment<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: SourceCharacter,
    mut policy: NextPolicy,
) -> TokenRef<'i> {
    //
    // comment is already started
    //

    assert!(c == '*');

    session.next_source_char(policy);

    incr_diagnostic!(Tokenizer_CommentCount);

    policy |= STRING_OR_COMMENT;

    let mut depth = 1;

    c = session.next_source_char(policy);

    loop {
        //
        // No need to check for comment length
        //

        match c {
            Char('(') => {
                c = session.next_source_char(policy);

                if c == '*' {
                    depth = depth + 1;

                    c = session.next_source_char(policy);
                }
            },
            Char('*') => {
                c = session.next_source_char(policy);

                if c == ')' {
                    // This comment is closing

                    depth = depth - 1;

                    if depth == 0 {
                        return session.token(TokenKind::Comment, token_start);
                    }

                    c = session.next_source_char(policy);
                }
            },
            EndOfFile => {
                return session
                    .token(TokenKind::Error_UnterminatedComment, token_start);
            },
            Char('\n' | '\r') | CRLF => {
                if session.compute_oob {
                    session.add_embedded_newline(token_start.loc);
                }

                c = session.next_source_char(policy);
            },
            Char('\t') => {
                if session.compute_oob {
                    session.add_embedded_tab(token_start.loc);
                }

                c = session.next_source_char(policy);
            },
            _ => {
                c = session.next_source_char(policy);
            },
        }
    } // loop
}

fn tokenizer_handle_mb_linear_syntax_blob<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == CODEPOINT_LINEARSYNTAX_OPENPAREN);

    let mut depth = 1;

    c = tokenizer_next_wl_character(session, token_start, policy);

    loop {
        match c.to_point() {
            Char(CODEPOINT_LINEARSYNTAX_OPENPAREN) => {
                depth = depth + 1;

                c = tokenizer_next_wl_character(session, token_start, policy);
            },
            Char(CODEPOINT_LINEARSYNTAX_CLOSEPAREN) => {
                depth = depth - 1;

                if depth == 0 {
                    return session
                        .token(TokenKind::LinearSyntaxBlob, token_start);
                }

                c = tokenizer_next_wl_character(session, token_start, policy);
            },
            EndOfFile => {
                return session.token(
                    TokenKind::Error_UnterminatedLinearSyntaxBlob,
                    token_start,
                );
            },
            _ => {
                c = tokenizer_next_wl_character(session, token_start, policy);
            },
        }
    } // loop
}

//
// a segment is: [a-z$]([a-z$0-9])*
// a symbol is: (segment)?(`segment)*
//
fn tokenizer_handle_symbol<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '`' || c.is_letterlike() || c.is_mb_letterlike());

    incr_diagnostic!(Tokenizer_SymbolCount);

    if c.is_letterlike() || c.is_mb_letterlike() {
        c = tokenizer_handle_symbol_segment(
            session,
            token_start,
            token_start.buf,
            token_start.loc,
            c,
            policy,
        );
    }

    //
    // if c == '`', then buffer is pointing past ` now
    //

    loop {
        if c.to_point() != '`' {
            break;
        }

        if session.check_issues && policy.contains(INSIDE_SLOT) {
            //
            // Something like  #`a
            //
            // It's hard to keep track of the ` characters, so just report the entire symbol. Oh well
            //

            let issue = syntax_issue(
                IssueTag::UndocumentedSlotSyntax,
                "The name following ``#`` is not documented to allow the **`** character.".into(),
                Severity::Warning,
                session.get_token_span(token_start.loc),
                0.33,
                vec![],
                vec![],
            );

            session.add_issue(issue);
        }

        c = tokenizer_current_wl_character(session, token_start, policy);

        if c.is_letterlike() || c.is_mb_letterlike() {
            let letterlike_buf = session.buffer();
            let letterlike_loc = session.src_loc;

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_handle_symbol_segment(
                session,
                token_start,
                letterlike_buf,
                letterlike_loc,
                c,
                policy,
            );
        } else {
            //
            // Something like  a`1
            //

            return session
                .token(TokenKind::Error_ExpectedLetterlike, token_start);
        }
    } // while

    return session.token(
        if policy.contains(INSIDE_SLOT) {
            TokenKind::String
        } else {
            TokenKind::Symbol
        },
        token_start,
    );
}

//
// Precondition: currentWLCharacter is letterlike
// Postcondition: buffer is pointing to first NON-SYMBOLSEGMENT character after all symbol segment characters
//
// return: the first NON-SYMBOLSEGMENT character after all symbol segment characters
//
fn tokenizer_handle_symbol_segment<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    _charBuf: Buffer,
    mut charLoc: Location,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> WLCharacter {
    assert!(c.is_letterlike() || c.is_mb_letterlike());

    if session.check_issues {
        if c.to_point() == '$' {
            if policy.contains(INSIDE_SLOT) {
                //
                // Something like  #$a
                //

                let issue = syntax_issue(
                    IssueTag::UndocumentedSlotSyntax,
                    "The name following ``#`` is not documented to allow the ``$`` character."
                        .to_owned(),
                    Severity::Warning,
                    session.get_token_span(charLoc),
                    0.33,
                    vec![],
                    vec![],
                );

                session.add_issue(issue);
            }
        } else if c.is_strange_letterlike() {
            add_unexpected_char_issue(
                session,
                c,
                charLoc,
                IssueTag::UnexpectedLetterlikeCharacter,
                0.85,
            )
        } else if c.is_mb_strange_letterlike() {
            add_unexpected_char_issue(
                session,
                c,
                charLoc,
                IssueTag::UnexpectedLetterlikeCharacter,
                0.80,
            )
        } else if !c.is_alpha() {
            if policy.contains(INSIDE_STRINGIFY_AS_TAG) {
                //
                // Something like  a::\[Beta]
                //

                let issue = syntax_issue(
                    IssueTag::UnexpectedCharacter,
                    "The tag has non-alphanumeric source characters."
                        .to_owned(),
                    Severity::Warning,
                    Span::new(charLoc, session.src_loc),
                    0.85,
                    vec![],
                    vec![],
                );

                session.add_issue(issue);
            }
        }
    }

    charLoc = session.src_loc;

    c = tokenizer_current_wl_character(session, token_start, policy);

    loop {
        if c.is_digit() {
            tokenizer_next_wl_character(session, token_start, policy);

            charLoc = session.src_loc;

            c = tokenizer_current_wl_character(session, token_start, policy);
        } else if c.is_letterlike() || c.is_mb_letterlike() {
            tokenizer_next_wl_character(session, token_start, policy);

            if session.check_issues {
                if c.to_point() == '$' {
                    if policy.contains(INSIDE_SLOT) {
                        //
                        // Something like  #$a
                        //

                        let issue = syntax_issue(
                            IssueTag::UndocumentedSlotSyntax,
                            format!("The name following ``#`` is not documented to allow the ``$`` character."),
                            Severity::Warning,
                            session.get_token_span( charLoc),
                            0.33,
                            vec![],
                            vec![],
                        );

                        session.add_issue(issue);
                    }
                } else if c.is_strange_letterlike() {
                    add_unexpected_char_issue(
                        session,
                        c,
                        charLoc,
                        IssueTag::UnexpectedLetterlikeCharacter,
                        0.85,
                    )
                } else if c.is_mb_strange_letterlike() {
                    add_unexpected_char_issue(
                        session,
                        c,
                        charLoc,
                        IssueTag::UnexpectedLetterlikeCharacter,
                        0.80,
                    )
                } else if !c.is_alpha_or_digit() {
                    if policy.contains(INSIDE_STRINGIFY_AS_TAG) {
                        //
                        // Something like  a::b\[Beta]
                        //

                        let issue = syntax_issue(
                            IssueTag::UnexpectedCharacter,
                            "The tag has non-alphanumeric source characters."
                                .to_owned(),
                            Severity::Warning,
                            Span::new(charLoc, session.src_loc),
                            0.85,
                            vec![],
                            vec![],
                        );

                        session.add_issue(issue);
                    }
                }
            }

            charLoc = session.src_loc;

            c = tokenizer_current_wl_character(session, token_start, policy);
        } else if c.to_point() == '`' {
            //
            // Advance past trailing `
            //

            tokenizer_next_wl_character(session, token_start, policy);

            break;
        } else {
            break;
        }
    } // while

    return c;
}

fn tokenizer_handle_string<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    mut policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '"');

    if session.check_issues && policy.contains(INSIDE_SLOT) {
        //
        // Something like  #"a"
        //

        let issue = syntax_issue(
            IssueTag::UndocumentedSlotSyntax,
            format!("The name following ``#`` is not documented to allow the ``\"`` character."),
            Severity::Warning,
            session.get_token_span(token_start.loc),
            0.33,
            vec![],
            vec![],
        );

        session.add_issue(issue);
    }

    let mut quot_offset: Option<usize> = None;
    let mut fast = false;
    let mut terminated = false;

    if feature::FAST_STRING_SCAN
        && !session.compute_oob
        && !session.check_issues
    {
        //
        // !CHECK_ISSUES (so do not need to warn about strange SourceCharacters)
        // !COMPUTE_OOB (so do not need to care about embedded newlines or tabs)
        //
        // FAST_STRING_SCAN (as a final check that skipping bad SourceCharacters and WLCharacters is ok)
        //

        //
        // The idea is to use memchr to scan for the next '"' character byte and then just jump to it.
        //
        // This is faster than explicitly calling TheCharacterDecoder->nextWLCharacter0 over and over again.
        //
        // Diagnostics that count SourceCharacters and WLCharacters will not be accurate inside of fast strings.
        //
        // Bad SourceCharacters will not be detected. This means that incomplete sequences, stray surrogates, and BOM will not be reported.
        //
        // Bad WLCharacters will not be detected. This means that badly escaped characters will not be reported.
        //

        let quot_relative_offset = memchr::memchr(b'"', session.buffer());

        quot_offset = quot_relative_offset
            .map(|buffer_offset| session.offset + buffer_offset);

        if let Some(quot_offset) = quot_offset {
            let prev_char = match quot_offset.checked_sub(1) {
                Some(index) => Some(session.buffer_at(index)[0]),
                None => None,
            };

            if prev_char != Some(b'\\') {
                //
                // first double-quote character is NOT preceded by a backslash character
                //

                fast = true;
                terminated = true;
            } else {
                //
                // there is a backslash character, so fall-through to SLOW
                //

                fast = false;
                terminated = true;
            }
        } else {
            //
            // unterminated, so fall-through to SLOW
            //

            fast = false;
            terminated = false;
        }
    }

    if fast {
        incr_diagnostic!(Tokenizer_StringFastCount);

        //
        // just set buffer to quotPtr + 1
        //

        if terminated {
            session.offset = safe_expect!(quot_offset, "quot_offset should be set when terminated=true") + 1;

            return session.token(TokenKind::String, token_start);
        } else {
            session.offset = session.input.len();
            session.was_eof = true;

            return session
                .token(TokenKind::Error_UnterminatedString, token_start);
        }
    }

    //
    // SLOW FALL-THROUGH
    //

    incr_diagnostic!(Tokenizer_StringSlowCount);

    policy |= STRING_OR_COMMENT;

    loop {
        c = tokenizer_next_wl_character(session, token_start, policy);

        match c.to_point() {
            Char('"') => {
                return session.token(TokenKind::String, token_start);
            },
            EndOfFile => {
                return session
                    .token(TokenKind::Error_UnterminatedString, token_start);
            },
            Char('\n' | '\r') | CRLF if session.compute_oob => {
                session.add_embedded_newline(token_start.loc);
            },
            Char('\t') if session.compute_oob => {
                session.add_embedded_tab(token_start.loc);
            },
            _ => (),
        }
    } // while
}

fn tokenizer_handle_string_stringify_as_tag<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    //
    // Nothing to assert
    //

    //
    // magically turn into a string
    //

    if c.is_letterlike() || c.is_mb_letterlike() {
        let letterlike_buf = session.buffer();
        let letterlike_loc = session.src_loc;

        tokenizer_handle_symbol_segment(
            session,
            token_start,
            letterlike_buf,
            letterlike_loc,
            c,
            policy,
        );

        return session.token(TokenKind::String, token_start);
    }

    //
    // Something like  a::5
    //

    return Token::new2(
        TokenKind::Error_ExpectedTag,
        BufferAndLength::from_buffer_with_len(token_start.buf, 0),
        Span::at(token_start.loc),
    );
}

//
// Use SourceCharacters here, not WLCharacters
//
fn tokenizer_handle_string_stringify_as_file<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: SourceCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    //
    // Nothing to assert
    //

    //
    // magically turn into a string
    //

    //
    // sync-up with current character
    //

    match c {
        #[rustfmt::skip]
        Char(
            'A'..='Z' | 'a'..='z' | '0'..='9'
            | '$' | '`' | '/' | '.' | '\\' | '!'
            | '-' | '_' | ':' | '*' | '~' | '?',
        ) => {
            c = session.peek_source_char(policy);
        },
        Char('[') => {
            // handle matched pairs of [] enclosing any characters other than spaces, tabs, and newlines

            match tokenizer_handle_file_ops_brackets(
                session,
                token_start,
                c,
                policy,
            ) {
                HandledFileOpsBracket::Finished(char) => c = char,
                HandledFileOpsBracket::UnterminatedFileString => {
                    return session.token(
                        TokenKind::Error_UnterminatedFileString,
                        token_start,
                    );
                },
            }
        },
        _ => {
            //
            // Something like  <<EOF
            //
            // EndOfFile is special because there is no source
            //
            // So invent source
            //

            return session
                .error_token_at(TokenKind::Error_ExpectedFile, token_start);
        },
    }

    loop {
        //
        // tutorial/OperatorInputForms
        //
        // File Names
        //
        // Any file name can be given in quotes after <<, >>, and >>>.
        // File names can also be given without quotes if they contain only alphanumeric
        // characters and the characters `, /, ., \[Backslash], !, -, _, :, $, *, ~, and ?, together with
        // matched pairs of square brackets enclosing any characters other than spaces, tabs, and newlines.
        // Note that file names given without quotes can be followed only by spaces, tabs, or newlines, or
        // by the characters ), ], or }, as well as semicolons and commas.
        //

        match c {
            #[rustfmt::skip]
            Char(
                'A'..='Z' | 'a'..='z' | '0'..='9'
                | '`' | '/' | '.' | '\\' | '!'
                | '-' | '_' | ':' | '*' | '~' | '?',
            ) => {
                session.next_source_char(policy);

                c = session.peek_source_char(policy);
            },
            Char('[') => {
                // handle matched pairs of [] enclosing any characters other than spaces, tabs, and newlines

                session.next_source_char(policy);

                match tokenizer_handle_file_ops_brackets(
                    session,
                    token_start,
                    c,
                    policy,
                ) {
                    HandledFileOpsBracket::Finished(char) => {
                        c = char;
                    },
                    HandledFileOpsBracket::UnterminatedFileString => {
                        return session.token(
                            TokenKind::Error_UnterminatedFileString,
                            token_start,
                        );
                    },
                }
            },
            _ => {
                return session.token(TokenKind::String, token_start);
            },
        }
    } // while
}

/// Outcome from [`tokenizer_handle_file_ops_brackets()`].
enum HandledFileOpsBracket {
    Finished(SourceCharacter),
    UnterminatedFileString,
}


//
// Handle parsing the brackets in:
// a >> foo[[]]
//
// tutorial/OperatorInputForms
//
// File Names
//
// handle matched pairs of [] enclosing any characters other than spaces, tabs, and newlines
//
// Use SourceCharacters here, not WLCharacters
//
fn tokenizer_handle_file_ops_brackets<'i>(
    session: &mut Tokenizer<'i>,
    _token_start: &TokenStart<'i>,
    mut c: SourceCharacter,
    policy: NextPolicy,
) -> HandledFileOpsBracket {
    assert!(c == '[');

    //
    // sync-up with current character
    //

    c = session.peek_source_char(policy);

    let mut depth = 1;

    loop {
        match c {
            //
            // Spaces and Newlines
            //
            Char(' ' | '\t' | ASCII_VTAB | ASCII_FORM_FEED | '\n' | '\r')
            | CRLF => {
                //
                // Cannot have spaces in the string here, so bail out
                //

                return HandledFileOpsBracket::UnterminatedFileString;
            },
            EndOfFile => {
                return HandledFileOpsBracket::UnterminatedFileString;
            },
            Char('[') => {
                depth = depth + 1;

                session.next_source_char(policy);

                c = session.peek_source_char(policy);
            },
            Char(']') => {
                depth = depth - 1;

                session.next_source_char(policy);

                c = session.peek_source_char(policy);

                if depth == 0 {
                    return HandledFileOpsBracket::Finished(c);
                }
            },
            _ => {
                if c.is_mb_whitespace() || c.is_mb_newline() {
                    return HandledFileOpsBracket::UnterminatedFileString;
                }

                session.next_source_char(policy);

                c = session.peek_source_char(policy);
            },
        }
    } // loop
}

//
//digits                  integer
//digits.digits           approximate number
//base^^digits            integer in specified base
//base^^digits.digits     approximate number in specified base
//mantissa*^n             scientific notation (mantissa*10^n)
//base^^mantissa*^n       scientific notation in specified base (mantissa*base^n)
//number`                 machine-precision approximate number
//number`s                arbitrary-precision number with precision s
//number``s               arbitrary-precision number with accuracy s
//
// base = (digits^^)?
// approximate = digits(.digits?)?|.digits
// precision = `(-?approximate)?
// accuracy = ``-?approximate
// mantissa = approximate+(precision|accuracy)?
// exponent = (*^-?digits)?
//
// numer = base+mantissa+exponent
//
fn tokenizer_handle_number<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_digit() || c.to_point() == '.');

    incr_diagnostic!(Tokenizer_NumberCount);

    let mut ctxt = NumberTokenizationContext::new();

    let mut leading_digits_count: u32 = 0;

    //
    // leading_digits_end_mark will point to the first character after all leading digits and ^^
    //
    // 16^^0.F
    //      ^leading_digits_end_mark
    //
    // 16^^.F
    //     ^leading_digits_end_mark
    //
    // 0.123
    //  ^leading_digits_end_mark
    //
    // TODO(cleanup): Replace InputMark::new() with new InputMark::from_token_start()?
    let mut leading_digits_end_mark =
        InputMark::new(session.offset_of(token_start.buf), token_start.loc);

    let mut caret1Buf: Option<Buffer> = None;
    let mut caret_1_mark: Option<InputMark> = None;

    let mut star_buf: Option<Buffer> = None;
    let mut star_mark: Option<InputMark> = None;

    if c.is_digit() {
        //        leading_digits_count++;

        //
        // Count leading zeros
        //

        //
        // 002^^111
        //   ^non_zero_start_buf
        //
        let mut non_zero_start_buf = token_start.buf;

        if c.to_point() == '0' {
            let mut leading_zero_count: u32 = 0;
            (leading_zero_count, c) =
                tokenizer_handle_zeros(session, token_start, policy, c);

            leading_digits_count += leading_zero_count;

            non_zero_start_buf = session.buffer();
        }

        //
        // Count the rest of the leading digits
        //

        leading_digits_end_mark = session.mark();

        if c.is_digit() {
            let mut count: u32 = 0;
            (count, c) =
                tokenizer_handle_digits(session, token_start, policy, c);

            leading_digits_count += count;

            leading_digits_end_mark = session.mark();
        }

        if policy.contains(INTEGER_SHORT_CIRCUIT) {
            if session.check_issues && c.to_point() == '.' {
                //
                // Something like  #2.a
                //

                let dot_loc = session.src_loc;

                let mut actions: Vec<CodeAction> = Vec::new();

                actions.push(CodeAction::insert_text(
                    "Insert space".to_owned(),
                    Span::at(dot_loc),
                    " ".to_owned(),
                ));

                let issue = format_issue(
                    IssueTag::Ambiguous,
                    format!("Ambiguous syntax."),
                    Severity::Formatting,
                    session.get_token_span(dot_loc),
                    1.0,
                    actions,
                    vec![],
                );

                session.add_issue(issue);
            }

            //
            // Success!
            //

            return session.token(ctxt.compute_tok(), token_start);
        }

        match c.to_point() {
            //
            // These are the possible next characters for a number
            //
            Char('^' | '*' | '.' | '`') => {
                if c.to_point() == '^' {
                    caret1Buf = Some(session.buffer());
                    caret_1_mark = Some(session.mark());

                    assert!(utils::if_ascii_wl_character(
                        safe_expect!(caret1Buf, "caret1Buf should be set")[0],
                        b'^'
                    ));
                } else if c.to_point() == '*' {
                    star_buf = Some(session.buffer());
                    star_mark = Some(session.mark());

                    assert!(utils::if_ascii_wl_character(
                        star_buf.unwrap()[0],
                        b'*'
                    ));
                }

                //
                // Preserve c, but advance buffer to next character
                //

                tokenizer_next_wl_character(session, token_start, policy);
            },
            _ => {
                //
                // Something else
                //

                //
                // Success!
                //

                return session.token(ctxt.compute_tok(), token_start);
            },
        }

        if c.to_point() == '^' {
            //
            // Could be 16^^blah
            //

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() != '^' {
                //
                // Something like  2^a
                //
                // Must now do surgery and back up
                //

                session.seek(caret_1_mark.unwrap());

                //
                // Success!
                //

                return session.token(ctxt.compute_tok(), token_start);
            }

            assert!(c.to_point() == '^');

            //
            // Something like  2^^
            //
            // Must be a number
            //

            tokenizer_next_wl_character(session, token_start, policy);

            if non_zero_start_buf == safe_expect!(caret1Buf, "caret1Buf should be set in base parsing") {
                //
                // Something like  0^^2
                //

                ctxt.InvalidBase = true;
            } else {
                // PRE_COMMIT: Compute string length differently
                let base_str_len = BufferAndLength::between(
                    non_zero_start_buf,
                    safe_expect!(caret1Buf, "caret1Buf should be set in base parsing"),
                )
                .buf
                .len();

                //
                // bases can only be between 2 and 36, so we know they can only be 1 or 2 characters
                //
                if base_str_len > 2 {
                    ctxt.InvalidBase = true;
                } else if base_str_len == 2 {
                    let d1 = i32::from(utils::to_digit(non_zero_start_buf[0]));
                    let d0 = i32::from(utils::to_digit(non_zero_start_buf[1]));
                    ctxt.Base = d1 * 10 + d0;
                } else {
                    assert!(base_str_len == 1);

                    let d0 = i32::from(utils::to_digit(non_zero_start_buf[0]));
                    ctxt.Base = d0;
                }

                if !(2 <= ctxt.Base && ctxt.Base <= 36) {
                    ctxt.InvalidBase = true;
                }
            }

            c = tokenizer_current_wl_character(session, token_start, policy);

            //
            // What can come after ^^ ?
            //

            leading_digits_count = 0;

            match c.to_point() {
                Char('A'..='Z' | 'a'..='z' | '0'..='9') => {
                    //
                    // Something like  16^^A
                    //

                    (leading_digits_count, c) = tokenizer_handle_alpha_or_digits(
                        session,
                        token_start,
                        c,
                        ctxt.Base,
                        policy,
                        &mut ctxt,
                    );

                    leading_digits_end_mark = session.mark();

                    match c.to_point() {
                        //
                        // These are the possible next characters for a number
                        //
                        Char('*' | '.' | '`') => {
                            if c.to_point() == '*' {
                                star_buf = Some(session.buffer());
                                star_mark = Some(session.mark());

                                assert!(utils::if_ascii_wl_character(
                                    star_buf.unwrap()[0],
                                    b'*'
                                ));
                            }

                            //
                            // Preserve c, but advance buffer to next character
                            //

                            tokenizer_next_wl_character(
                                session,
                                token_start,
                                policy,
                            );
                        },
                        _ => {
                            //
                            // Something else
                            //

                            //
                            // Success!
                            //

                            return session
                                .token(ctxt.compute_tok(), token_start);
                        },
                    }
                },
                Char('.') => {
                    //
                    // Something like  2^^.0
                    //

                    leading_digits_end_mark = session.mark();

                    //
                    // Preserve c, but advance buffer to next character
                    //

                    tokenizer_next_wl_character(session, token_start, policy);
                },
                EndOfFile => {
                    //
                    // Something like  2^^<EOF>
                    //

                    //
                    // Make sure that bad character is read
                    //

                    tokenizer_next_wl_character(session, token_start, policy);

                    c = tokenizer_current_wl_character(
                        session,
                        token_start,
                        policy,
                    );

                    // nee TokenKind::Error_ExpectedDIGIT
                    return session.token(TokenKind::Error_Number, token_start);
                },
                _ => {
                    //
                    // Something like  2^^@
                    //

                    // nee TokenKind::Error_UNRECOGNIZEDDIGIT
                    return session.token(TokenKind::Error_Number, token_start);
                },
            }
        } // if (c.to_point() == '^')
    } // if (c.is_digit())

    if c.to_point() == '.' {
        // PRE_COMMIT: Rename this assert
        // assert!(utils::if_ascii_wl_character(*(session.buffer - 1), b'.'));

        let handled: HandledFractionalPart;
        (handled, c) = tokenizer_handle_possible_fractional_part(
            session,
            token_start,
            leading_digits_end_mark,
            c,
            ctxt.Base,
            policy,
            &mut ctxt,
        );

        match handled {
            HandledFractionalPart::Bailout => {
                if leading_digits_count == 0 {
                    //
                    // Something like  2^^..
                    //

                    // nee TokenKind::Error_UNHANDLEDDOT
                    return session.token(TokenKind::Error_Number, token_start);
                }

                //
                // Something like  0..
                //

                //
                // Success!
                //

                return session.token(ctxt.compute_tok(), token_start);
            },
            HandledFractionalPart::Count(0) => {
                if leading_digits_count == 0 {
                    //
                    // Something like  2^^.
                    //

                    // nee TokenKind::Error_UNHANDLEDDOT
                    return session.token(TokenKind::Error_Number, token_start);
                }

                //
                // Something like  0.
                //

                ctxt.Real = true;

                match c.to_point() {
                    //
                    // These are the possible next characters for a number
                    //
                    Char('`' | '*') => {
                        if c.to_point() == '*' {
                            star_buf = Some(session.buffer());
                            star_mark = Some(session.mark());

                            assert!(utils::if_ascii_wl_character(
                                star_buf.unwrap()[0],
                                b'*'
                            ));
                        }

                        //
                        // Preserve c, but advance buffer to next character
                        //

                        tokenizer_next_wl_character(session, token_start, policy);
                    },
                    _ => {
                        //
                        // Something like  123.
                        //

                        //
                        // Success!
                        //

                        return session.token(ctxt.compute_tok(), token_start);
                    },
                }
            },
            HandledFractionalPart::Count(_) => {
                //
                // Something like  123.456
                //

                ctxt.Real = true;

                match c.to_point() {
                    //
                    // These are the possible next characters for a number
                    //
                    Char('`' | '*') => {
                        if c.to_point() == '*' {
                            star_buf = Some(session.buffer());
                            star_mark = Some(session.mark());

                            assert!(utils::if_ascii_wl_character(
                                star_buf.unwrap()[0],
                                b'*'
                            ));
                        }

                        //
                        // Preserve c, but advance buffer to next character
                        //

                        tokenizer_next_wl_character(session, token_start, policy);
                    },
                    _ => {
                        //
                        // Something like  123.456
                        //

                        //
                        // Success!
                        //

                        return session.token(ctxt.compute_tok(), token_start);
                    },
                }
            },
        }
    }

    assert!(c.to_point() == '`' || c.to_point() == '*');

    //
    // Handle all ` logic here
    //
    // foo`
    // foo`bar
    // foo``bar
    //
    if c.to_point() == '`' {
        ctxt.Real = true;

        c = tokenizer_current_wl_character(session, token_start, policy);

        let mut accuracy = false;
        let mut sign = false;
        let mut prec_or_acc_supplied = false;

        let mut sign_mark: Option<InputMark> = None;

        if c.to_point() == '`' {
            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            accuracy = true;
        }

        match c.to_point() {
            Char('-' | '+') => {
                //
                // Something like  1.2`-
                //

                // Take one character so we can display this
                let sign_buf =
                    BufferAndLength::from_buffer_with_len(session.buffer(), 1);

                sign_mark = Some(session.mark());

                assert!(
                    utils::if_ascii_wl_character(sign_buf.buf[0], b'-')
                        || utils::if_ascii_wl_character(sign_buf.buf[0], b'+')
                );

                //
                // Eat the sign
                //

                tokenizer_next_wl_character(session, token_start, policy);

                c = tokenizer_current_wl_character(session, token_start, policy);

                match c.to_point() {
                    //
                    // These are the possible next characters for a number
                    //
                    Char(
                        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8'
                        | '9',
                    ) => {
                        //
                        // Something like  1.2`-3
                        //

                        sign = true;

                        if session.check_issues {
                            if accuracy {
                                //
                                // do not warn about 1.2``+3 for now
                                //
                            } else {
                                let issue = syntax_issue(
                                    IssueTag::UnexpectedSign,
                                    format!(
                                        "The real number has a ``{}`` sign in its precision specification.",
                                        sign_buf.as_str()
                                    ),
                                    Severity::Warning,
                                    Span::at(safe_expect!(sign_mark, "sign_mark should be set when sign found").src_loc),
                                    0.95,
                                    vec![],
                                    vec!["This is usually unintentional.".into()],
                                );

                                session.add_issue(issue);
                            }
                        }
                    },
                    Char('.') => {
                        //
                        // Something like  1.2`-.3
                        //

                        sign = true;

                        if session.check_issues {
                            if accuracy {
                                //
                                // do not warn about 1.2``+.3 for now
                                //
                            } else {
                                let issue = syntax_issue(
                                    IssueTag::UnexpectedSign,
                                    format!(
                                        "The real number has a ``{}`` sign in its precision specification.",
                                        sign_buf.as_str()
                                    ),
                                    Severity::Warning,
                                    Span::at(safe_expect!(sign_mark, "sign_mark should be set when sign found").src_loc),
                                    0.95,
                                    vec![],
                                    vec!["This is usually unintentional.".into()],
                                );

                                session.add_issue(issue);
                            }
                        }
                    },
                    _ => {
                        //
                        // Something like  1.2`->
                        //

                        if accuracy {
                            //
                            // Something like  1.2``->3
                            //

                            // nee TokenKind::Error_ExpectedACCURACY
                            return session
                                .token(TokenKind::Error_Number, token_start);
                        }

                        //
                        // Something like  1.2`->3  or  1`+#
                        //
                        // Must now do surgery and back up
                        //
                        tokenizer_backup_and_warn(session, safe_expect!(sign_mark, "sign_mark should be set when sign found"));

                        //
                        // Success!
                        //

                        return session.token(ctxt.compute_tok(), token_start);
                    },
                }
            }, // case '-': case '+'
            _ => (),
        }

        match c.to_point() {
            Char('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9') => {
                let mut count: u32 = 0;

                (count, c) =
                    tokenizer_handle_digits(session, token_start, policy, c);

                if count > 0 {
                    prec_or_acc_supplied = true;
                }

                match c.to_point() {
                    //
                    // These are the possible next characters for a number
                    //
                    Char('.') => {},
                    Char('*') => {},
                    _ => {
                        //
                        // Success!
                        //

                        return session.token(ctxt.compute_tok(), token_start);
                    },
                }
            },
            _ => (),
        }

        match c.to_point() {
            Char('.') => {
                let dot_buf = session.buffer();
                let dot_mark = session.mark();

                assert!(utils::if_ascii_wl_character(dot_buf[0], b'.'));

                let mut tentative_actual_decimal_point = false;

                //
                // If there was already a sign, or if the leading digits have already been supplied,
                // then this is an actual decimal point
                //
                if sign || prec_or_acc_supplied {
                    tentative_actual_decimal_point = true;
                }

                //
                // Need to decide if the  .  here is actual radix point, or something like
                // the . in  123`.xxx  (which is Dot)
                //

                if !tentative_actual_decimal_point {
                    //
                    // Need to peek ahead
                    //
                    // Something like  123`.xxx
                    //

                    // look ahead

                    tokenizer_next_wl_character(session, token_start, policy);

                    let next_char = tokenizer_current_wl_character(
                        session,
                        token_start,
                        policy,
                    );

                    if !next_char.is_digit() {
                        if accuracy {
                            //
                            // Something like  123``.EOF
                            //

                            // TokenKind::Error_ExpectedDIGIT
                            return session
                                .token(TokenKind::Error_Number, token_start);
                        }

                        if next_char.is_sign() {
                            //
                            // Something like  123`.+4
                            //

                            tokenizer_next_wl_character(
                                session,
                                token_start,
                                policy,
                            );

                            // nee TokenKind::Error_ExpectedDIGIT
                            return session
                                .token(TokenKind::Error_Number, token_start);
                        }

                        //
                        // Something like  123`.xxx  where the . could be a Dot operator
                        //
                        // Number stops at `
                        //
                        // NOT actual decimal point
                        //
                        // Must now do surgery and back up
                        //

                        tokenizer_backup_and_warn(session, dot_mark);

                        //
                        // Success!
                        //

                        return session.token(ctxt.compute_tok(), token_start);
                    } else {
                        //
                        // digit
                        //

                        c = next_char;
                    }
                } else {
                    //
                    // actual decimal point
                    //

                    tokenizer_next_wl_character(session, token_start, policy);

                    c = tokenizer_current_wl_character(
                        session,
                        token_start,
                        policy,
                    );
                }

                //
                // actual decimal point
                //

                let handled: HandledFractionalPart;
                //
                // The base to use inside of precision/accuracy processing is 0, i.e., implied 10
                //
                let base_to_use: i32 = 0;

                (handled, c) = tokenizer_handle_possible_fractional_part_past_dot(
                    session,
                    token_start,
                    dot_mark,
                    c,
                    base_to_use,
                    policy,
                    &mut ctxt,
                );

                match handled {
                    HandledFractionalPart::Bailout => {
                        if prec_or_acc_supplied {
                            //
                            // Something like  6`5..
                            //

                            //
                            // Success!
                            //

                            return session
                                .token(ctxt.compute_tok(), token_start);
                        }

                        if sign {
                            //
                            // Something like  1`+..
                            //

                            tokenizer_backup_and_warn(
                                session,
                                safe_expect!(sign_mark, "sign_mark should be set when sign found"),
                            );

                            //
                            // Success!
                            //

                            return session
                                .token(ctxt.compute_tok(), token_start);
                        }

                        assert!(false);
                    },
                    HandledFractionalPart::Count(0) => {},
                    HandledFractionalPart::Count(_) => {
                        prec_or_acc_supplied = true;
                    },
                }

                if !prec_or_acc_supplied {
                    //
                    // Something like  1`+.a
                    //

                    // nee TokenKind::Error_ExpectedDIGIT
                    return session.token(TokenKind::Error_Number, token_start);
                }
            }, // case '.'
            _ => (),
        } // switch (c.to_point())

        match c.to_point() {
            //
            // These are the possible next characters for a number
            //
            Char('*') => {
                if accuracy {
                    if !prec_or_acc_supplied {
                        //
                        // Something like  123.45``*^6
                        //

                        // nee TokenKind::Error_ExpectedACCURACY
                        return session
                            .token(TokenKind::Error_Number, token_start);
                    }
                }

                star_buf = Some(session.buffer());
                star_mark = Some(session.mark());

                assert!(utils::if_ascii_wl_character(star_buf.unwrap()[0], b'*'));

                //
                // Preserve c, but advance buffer to next character
                //

                tokenizer_next_wl_character(session, token_start, policy);
            },
            _ => {
                if accuracy {
                    if !prec_or_acc_supplied {
                        //
                        // Something like  123``EOF
                        //

                        // nee TokenKind::Error_ExpectedACCURACY
                        return session
                            .token(TokenKind::Error_Number, token_start);
                    }
                }

                //
                // Success!
                //

                return session.token(ctxt.compute_tok(), token_start);
            },
        }
    } // if (c.to_point() == '`')

    assert!(c.to_point() == '*');

    assert!(utils::if_ascii_wl_character(star_buf.unwrap()[0], b'*'));

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() != '^' {
        //
        // Something like  1*a
        //
        // Must now do surgery and back up
        //

        session.seek(star_mark.unwrap());

        //
        // Success!
        //

        return session.token(ctxt.compute_tok(), token_start);
    }

    assert!(c.to_point() == '^');

    //
    // c is '^'
    //
    // So now examine *^ notation
    //

    tokenizer_next_wl_character(session, token_start, policy);

    c = tokenizer_current_wl_character(session, token_start, policy);

    if let Char(c2 @ ('+' | '-')) = c.to_point() {
        if c2 == '-' {
            ctxt.NegativeExponent = true;
        }

        tokenizer_next_wl_character(session, token_start, policy);
        c = tokenizer_current_wl_character(session, token_start, policy);
    }

    if !c.is_digit() {
        //
        // Something like  123*^-<EOF>
        //

        // TokenKind::Error_ExpectedEXPONENT
        return session.token(TokenKind::Error_Number, token_start);
    }

    assert!(c.is_digit());

    //
    // Count leading zeros in exponent
    //
    if c.to_point() == '0' {
        let _exponentLeadingZeroCount: u32;

        (_exponentLeadingZeroCount, c) =
            tokenizer_handle_zeros(session, token_start, policy, c);
    }

    if c.is_digit() {
        (ctxt.NonZeroExponentDigitCount, c) =
            tokenizer_handle_digits(session, token_start, policy, c);
    }

    if c.to_point() != '.' {
        //
        // Success!
        //

        return session.token(ctxt.compute_tok(), token_start);
    }

    assert!(c.to_point() == '.');

    let dot_buf = session.buffer();
    let dot_mark = session.mark();

    assert!(utils::if_ascii_wl_character(dot_buf[0], b'.'));

    tokenizer_next_wl_character(session, token_start, policy);

    c = tokenizer_current_wl_character(session, token_start, policy);

    let handled: HandledFractionalPart;
    (handled, c) = tokenizer_handle_possible_fractional_part_past_dot(
        session,
        token_start,
        dot_mark,
        c,
        ctxt.Base,
        policy,
        &mut ctxt,
    );

    match handled {
        HandledFractionalPart::Bailout => {
            //
            // Something like  123*^2..
            //
            // The first . is not actually a radix point
            //

            //
            // Success!
            //

            return session.token(ctxt.compute_tok(), token_start);
        },
        HandledFractionalPart::Count(_) => {
            //
            // Something like  123*^0.5
            //
            // Make this an error; do NOT make this Dot[123*^0, 5]
            //

            // nee TokenKind::Error_ExpectedEXPONENT
            return session.token(TokenKind::Error_Number, token_start);
        },
    }
}

impl NumberTokenizationContext {
    pub(crate) fn new() -> Self {
        NumberTokenizationContext {
            InvalidBase: false,
            UnrecognizedDigit: false,
            NegativeExponent: false,
            Real: false,
            NonZeroExponentDigitCount: 0,
            Base: 0,
        }
    }

    fn compute_tok(&self) -> TokenKind {
        //
        // We wait until returning to handle these errors because we do not want invalid base or unrecognized digit to prevent further parsing
        //
        // e.g., we want  0^^1.2``3  to parse completely before returning the error
        //

        if self.InvalidBase {
            // nee TokenKind::Error_INVALIDBASE
            return TokenKind::Error_Number;
        }

        if self.UnrecognizedDigit {
            // nee TokenKind::Error_UNRECOGNIZEDDIGIT
            return TokenKind::Error_Number;
        }

        if self.Real {
            return TokenKind::Real;
        }

        if self.NegativeExponent && self.NonZeroExponentDigitCount != 0 {
            //
            // Something like  1*^-2..
            //

            return TokenKind::Rational;
        }

        return TokenKind::Integer;
    }
}

/// Outcome from attempting to handle the fractional part (i.e. the digits after
/// the decimal point) of a number.
enum HandledFractionalPart {
    Count(u32),
    /// Bailed out from handling the fractional part because the first dot
    /// was followed by a second, e.g. the input was `0..`.
    Bailout,
}

//
// Precondition: currentWLCharacter is NOT in String
//
// Return: number of digits handled after ., possibly 0, or -1 if error
//
fn tokenizer_handle_possible_fractional_part<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    dot_mark: InputMark,
    mut c: WLCharacter,
    base: i32,
    policy: NextPolicy,
    ctxt: &mut NumberTokenizationContext,
) -> (HandledFractionalPart, WLCharacter) {
    assert!(c.to_point() == '.');

    c = tokenizer_current_wl_character(session, token_start, policy);
    return tokenizer_handle_possible_fractional_part_past_dot(
        session,
        token_start,
        dot_mark,
        c,
        base,
        policy,
        ctxt,
    );
}

/// Precondition: currentWLCharacter is NOT in String
///
/// Returns:
///
/// * number of digits handled after ., possibly 0
/// * UNRECOGNIZED_DIGIT if base error
/// * [`HandledFractionalPart::Bailout`] if not a radix point (and also backup before dot)
///
fn tokenizer_handle_possible_fractional_part_past_dot<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    dot_mark: InputMark,
    mut c: WLCharacter,
    base: i32,
    policy: NextPolicy,
    ctxt: &mut NumberTokenizationContext,
) -> (HandledFractionalPart, WLCharacter) {
    //
    // Nothing to assert
    //

    if c.to_point() == '.' {
        //
        // Something like  0..
        //
        // The first . is not actually a radix point
        //
        // Must now do surgery and back up
        //

        tokenizer_backup_and_warn(session, dot_mark);

        c = tokenizer_current_wl_character(session, token_start, policy);

        return (HandledFractionalPart::Bailout, c);
    }

    if c.is_alpha_or_digit() {
        let handled: u32;
        (handled, c) = tokenizer_handle_alpha_or_digits(
            session,
            token_start,
            c,
            base,
            policy,
            ctxt,
        );

        if handled > 0 {
            if session.check_issues && c.to_point() == '.' {
                //
                // Something like  1.2.3
                //

                let mut actions = Vec::new();

                actions.push(CodeAction::insert_text(
                    "Insert ``*``".into(),
                    Span::at(dot_mark.src_loc),
                    "*".into(),
                ));

                let issue = syntax_issue(
                    IssueTag::UnexpectedImplicitTimes,
                    format!("Suspicious syntax."),
                    Severity::Error,
                    Span::at(dot_mark.src_loc),
                    0.99,
                    actions,
                    vec![],
                );

                session.add_issue(issue);
            }
        }

        return (HandledFractionalPart::Count(handled), c);
    }

    return (HandledFractionalPart::Count(0), c);
}

fn tokenizer_backup_and_warn<'i>(session: &mut Tokenizer<'i>, reset: InputMark) {
    if session.check_issues {
        let mut actions: Vec<CodeAction> = Vec::new();

        actions.push(CodeAction::insert_text(
            "Insert space".into(),
            Span::at(reset.src_loc),
            " ".into(),
        ));

        let issue = format_issue(
            IssueTag::Ambiguous,
            "Ambiguous syntax.".into(),
            Severity::Formatting,
            Span::at(reset.src_loc),
            1.0,
            actions,
            vec![],
        );

        session.add_issue(issue);
    }

    session.seek(reset);
}

//
// Precondition: currentWLCharacter is 0
// Postcondition: buffer is pointing to first NON-ZERO character after all zeros
//
// return: the first NON-ZERO character after all digits
//
fn tokenizer_handle_zeros<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    policy: NextPolicy,
    mut c: WLCharacter,
) -> (u32, WLCharacter) {
    assert!(c.to_point() == '0');

    let mut count = 1;

    c = tokenizer_current_wl_character(session, token_start, policy);

    loop {
        if c.to_point() != '0' {
            break;
        }

        tokenizer_next_wl_character(session, token_start, policy);

        c = tokenizer_current_wl_character(session, token_start, policy);

        count += 1;
    } // while

    return (count, c);
}

//
// Precondition: currentWLCharacter is a digit
// Postcondition: buffer is pointing to first NON-DIGIT character after all digits
//
// return: the first NON-DIGIT character after all digits
//
fn tokenizer_handle_digits<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    policy: NextPolicy,
    mut c: WLCharacter,
) -> (u32, WLCharacter) {
    assert!(c.is_digit());

    let mut count = 1;

    c = tokenizer_current_wl_character(session, token_start, policy);

    loop {
        if !c.is_digit() {
            break;
        }

        tokenizer_next_wl_character(session, token_start, policy);

        c = tokenizer_current_wl_character(session, token_start, policy);

        count += 1;
    } // while

    return (count, c);
}

//
// Precondition: currentWLCharacter is NOT in String
// Postcondition: currentWLCharacter is the first WLCharacter AFTER all good digits or alphas
//
// Return: number of digits handled, possibly 0, or -1 if error
//
fn tokenizer_handle_alpha_or_digits<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    base: i32,
    policy: NextPolicy,
    ctxt: &mut NumberTokenizationContext,
) -> (u32, WLCharacter) {
    assert!(c.is_alpha_or_digit());

    let mut count = 0;

    loop {
        if !c.is_alpha_or_digit() {
            break;
        }

        if base == 0 {
            if !c.is_digit() {
                break;
            }
        } else {
            let cp: CodePoint = c.to_point();
            let cp_i32 = cp.as_i32();
            let cp_u8 = u8::try_from(cp_i32)
                .expect("unable to convert digit character to u8 value");
            let dig = i32::from(utils::to_digit(cp_u8));

            if base <= dig {
                ctxt.UnrecognizedDigit = true;
            }
        }

        tokenizer_next_wl_character(session, token_start, policy);

        c = tokenizer_current_wl_character(session, token_start, policy);

        count += 1;
    } // while

    return (count, c);
}

fn tokenizer_handle_colon<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == ':');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char(':') => {
            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '[' {
                //
                // ::[
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session
                    .token(TokenKind::ColonColonOpenSquare, token_start);
            }

            //
            // ::
            //

            return session.token(TokenKind::ColonColon, token_start);
        },
        Char('=') => {
            //
            // :=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::ColonEqual, token_start);
        },
        Char('>') => {
            //
            // :>
            //

            incr_diagnostic!(Tokenizer_ColonGreaterCount);

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::ColonGreater, token_start);
        },
        _ => {
            //
            // :
            //

            return session.token(TokenKind::Colon, token_start);
        },
    }
}

fn tokenizer_handle_open_paren<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '(');

    let second_char = session.peek_source_char(policy);

    //
    // Comments must start literally with (*
    // Escaped characters do not work
    //
    if (c.to_point() == '(' && c.escape() == Escape::None) && second_char == '*'
    {
        //
        
        //
        return tokenizer_handle_comment(
            session,
            token_start,
            second_char,
            policy,
        );
    }

    //
    // (
    //

    incr_diagnostic!(Tokenizer_OpenParenCount);

    return session.token(TokenKind::OpenParen, token_start);
}

fn tokenizer_handle_dot<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    firstChar: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    let mut c = firstChar;

    //
    // handleDot
    // Could be  .  or  ..  or ...  or  .0
    //

    assert!(c.to_point() == '.');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.is_digit() {
        return tokenizer_handle_number(session, token_start, firstChar, policy);
    }

    if c.to_point() == '.' {
        tokenizer_next_wl_character(session, token_start, policy);

        c = tokenizer_current_wl_character(session, token_start, policy);

        if c.to_point() == '.' {
            //
            // ...
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::DotDotDot, token_start);
        }

        //
        // ..
        //

        return session.token(TokenKind::DotDot, token_start);
    }

    //
    // .
    //

    return session.token(TokenKind::Dot, token_start);
}

fn tokenizer_handle_equal<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '=');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('=') => {
            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '=' {
                //
                // ===
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::EqualEqualEqual, token_start);
            }

            //
            // ==
            //

            return session.token(TokenKind::EqualEqual, token_start);
        },
        Char('!') => {
            let bang_mark = session.mark();

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '=' {
                //
                // =!=
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::EqualBangEqual, token_start);
            }

            //
            // Something like  x=!y
            //
            // Must now do surgery and back up
            //

            tokenizer_backup_and_warn(session, bang_mark);

            return session.token(TokenKind::Equal, token_start);
        },
        _ => (),
    }

    //
    // =
    //

    return session.token(TokenKind::Equal, token_start);
}

fn tokenizer_handle_under<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '_');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('_') => {
            //
            // __
            //

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '_' {
                //
                // ___
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::UnderUnderUnder, token_start);
            }

            return session.token(TokenKind::UnderUnder, token_start);
        },
        Char('.') => {
            //
            // _.
            //

            tokenizer_next_wl_character(session, token_start, policy);

            if session.check_issues {
                let after_loc = session.src_loc;

                c = tokenizer_current_wl_character(session, token_start, policy);

                if c.to_point() == '.' {
                    //
                    // Something like  a_..b  or  _...
                    //
                    // Prior to 12.2,  a_..b  was parsed as Times[(a_).., b]
                    //
                    // 12.2 and onward,  a_..b  is parsed as Dot[a_., b]
                    //
                    // Related bugs: 390755
                    //

                    let dot_loc = after_loc;

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::insert_text(
                        "Insert space".into(),
                        Span::at(dot_loc),
                        " ".into(),
                    ));

                    let issue = syntax_issue(
                        IssueTag::UnexpectedDot,
                        "Suspicious syntax.".into(),
                        Severity::Error,
                        Span::at(dot_loc),
                        0.95,
                        actions,
                        vec![],
                    );

                    session.add_issue(issue);
                }
            }

            return session.token(TokenKind::UnderDot, token_start);
        },
        _ => (),
    }

    //
    // _
    //

    return session.token(TokenKind::Under, token_start);
}

fn tokenizer_handle_less<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '<');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('|') => {
            //
            // <|
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::LessBar, token_start);
        },
        Char('<') => {
            //
            // <<
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::LessLess, token_start);
        },
        Char('>') => {
            //
            // <>
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::LessGreater, token_start);
        },
        Char('=') => {
            //
            // <=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::LessEqual, token_start);
        },
        Char('-') => {
            let minus_mark = session.mark();

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '>' {
                //
                // <->
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::LessMinusGreater, token_start);
            }

            //
            // Something like  a<-4
            //
            // Must now do surgery and back up
            //

            tokenizer_backup_and_warn(session, minus_mark);

            return session.token(TokenKind::Less, token_start);
        },
        _ => (),
    }

    //
    // <
    //

    return session.token(TokenKind::Less, token_start);
}

fn tokenizer_handle_greater<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '>');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('>') => {
            //
            // >>
            //

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '>' {
                //
                // >>>
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session
                    .token(TokenKind::GreaterGreaterGreater, token_start);
            }

            return session.token(TokenKind::GreaterGreater, token_start);
        },
        Char('=') => {
            //
            // >=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::GreaterEqual, token_start);
        },
        _ => (),
    }

    //
    // >
    //

    return session.token(TokenKind::Greater, token_start);
}

fn tokenizer_handle_minus<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '-');

    c = tokenizer_current_wl_character(session, token_start, policy);

    //
    // Do not lex as a number here
    // Makes it easier to handle implicit times later
    //
    // Because if we lexed - as a number here, then it is
    // harder to know that b-1 is Plus[b, -1] instead of
    // b<invisiblespace>-1 which is Times[b, -1]
    //

    match c.to_point() {
        Char('>') => {
            //
            // ->
            //

            incr_diagnostic!(Tokenizer_MinusGreaterCount);

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::MinusGreater, token_start);
        },
        Char('-') => {
            //
            // --
            //

            tokenizer_next_wl_character(session, token_start, policy);

            if session.check_issues {
                let after_loc = session.src_loc;

                c = tokenizer_current_wl_character(session, token_start, policy);

                if c.to_point() == '>' {
                    //
                    // Something like  a-->0
                    //
                    // Was originally just a FormatIssue
                    //
                    // But a real-world example was demonstrated and this is now considered a real thing that could happen
                    //
                    // https://stash.wolfram.com/projects/WA/repos/alphasource/pull-requests/30963/overview
                    //

                    let greater_loc = after_loc;

                    let mut actions: Vec<CodeAction> = Vec::new();

                    //
                    // HACK: little bit of a hack here
                    // would like to replace  -->  with  ->
                    // but the current token is only -- and I would prefer to not read past the > just for this action
                    //
                    // So actually just replace the -- with -
                    //
                    actions.push(CodeAction::replace_text(
                        "Replace with ``->``".into(),
                        Span::new(token_start.loc, after_loc),
                        "-".into(),
                    ));

                    actions.push(CodeAction::insert_text(
                        "Insert space".into(),
                        Span::at(greater_loc),
                        " ".into(),
                    ));

                    let issue = syntax_issue(
                        IssueTag::Ambiguous,
                        "``-->`` is ambiguous syntax.".into(),
                        Severity::Error,
                        Span::new(token_start.loc, after_loc),
                        0.95,
                        actions,
                        vec![],
                    );

                    session.add_issue(issue);
                } else if c.to_point() == '=' {
                    //
                    // Something like  a--=0
                    //

                    let equal_loc = after_loc;

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::insert_text(
                        "Insert space".into(),
                        Span::at(equal_loc),
                        " ".into(),
                    ));

                    let issue = format_issue(
                        IssueTag::Ambiguous,
                        "Put a space between ``--`` and ``=`` to reduce ambiguity".into(),
                        Severity::Formatting,
                        Span::at(equal_loc),
                        1.0,
                        actions,
                        vec![],
                    );

                    session.add_issue(issue);
                }
            }

            return session.token(TokenKind::MinusMinus, token_start);
        },
        Char('=') => {
            //
            // -=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::MinusEqual, token_start);
        },
        _ => (),
    }

    //
    // -
    //

    incr_diagnostic!(Tokenizer_MinusCount);

    return session.token(TokenKind::Minus, token_start);
}

fn tokenizer_handle_bar<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '|');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('>') => {
            //
            // |>
            //

            tokenizer_next_wl_character(session, token_start, policy);

            if session.check_issues {
                let after_loc = session.src_loc;

                c = tokenizer_current_wl_character(session, token_start, policy);

                if c.to_point() == '=' {
                    //
                    // Something like  <||>=0
                    //

                    let equal_loc = after_loc;

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::insert_text(
                        "Insert space".into(),
                        Span::at(equal_loc),
                        " ".into(),
                    ));

                    let issue = format_issue(
                        IssueTag::Ambiguous,
                        "Put a space between ``|>`` and ``=`` to reduce ambiguity".into(),
                        Severity::Formatting,
                        Span::at(equal_loc),
                        1.0,
                        actions,
                        vec![],
                    );

                    session.add_issue(issue);
                }
            }

            return session.token(TokenKind::BarGreater, token_start);
        },
        Char('|') => {
            //
            // ||
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::BarBar, token_start);
        },
        Char('-') => {
            let bar_mark = session.mark();

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '>' {
                //
                // |->
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::BarMinusGreater, token_start);
            }

            //
            // Something like  x|-y
            //
            // Must now do surgery and back up
            //

            tokenizer_backup_and_warn(session, bar_mark);

            return session.token(TokenKind::Bar, token_start);
        },
        _ => (),
    }

    //
    // |
    //

    return session.token(TokenKind::Bar, token_start);
}

fn tokenizer_handle_semi<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == ';');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == ';' {
        //
        // ;;
        //

        tokenizer_next_wl_character(session, token_start, policy);

        return session.token(TokenKind::SemiSemi, token_start);
    }

    //
    // ;
    //

    return session.token(TokenKind::Semi, token_start);
}

fn tokenizer_handle_bang<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '!');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('=') => {
            //
            // !=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::BangEqual, token_start);
        },
        Char('!') => {
            //
            // !!
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::BangBang, token_start);
        },
        _ => (),
    }

    //
    // !
    //

    return session.token(TokenKind::Bang, token_start);
}

fn tokenizer_handle_hash<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '#');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == '#' {
        //
        // ##
        //

        tokenizer_next_wl_character(session, token_start, policy);

        return session.token(TokenKind::HashHash, token_start);
    }

    //
    // #
    //

    incr_diagnostic!(Tokenizer_HashCount);

    return session.token(TokenKind::Hash, token_start);
}

fn tokenizer_handle_percent<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '%');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == '%' {
        //
        // %%
        //

        c = tokenizer_current_wl_character(session, token_start, policy);

        loop {
            if c.to_point() != '%' {
                break;
            }

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);
        } // while

        return session.token(TokenKind::PercentPercent, token_start);
    }

    //
    // %
    //

    return session.token(TokenKind::Percent, token_start);
}

fn tokenizer_handle_amp<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '&');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == '&' {
        //
        // &&
        //

        tokenizer_next_wl_character(session, token_start, policy);

        return session.token(TokenKind::AmpAmp, token_start);
    }

    //
    // &
    //

    incr_diagnostic!(Tokenizer_AmpCount);

    return session.token(TokenKind::Amp, token_start);
}

fn tokenizer_handle_slash<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '/');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('@') => {
            //
            // /@
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::SlashAt, token_start);
        },
        Char(';') => {
            //
            // /;
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::SlashSemi, token_start);
        },
        Char('.') => {
            let dot_mark = session.mark();

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if !c.is_digit() {
                //
                // /.
                //

                return session.token(TokenKind::SlashDot, token_start);
            }

            //
            // Something like  t/.3
            //
            // Must now do surgery and back up
            //

            tokenizer_backup_and_warn(session, dot_mark);

            return session.token(TokenKind::Slash, token_start);
        },
        Char('/') => {
            //
            // //
            //

            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            match c.to_point() {
                Char('.') => {
                    //
                    // //.
                    //

                    tokenizer_next_wl_character(session, token_start, policy);

                    return session
                        .token(TokenKind::SlashSlashDot, token_start);
                },
                Char('@') => {
                    //
                    // //@
                    //

                    tokenizer_next_wl_character(session, token_start, policy);

                    return session.token(TokenKind::SlashSlashAt, token_start);
                },
                Char('=') => {
                    //
                    // //=
                    //

                    tokenizer_next_wl_character(session, token_start, policy);

                    return session
                        .token(TokenKind::SlashSlashEqual, token_start);
                },
                _ => (),
            }

            //
            // //
            //

            return session.token(TokenKind::SlashSlash, token_start);
        },
        Char(':') => {
            //
            // /:
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::SlashColon, token_start);
        },
        Char('=') => {
            //
            // /=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::SlashEqual, token_start);
        },
        Char('*') => {
            //
            // /*
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::SlashStar, token_start);
        },
        _ => (),
    }

    //
    // /
    //

    return session.token(TokenKind::Slash, token_start);
}

fn tokenizer_handle_at<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '@');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('@') => {
            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '@' {
                //
                // @@@
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::AtAtAt, token_start);
            }

            //
            // @@
            //

            return session.token(TokenKind::AtAt, token_start);
        },
        Char('*') => {
            //
            // @*
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::AtStar, token_start);
        },
        _ => (),
    }

    //
    // @
    //

    return session.token(TokenKind::At, token_start);
}

fn tokenizer_handle_plus<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '+');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('+') => {
            //
            // ++
            //

            tokenizer_next_wl_character(session, token_start, policy);

            if session.check_issues {
                c = tokenizer_current_wl_character(session, token_start, policy);

                if c.to_point() == '=' {
                    //
                    // Something like  a++=0
                    //

                    let loc = session.src_loc;

                    let mut actions: Vec<CodeAction> = Vec::new();

                    actions.push(CodeAction::insert_text(
                        "Insert space".into(),
                        Span::at(loc),
                        " ".into(),
                    ));

                    let issue = format_issue(
                        IssueTag::Ambiguous,
                        "Put a space between ``++`` and ``=`` to reduce ambiguity".into(),
                        Severity::Formatting,
                        Span::at(loc),
                        1.0,
                        actions,
                        vec![],
                    );

                    session.add_issue(issue);
                }
            }

            return session.token(TokenKind::PlusPlus, token_start);
        },
        Char('=') => {
            //
            // +=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::PlusEqual, token_start);
        },
        _ => (),
    }

    //
    // +
    //

    incr_diagnostic!(Tokenizer_PlusCount);

    return session.token(TokenKind::Plus, token_start);
}

fn tokenizer_handle_tilde<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '~');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == '~' {
        //
        // ~~
        //

        tokenizer_next_wl_character(session, token_start, policy);

        return session.token(TokenKind::TildeTilde, token_start);
    }

    //
    // ~
    //

    return session.token(TokenKind::Tilde, token_start);
}

fn tokenizer_handle_question<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '?');

    c = tokenizer_current_wl_character(session, token_start, policy);

    if c.to_point() == '?' {
        //
        // ??
        //

        tokenizer_next_wl_character(session, token_start, policy);

        return session.token(TokenKind::QuestionQuestion, token_start);
    }

    //
    // ?
    //

    return session.token(TokenKind::Question, token_start);
}

fn tokenizer_handle_star<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '*');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('=') => {
            //
            // *=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::StarEqual, token_start);
        },
        Char('*') => {
            //
            // **
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::StarStar, token_start);
        },
        Char(')') => {
            //
            // *)
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session
                .token(TokenKind::Error_UnexpectedCommentCloser, token_start);
        },
        _ => (),
    }

    //
    // *
    //

    return session.token(TokenKind::Star, token_start);
}

fn tokenizer_handle_caret<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.to_point() == '^');

    c = tokenizer_current_wl_character(session, token_start, policy);

    match c.to_point() {
        Char(':') => {
            tokenizer_next_wl_character(session, token_start, policy);

            c = tokenizer_current_wl_character(session, token_start, policy);

            if c.to_point() == '=' {
                //
                // ^:=
                //

                tokenizer_next_wl_character(session, token_start, policy);

                return session.token(TokenKind::CaretColonEqual, token_start);
            }

            //
            // Has to be ^:=
            //

            return session.token(TokenKind::Error_ExpectedEqual, token_start);
        },
        Char('=') => {
            //
            // ^=
            //

            tokenizer_next_wl_character(session, token_start, policy);

            return session.token(TokenKind::CaretEqual, token_start);
        },
        _ => (),
    }

    //
    // ^
    //

    return session.token(TokenKind::Caret, token_start);
}

fn tokenizer_handle_unhandled_backslash<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    mut c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    //
    // Unhandled \
    //
    // Something like  \A  or  \{  or  \<EOF>
    //
    // If the bad character looks like a special input, then try to reconstruct the character up to the bad SourceCharacter
    // This duplicates some logic in CharacterDecoder
    //

    assert!(c.to_point() == '\\');

    c = tokenizer_next_wl_character(session, token_start, policy);

    match c.to_point() {
        Char('[') => {
            //
            // Try to reconstruct \[XXX]
            //

            let mut reset_mark = session.mark();

            c = tokenizer_next_wl_character(session, token_start, policy);

            let mut well_formed = false;

            if c.is_upper() {
                reset_mark = session.mark();

                c = tokenizer_next_wl_character(session, token_start, policy);

                loop {
                    if c.is_alpha_or_digit() {
                        reset_mark = session.mark();

                        c = tokenizer_next_wl_character(
                            session,
                            token_start,
                            policy,
                        );

                        continue;
                    }

                    if c.to_point() == ']' {
                        well_formed = true;
                    } else {
                        session.seek(reset_mark);
                    }

                    break;
                }
            }

            if well_formed {
                return session
                    .token(TokenKind::Error_UnhandledCharacter, token_start);
            }

            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        Char(':') => {
            //
            // Try to reconstruct \:XXXX
            //

            let mut reset_mark = session.mark();

            c = tokenizer_next_wl_character(session, token_start, policy);

            for _ in 0..4 {
                if c.is_hex() {
                    reset_mark = session.mark();

                    c = tokenizer_next_wl_character(session, token_start, policy);

                    continue;
                }

                session.seek(reset_mark);

                break;
            }

            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        Char('.') => {
            //
            // Try to reconstruct \.XX
            //

            let mut reset_mark = session.mark();

            c = tokenizer_next_wl_character(session, token_start, policy);

            for _ in 0..2 {
                if c.is_hex() {
                    reset_mark = session.mark();

                    c = tokenizer_next_wl_character(session, token_start, policy);

                    continue;
                }

                session.seek(reset_mark);

                break;
            }

            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        Char('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7') => {
            //
            // Try to reconstruct \XXX
            //

            let mut reset_mark = session.mark();

            c = tokenizer_next_wl_character(session, token_start, policy);

            for _ in 0..3 {
                if c.is_octal() {
                    reset_mark = session.mark();

                    c = tokenizer_next_wl_character(session, token_start, policy);

                    continue;
                }

                session.seek(reset_mark);

                break;
            }

            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        Char('|') => {
            //
            // Try to reconstruct \|XXXXXX
            //

            let mut reset_mark = session.mark();
            c = tokenizer_next_wl_character(session, token_start, policy);

            for _ in 0..6 {
                if c.is_hex() {
                    reset_mark = session.mark();

                    c = tokenizer_next_wl_character(session, token_start, policy);

                    continue;
                }

                session.seek(reset_mark);

                break;
            }

            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        EndOfFile => {
            return session
                .token(TokenKind::Error_UnhandledCharacter, token_start);
        },
        _ => (),
    } // switch

    //
    // Nothing special, just read next single character
    //

    return session.token(TokenKind::Error_UnhandledCharacter, token_start);
}

fn tokenizer_handle_mb_strange_newline<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_mb_strange_newline());

    if session.check_issues {
        add_unexpected_char_issue(
            session,
            c,
            token_start.loc,
            IssueTag::UnexpectedNewlineCharacter,
            0.85,
        )
    }

    //
    // Return INTERNALNEWLINE or TOPLEVELNEWLINE, depending on policy
    //
    return session.token(TokenKind::newline_with_policy(policy), token_start);
}

fn tokenizer_handle_mb_strange_whitespace<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_mb_strange_whitespace());

    if session.check_issues {
        add_unexpected_char_issue(
            session,
            c,
            token_start.loc,
            IssueTag::UnexpectedSpaceCharacter,
            0.85,
        )
    }

    return session.token(TokenKind::Whitespace, token_start);
}

fn tokenizer_handle_mb_punctuation<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_mb_punctuation());

    let char = c
        .to_point()
        .as_char()
        .expect("expected MBPunctuation to be char");

    let operator =
        crate::generated::long_names::LongNameCodePointToOperator(
            char,
        );

    return session.token(operator, token_start);
}

fn tokenizer_handle_naked_mb_linear_syntax<'i>(
    session: &mut Tokenizer<'i>,
    token_start: &TokenStart<'i>,
    c: WLCharacter,
    _policy: NextPolicy,
) -> TokenRef<'i> {
    assert!(c.is_mb_linear_syntax());

    match c.to_point() {
        Char(CODEPOINT_LINEARSYNTAX_CLOSEPAREN) => {
            return session
                .token(TokenKind::LinearSyntax_CloseParen, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_AT) => {
            return session.token(TokenKind::LinearSyntax_At, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_PERCENT) => {
            return session.token(TokenKind::LinearSyntax_Percent, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_CARET) => {
            return session.token(TokenKind::LinearSyntax_Caret, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_AMP) => {
            return session.token(TokenKind::LinearSyntax_Amp, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_STAR) => {
            return session.token(TokenKind::LinearSyntax_Star, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_UNDER) => {
            return session.token(TokenKind::LinearSyntax_Under, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_PLUS) => {
            return session.token(TokenKind::LinearSyntax_Plus, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_SLASH) => {
            return session.token(TokenKind::LinearSyntax_Slash, token_start);
        },
        Char(CODEPOINT_LINEARSYNTAX_BACKTICK) => {
            return session
                .token(TokenKind::LinearSyntax_BackTick, token_start);
        },
        CodePoint::LinearSyntax_Space => {
            return session.token(TokenKind::LinearSyntax_Space, token_start);
        },
        _ => todo!(),
    }
}

//==========================================================
// Helper functions
//==========================================================

/// Add an [`Issue`][crate::issue::Issue] for an unexpected character.
fn add_unexpected_char_issue(
    session: &mut Tokenizer,
    c: WLCharacter,
    char_loc: Location,
    tag: IssueTag,
    confidence: f64,
) {
    let src = session.get_token_span(char_loc);

    let actions: Vec<CodeAction> =
        utils::certain_character_replacement_actions(c, src);

    let formatted = match tag {
        IssueTag::UnexpectedNewlineCharacter => c.graphical_string(),
        _ => c.safe_and_graphical_string(),
    };

    let char_type = match tag {
        IssueTag::UnexpectedSpaceCharacter => "space",
        IssueTag::UnexpectedLetterlikeCharacter => "letterlike",
        IssueTag::UnexpectedNewlineCharacter => "newline",
        _ => panic!("unhandled unexpected char IssueTag: {tag:?}"),
    };

    let issue = syntax_issue(
        tag,
        format!("Unexpected {char_type} character: ``{}``.", formatted),
        Severity::Warning,
        src,
        confidence,
        actions,
        vec![],
    );

    session.add_issue(issue);
}
