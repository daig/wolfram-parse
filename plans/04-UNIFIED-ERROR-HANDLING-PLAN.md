# Plan 04: Unified Error Handling Implementation

## Overview
Replace `unwrap()` calls with proper error handling and create a unified error type hierarchy for better error propagation and user experience.

## Impact
**Performance**: MINIMAL - Small overhead for error checking
**Risk**: MEDIUM - Requires careful refactoring of core paths  
**Effort**: HIGH - 2-3 weeks

## Current State Analysis

### Critical `unwrap()` Usage (by severity)

#### CRITICAL - Parser State (10 occurrences)
```rust
// src/parse.rs
self.context_stack.pop().unwrap()      // Lines 1254-1505
self.context_stack.last_mut().unwrap() // Line 1538
```

#### HIGH - Tokenizer State (15 occurrences)
```rust
// src/tokenize/tokenizer.rs
quot_offset.unwrap()    // String parsing
caret1Buf.unwrap()      // Base parsing
sign_mark.unwrap()      // Number parsing
```

#### MEDIUM - Type Conversions (20+ occurrences)
```rust
usize::try_from(...).unwrap()
u32::try_from(...).unwrap()
```

#### LOW - Test Code (50+ occurrences)
Test code can use `expect()` with descriptive messages.

## Implementation Plan

### Step 1: Define Error Type Hierarchy (Day 1-2)

```rust
// src/error.rs (refactored)
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum TokenizerError {
    #[error("Invalid character '{char}' at {position}")]
    InvalidCharacter { position: Span, char: char },
    
    #[error("Unterminated string starting at {start}")]
    UnterminatedString { start: Span },
    
    #[error("Invalid number at {span}: {reason}")]
    InvalidNumber { span: Span, reason: String },
    
    #[error("Invalid escape sequence at {position}")]
    InvalidEscape { position: Span },
    
    #[error("Missing quote offset for string literal")]
    MissingQuoteOffset,
    
    #[error("Missing caret buffer for base number")]
    MissingCaretBuffer,
    
    #[error("Missing sign mark for number")]
    MissingSignMark,
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Unexpected token: expected {expected:?}, found {found:?}")]
    UnexpectedToken { expected: TokenKind, found: Token },
    
    #[error("Missing closing {expected} for {opener}")]
    MissingCloser { opener: Span, expected: &'static str },
    
    #[error("Empty context stack during {operation}")]
    EmptyContextStack { operation: &'static str },
    
    #[error("Invalid parser state: {description}")]
    InvalidState { description: String },
}

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("Numeric overflow in {context}")]
    NumericOverflow { context: String },
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
}

// Extension trait for better error context
pub trait ErrorContext<T> {
    fn context(self, msg: &str) -> Result<T, ParseError>;
    fn with_span(self, span: Span) -> Result<T, ParseError>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<ParseError>,
{
    fn context(self, msg: &str) -> Result<T, ParseError> {
        self.map_err(|e| ParseError::Internal(
            InternalError::InvalidState(format!("{}: {}", msg, e.into()))
        ))
    }
    
    fn with_span(self, span: Span) -> Result<T, ParseError> {
        // Add span information to error
        self.map_err(|e| {
            let mut err = e.into();
            // Attach span to error (implementation detail)
            err
        })
    }
}
```

### Step 2: Fix Critical Parser Context Stack (Day 3-4)

```rust
// src/parse.rs - Replace unwrap() with proper error handling

impl<'i, B: ParseBuilder<'i>> Parser<'i, B> {
    fn pop_context(&mut self) -> Result<ContextData, ParserError> {
        self.context_stack.pop()
            .ok_or(ParserError::EmptyContextStack {
                operation: "pop_context"
            })
    }
    
    fn current_context_mut(&mut self) -> Result<&mut Context, ParserError> {
        self.context_stack.last_mut()
            .ok_or(ParserError::EmptyContextStack {
                operation: "current_context_mut"
            })
    }
    
    // Update all call sites
    fn parse_contexted(&mut self) -> Result<B::Node, ParseError> {
        // Before:
        // let ctx = self.context_stack.pop().unwrap();
        
        // After:
        let ctx = self.pop_context()?;
        
        // Process with error propagation
        let result = self.process_context(ctx)?;
        Ok(result)
    }
}
```

### Step 3: Fix Tokenizer State Management (Day 5-6)

```rust
// src/tokenize/tokenizer.rs

impl<'i> Tokenizer<'i> {
    // Replace optional unwraps with proper error handling
    fn handle_string_quote(&mut self) -> Result<Token<'i>, TokenizerError> {
        let offset = self.quot_offset
            .ok_or(TokenizerError::MissingQuoteOffset)?;
        
        // Continue processing...
        Ok(token)
    }
    
    fn handle_base_number(&mut self) -> Result<Token<'i>, TokenizerError> {
        let caret_buf = self.caret1Buf
            .ok_or(TokenizerError::MissingCaretBuffer)?;
        
        // Continue processing...
        Ok(token)
    }
    
    fn handle_number_sign(&mut self) -> Result<Token<'i>, TokenizerError> {
        let sign = self.sign_mark
            .ok_or(TokenizerError::MissingSignMark)?;
        
        // Continue processing...
        Ok(token)
    }
}
```

### Step 4: Safe Numeric Conversions (Day 7)

```rust
// src/utils.rs - Add safe conversion utilities

pub fn safe_usize_from_u32(value: u32) -> Result<usize, InternalError> {
    usize::try_from(value)
        .map_err(|_| InternalError::NumericOverflow {
            context: format!("converting u32 {} to usize", value)
        })
}

pub fn safe_u32_from_usize(value: usize) -> Result<u32, InternalError> {
    u32::try_from(value)
        .map_err(|_| InternalError::NumericOverflow {
            context: format!("converting usize {} to u32", value)
        })
}

// Use throughout codebase
// Before:
let width = u32::try_from(tab_width).unwrap();

// After:
let width = safe_u32_from_usize(tab_width)?;
```

### Step 5: Update ParseResult Integration (Day 8-9)

```rust
// Enhance ParseResult to work with new error types
impl<T> ParseResult<T> {
    pub fn from_error(error: ParseError) -> Self {
        let issue = Issue::from_parse_error(error);
        ParseResult {
            syntax: T::default(), // Or appropriate empty value
            fatal_issues: vec![issue],
            non_fatal_issues: vec![],
            unsafe_character_encoding: None,
        }
    }
    
    pub fn add_error(&mut self, error: ParseError) {
        let issue = Issue::from_parse_error(error);
        if error.is_fatal() {
            self.fatal_issues.push(issue);
        } else {
            self.non_fatal_issues.push(issue);
        }
    }
}

impl Issue {
    pub fn from_parse_error(error: ParseError) -> Self {
        match error {
            ParseError::Parser(e) => Issue {
                severity: IssueSeverity::Error,
                message: e.to_string(),
                // ... map other fields
            },
            // ... handle other error types
        }
    }
}
```

### Step 6: Create Migration Utilities (Day 10)

```rust
// Temporary utilities to ease migration
macro_rules! try_unwrap {
    ($expr:expr, $err:expr) => {
        $expr.ok_or_else(|| $err)?
    };
}

macro_rules! safe_expect {
    ($expr:expr, $msg:literal) => {
        $expr.ok_or_else(|| InternalError::AssertionFailed($msg.to_string()))?
    };
}

// For incremental migration
#[allow(unused)]
fn legacy_unwrap<T>(opt: Option<T>, context: &str) -> T {
    opt.unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            panic!("Legacy unwrap failed: {}", context);
        } else {
            // Log error and return default in production
            eprintln!("WARNING: Legacy unwrap failed: {}", context);
            std::process::exit(1);
        }
    })
}
```

### Step 7: Update Public API (Day 11-12)

```rust
// Update public functions to handle errors gracefully
pub fn parse_cst_lines(input: &[u8], opts: &ParseOptions) -> ParseResult<CstSeq> {
    match parse_cst_lines_internal(input, opts) {
        Ok(result) => result,
        Err(e) => ParseResult::from_error(e),
    }
}

// Internal version that uses Result
fn parse_cst_lines_internal(
    input: &[u8], 
    opts: &ParseOptions
) -> Result<ParseResult<CstSeq>, ParseError> {
    // Implementation with proper error propagation
}
```

## Migration Strategy

### Phase 1: Infrastructure (Days 1-2)
- Define error types
- Add dependencies (thiserror)
- Create conversion utilities

### Phase 2: Critical Paths (Days 3-6)
- Fix parser context stack
- Fix tokenizer state
- Ensure no panics in core parsing

### Phase 3: Type Safety (Days 7-9)
- Safe numeric conversions
- Integration with existing types
- Helper macros

### Phase 4: API Updates (Days 10-12)
- Update public functions
- Maintain compatibility
- Documentation

## Testing Strategy

```rust
#[cfg(test)]
mod error_tests {
    #[test]
    fn test_context_stack_error() {
        let mut parser = Parser::new();
        // Artificially empty the stack
        parser.context_stack.clear();
        
        let result = parser.pop_context();
        assert!(matches!(
            result,
            Err(ParserError::EmptyContextStack { .. })
        ));
    }
    
    #[test]
    fn test_numeric_overflow() {
        let large_usize = usize::MAX;
        let result = safe_u32_from_usize(large_usize);
        assert!(matches!(
            result,
            Err(InternalError::NumericOverflow { .. })
        ));
    }
    
    #[test]
    fn test_error_propagation() {
        // Test that errors propagate correctly through the stack
    }
}
```

## Success Metrics

1. **Zero `unwrap()` in production code** (except where mathematically proven safe)
2. **All errors have descriptive messages**
3. **No performance regression** (< 2% overhead)
4. **Improved debuggability** with error context

## Rollback Plan

1. Keep legacy functions during migration
2. Feature flag for gradual rollout
3. Comprehensive test coverage before switching

## Risks and Mitigations

1. **Risk**: Performance overhead from error checking
   - **Mitigation**: Profile hot paths, optimize only where needed
   
2. **Risk**: API compatibility
   - **Mitigation**: Maintain existing public API, internal refactoring
   
3. **Risk**: Missing error cases
   - **Mitigation**: Comprehensive testing, fuzzing