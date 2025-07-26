//! Unified error handling for the Wolfram Language parser
//!
//! This module provides comprehensive error handling with zero-cost abstractions
//! for safe parsing operations.

use thiserror::Error;
use crate::source::Span;

/// Main error type for the parser
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Tokenizer error: {0}")]
    Tokenizer(#[from] TokenizerError),
    
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),
    
    #[error("Encoding error: {0}")]
    Encoding(#[from] EncodingError),
    
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Tokenizer-specific errors
#[derive(Error, Debug)]
pub enum TokenizerError {
    #[error("Invalid character '{char}' at {position:?}")]
    InvalidCharacter { position: Span, char: char },
    
    #[error("Unterminated string starting at {start:?}")]
    UnterminatedString { start: Span },
    
    #[error("Invalid number at {span:?}: {reason}")]
    InvalidNumber { span: Span, reason: String },
    
    #[error("Invalid escape sequence at {position:?}")]
    InvalidEscape { position: Span },
    
    #[error("Missing quote offset for string literal")]
    MissingQuoteOffset,
    
    #[error("Missing caret buffer for base number")]
    MissingCaretBuffer,
    
    #[error("Missing sign mark for number")]
    MissingSignMark,
    
    #[error("Buffer overflow: {context}")]
    BufferOverflow { context: String },
}

/// Parser-specific errors
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Unexpected token: expected {expected:?}, found {found:?}")]
    UnexpectedToken { expected: String, found: String },
    
    #[error("Missing closing {expected} for opener at {opener:?}")]
    MissingCloser { opener: Span, expected: &'static str },
    
    #[error("Empty context stack during {operation}")]
    EmptyContextStack { operation: &'static str },
    
    #[error("Invalid parser state: {description}")]
    InvalidState { description: String },
    
    #[error("Stack overflow: parser context stack exceeded maximum depth")]
    StackOverflow,
}

/// Internal/assertion errors
#[derive(Error, Debug)]
pub enum InternalError {
    #[error("Numeric overflow in {context}: {details}")]
    NumericOverflow { context: String, details: String },
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
    
    #[error("Index out of bounds: {index} >= {len} in {context}")]
    IndexOutOfBounds { index: usize, len: usize, context: String },
}

/// Encoding-related errors
#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Invalid UTF-8 sequence at byte {position}")]
    InvalidUtf8 { position: usize },
    
    #[error("Unsupported encoding: {encoding}")]
    UnsupportedEncoding { encoding: String },
    
    #[error("Character decode error: {details}")]
    DecodeError { details: String },
}


/// Safe error handling macros
/// 
/// These macros provide consistent error handling behavior throughout the parser.

/// Safe unwrap that returns an error instead of panicking
#[macro_export]
macro_rules! safe_unwrap {
    ($expr:expr, $err:expr) => {
        $expr.ok_or($err)?
    };
}

/// Safe expect that provides better error messages
#[macro_export]
macro_rules! safe_expect {
    ($expr:expr, $msg:literal) => {
        match $expr {
            Some(val) => val,
            None => {
                $crate::error_handling::perf_monitor::increment_error();
                panic!("Assertion failed: {}", $msg)
            }
        }
    };
}

/// Safe index access with bounds checking
#[macro_export]
macro_rules! safe_index {
    ($slice:expr, $index:expr, $context:literal) => {
        if $index >= $slice.len() {
            return Err($crate::error_handling::InternalError::IndexOutOfBounds {
                index: $index,
                len: $slice.len(),
                context: $context.to_string(),
            }.into());
        }
        &$slice[$index]
    };
}

/// Safe numeric conversion
#[macro_export]
macro_rules! safe_convert {
    ($value:expr, $target_type:ty, $context:literal) => {
        match <$target_type>::try_from($value) {
            Ok(val) => val,
            Err(e) => {
                $crate::error_handling::perf_monitor::increment_error();
                panic!("Numeric overflow during {}: converting {} to {}: {}", 
                    $context, stringify!($value), stringify!($target_type), e)
            }
        }
    };
}

/// Context for parser operations
pub trait ErrorContext<T> {
    fn with_context(self, msg: &str) -> Result<T, ParseError>;
    fn with_span(self, span: Span) -> Result<T, ParseError>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<ParseError>,
{
    fn with_context(self, msg: &str) -> Result<T, ParseError> {
        self.map_err(|e| {
            ParseError::Internal(InternalError::InvalidState(
                format!("{}: {}", msg, Into::<ParseError>::into(e))
            ))
        })
    }
    
    fn with_span(self, _span: Span) -> Result<T, ParseError> {
        // TODO: Enhance error with span information
        self.map_err(|e| e.into())
    }
}

/// Safe utilities for common operations
pub mod safe_utils {
    use super::InternalError;

    /// Safe usize to u32 conversion
    pub fn safe_usize_to_u32(value: usize) -> Result<u32, InternalError> {
        u32::try_from(value)
            .map_err(|_| InternalError::NumericOverflow {
                context: "usize_to_u32".to_string(),
                details: format!("value {} exceeds u32::MAX", value),
            })
    }
    
    /// Safe u32 to usize conversion  
    pub fn safe_u32_to_usize(value: u32) -> Result<usize, InternalError> {
        usize::try_from(value)
            .map_err(|_| InternalError::NumericOverflow {
                context: "u32_to_usize".to_string(),
                details: format!("value {} cannot fit in usize", value),
            })
    }
}

/// Performance monitoring for error handling
pub mod perf_monitor {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static ERROR_CHECK_COUNT: AtomicUsize = AtomicUsize::new(0);
    static ERROR_OCCURRED_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    pub fn increment_check() {
        ERROR_CHECK_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_error() {
        ERROR_OCCURRED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats() -> (usize, usize) {
        (
            ERROR_CHECK_COUNT.load(Ordering::Relaxed),
            ERROR_OCCURRED_COUNT.load(Ordering::Relaxed),
        )
    }
    
    pub fn reset_stats() {
        ERROR_CHECK_COUNT.store(0, Ordering::Relaxed);
        ERROR_OCCURRED_COUNT.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safe_conversion() {
        let result = safe_utils::safe_usize_to_u32(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
    
    #[test]
    fn test_safe_conversion_overflow() {
        let large_value = u64::MAX as usize;
        let result = safe_utils::safe_usize_to_u32(large_value);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InternalError::NumericOverflow { .. }));
    }
    
    #[test]
    fn test_performance_monitoring() {
        perf_monitor::reset_stats();
        perf_monitor::increment_check();
        perf_monitor::increment_error();
        
        let (checks, errors) = perf_monitor::get_stats();
        assert_eq!(checks, 1);
        assert_eq!(errors, 1);
    }
}