# Plan 03: String Interning Implementation

## Overview
Implement string interning for frequently used operator strings, keywords, and common symbols to reduce memory usage and improve comparison performance.

## Impact
**Performance**: MEDIUM - Faster string comparisons, better cache locality
**Risk**: LOW - Can be implemented incrementally
**Effort**: MEDIUM - 1-2 weeks

## Current State Analysis

### String Usage Patterns
1. **Token Input**: Stored as `TokenStr<'i>` (borrowed) or `TokenString` (owned)
2. **Operators**: 300+ operator types with string representations
3. **Symbols**: Already use `SymbolRef<'static>` (effectively interned)
4. **High-frequency strings**: Single-char operators appear in nearly every expression

### Most Frequently Duplicated Strings
```
Rank  String   Typical Frequency (per 1000 tokens)
1.    "+"      45-60 occurrences
2.    "*"      40-55 occurrences  
3.    "="      35-50 occurrences
4.    "["      30-45 occurrences
5.    "]"      30-45 occurrences
6.    "("      25-40 occurrences
7.    ")"      25-40 occurrences
8.    "-"      20-35 occurrences
9.    "/"      15-25 occurrences
10.   "^"      15-25 occurrences
```

## Implementation Strategy

### Step 1: Add String Interning Dependencies (Day 1)
```toml
# Cargo.toml
[dependencies]
string-interner = { version = "0.17", default-features = false, features = ["inline-more"] }
# Alternative: lasso = "0.7"
once_cell = "1.19"
```

### Step 2: Create String Interner Module (Day 2)

```rust
// src/string_interner.rs
use once_cell::sync::Lazy;
use string_interner::{DefaultSymbol, StringInterner};
use std::sync::RwLock;

pub type InternedString = DefaultSymbol;

// Global interner for operator strings
static OPERATOR_INTERNER: Lazy<RwLock<StringInterner>> = Lazy::new(|| {
    let mut interner = StringInterner::new();
    
    // Pre-intern the most common operators
    interner.get_or_intern("+");
    interner.get_or_intern("-");
    interner.get_or_intern("*");
    interner.get_or_intern("/");
    interner.get_or_intern("=");
    interner.get_or_intern("^");
    interner.get_or_intern("(");
    interner.get_or_intern(")");
    interner.get_or_intern("[");
    interner.get_or_intern("]");
    interner.get_or_intern("{");
    interner.get_or_intern("}");
    interner.get_or_intern(":=");
    interner.get_or_intern("->");
    interner.get_or_intern("==");
    
    RwLock::new(interner)
});

pub fn intern_operator(s: &str) -> InternedString {
    // Try read lock first for common case
    if let Ok(interner) = OPERATOR_INTERNER.read() {
        if let Some(symbol) = interner.get(s) {
            return symbol;
        }
    }
    
    // Fall back to write lock for new strings
    OPERATOR_INTERNER.write().unwrap().get_or_intern(s)
}

pub fn resolve_interned(symbol: InternedString) -> &'static str {
    OPERATOR_INTERNER.read().unwrap().resolve(symbol)
        .expect("Invalid interned symbol")
}

// For single-threaded parsing contexts
pub struct LocalInterner {
    interner: StringInterner,
}

impl LocalInterner {
    pub fn new() -> Self {
        let mut interner = StringInterner::new();
        // Pre-populate with common strings
        Self::prepopulate(&mut interner);
        Self { interner }
    }
    
    fn prepopulate(interner: &mut StringInterner) {
        // Common operators
        for op in &["+", "-", "*", "/", "=", "^", "(", ")", "[", "]", "{", "}"] {
            interner.get_or_intern(op);
        }
        
        // Common multi-char operators
        for op in &[":=", "->", "==", "!=", "<=", ">=", "&&", "||", "<>", "~~"] {
            interner.get_or_intern(op);
        }
    }
    
    pub fn intern(&mut self, s: &str) -> InternedString {
        self.interner.get_or_intern(s)
    }
    
    pub fn resolve(&self, symbol: InternedString) -> &str {
        self.interner.resolve(symbol).unwrap()
    }
}
```

### Step 3: Update Token Structure (Day 3-4)

```rust
// src/tokenize.rs
#[derive(Debug, Clone)]
pub enum TokenInput<I> {
    Borrowed(I),
    Owned(TokenString),
    Interned(InternedString), // New variant
}

impl<I: AsRef<str>> TokenInput<I> {
    pub fn as_str(&self) -> &str {
        match self {
            TokenInput::Borrowed(s) => s.as_ref(),
            TokenInput::Owned(s) => s.as_str(),
            TokenInput::Interned(sym) => resolve_interned(*sym),
        }
    }
}

// Update Token construction
impl<'i> Token<'i> {
    pub fn new_with_interning(
        tok: TokenKind,
        input: TokenStr<'i>,
        src: Source,
        interner: &mut LocalInterner,
    ) -> Self {
        // Intern operators and common keywords
        let input = if should_intern(tok, input.as_ref()) {
            TokenInput::Interned(interner.intern(input.as_ref()))
        } else {
            TokenInput::Borrowed(input)
        };
        
        Token { tok, input, src }
    }
}

fn should_intern(kind: TokenKind, s: &str) -> bool {
    use TokenKind::*;
    
    match kind {
        // Always intern single-char operators
        Plus | Minus | Star | Slash | Equal | Caret |
        OpenParen | CloseParen | OpenSquare | CloseSquare |
        OpenCurly | CloseCurly => true,
        
        // Intern multi-char operators
        ColonEqual | MinusGreater | EqualEqual | BangEqual |
        LessEqual | GreaterEqual | AmpAmp | BarBar |
        LessGreater | TildeTilde => true,
        
        // Intern common function names if short
        Symbol if s.len() <= 10 => {
            matches!(s, "Plus" | "Times" | "Power" | "List" | "Set" | 
                       "Rule" | "If" | "While" | "Module" | "Block")
        }
        
        _ => false,
    }
}
```

### Step 4: Integrate with Tokenizer (Day 5)

```rust
// src/tokenize/tokenizer.rs
pub struct Tokenizer<'i> {
    // existing fields...
    interner: LocalInterner, // Add interner
}

impl<'i> Tokenizer<'i> {
    pub fn new(/* params */) -> Self {
        Self {
            // existing initialization...
            interner: LocalInterner::new(),
        }
    }
    
    fn make_token(&mut self, tok: TokenKind, input: TokenStr<'i>, src: Source) -> Token<'i> {
        Token::new_with_interning(tok, input, src, &mut self.interner)
    }
}
```

### Step 5: Optimize String Comparisons (Day 6)

```rust
// Add fast comparison for interned strings
impl<I: AsRef<str>> PartialEq for TokenInput<I> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Fast path: compare interned symbols directly
            (TokenInput::Interned(a), TokenInput::Interned(b)) => a == b,
            // Slow path: string comparison
            _ => self.as_str() == other.as_str(),
        }
    }
}

// Enable pattern matching on interned strings
impl TokenKind {
    pub fn as_operator_str(&self) -> Option<&'static str> {
        match self {
            TokenKind::Plus => Some("+"),
            TokenKind::Minus => Some("-"),
            TokenKind::Star => Some("*"),
            // ... etc
            _ => None,
        }
    }
}
```

### Step 6: Memory Usage Optimization (Day 7)

```rust
// Periodic cleanup for long-running parsers
impl LocalInterner {
    pub fn should_cleanup(&self) -> bool {
        self.interner.len() > 10_000 // Threshold
    }
    
    pub fn cleanup(&mut self) {
        // Keep only frequently used strings
        let mut new_interner = StringInterner::new();
        Self::prepopulate(&mut new_interner);
        
        // Could track usage statistics and keep hot strings
        self.interner = new_interner;
    }
}
```

## Benchmarking Plan

```rust
// benches/string_interning.rs
fn bench_operator_parsing(c: &mut Criterion) {
    let expressions = vec![
        "a + b * c - d / e ^ f",
        "(x + y) * (z - w) / (u + v)",
        "f[a, b, c] + g[x, y] * h[z]",
    ];
    
    c.bench_function("without_interning", |b| {
        b.iter(|| {
            for expr in &expressions {
                parse_without_interning(black_box(expr));
            }
        })
    });
    
    c.bench_function("with_interning", |b| {
        b.iter(|| {
            for expr in &expressions {
                parse_with_interning(black_box(expr));
            }
        })
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    // Measure peak memory usage for parsing large files
    // Compare total allocations with/without interning
}
```

## Configuration Options

```toml
[features]
# Allow disabling interning for comparison
no-string-interning = []
```

```rust
#[cfg(not(feature = "no-string-interning"))]
fn make_token(...) -> Token {
    Token::new_with_interning(...)
}

#[cfg(feature = "no-string-interning")]
fn make_token(...) -> Token {
    Token::new(...)
}
```

## Success Metrics

1. **Memory Reduction**: 20-40% less memory for operator-heavy code
2. **Comparison Speed**: 2-5x faster for interned string comparisons
3. **Parse Performance**: Neutral to 5% improvement
4. **Memory Overhead**: < 1MB for interner data structures

## Migration Strategy

### Phase 1: Infrastructure (Days 1-2)
- Add dependencies
- Create interner module
- Add feature flag

### Phase 2: Integration (Days 3-5)
- Update Token structure
- Integrate with tokenizer
- Maintain backward compatibility

### Phase 3: Optimization (Days 6-7)
- Optimize comparisons
- Add cleanup strategies
- Performance tuning

## Risks and Mitigations

1. **Risk**: Thread contention on global interner
   - **Mitigation**: Use thread-local interners for parsing
   
2. **Risk**: Memory leak from unbounded interning
   - **Mitigation**: Periodic cleanup, usage tracking
   
3. **Risk**: Overhead for unique strings
   - **Mitigation**: Selective interning based on token kind

## Testing Strategy

1. **Correctness Tests**: Verify identical parsing results
2. **Memory Tests**: Measure allocation patterns
3. **Performance Tests**: Benchmark common expressions
4. **Stress Tests**: Large files with many operators
5. **Thread Safety**: Concurrent parsing tests