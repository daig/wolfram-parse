# Plan 05: API Ergonomics Improvements

## Overview
Improve API usability through newtypes, trait implementations, builder patterns, and consistent design patterns.

## Impact
**Performance**: NEUTRAL - Zero-cost abstractions
**Risk**: LOW - Mostly additive changes
**Effort**: MEDIUM - 1-2 weeks

## Current Issues and Solutions

### 1. Type Safety with Newtypes (Day 1-2)

#### Current Problems
- Raw numeric types lack semantic meaning
- Easy to mix up parameters of same type
- No compile-time validation

#### Implementation
```rust
// src/newtypes.rs
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabWidth(NonZeroU32);

impl TabWidth {
    pub fn new(width: u32) -> Option<Self> {
        NonZeroU32::new(width).map(TabWidth)
    }
    
    pub fn default() -> Self {
        TabWidth(NonZeroU32::new(4).unwrap())
    }
    
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ConfidenceLevel(f64);

impl ConfidenceLevel {
    pub fn new(level: f64) -> Option<Self> {
        if (0.0..=1.0).contains(&level) {
            Some(ConfidenceLevel(level))
        } else {
            None
        }
    }
    
    pub fn certain() -> Self {
        ConfidenceLevel(1.0)
    }
    
    pub fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineNumber(NonZeroU32);

impl LineNumber {
    pub fn new(line: u32) -> Option<Self> {
        NonZeroU32::new(line).map(LineNumber)
    }
    
    pub fn first() -> Self {
        LineNumber(NonZeroU32::new(1).unwrap())
    }
}

// Update ParseOptions
impl ParseOptions {
    pub fn tab_width(mut self, width: TabWidth) -> Self {
        self.tab_width = width.get();
        self
    }
}
```

### 2. Standard Trait Implementations (Day 3)

```rust
// src/node_seq.rs
use std::ops::{Deref, DerefMut, Index, IndexMut};

impl<N> Deref for NodeSeq<N> {
    type Target = Vec<N>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N> DerefMut for NodeSeq<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<N> IntoIterator for NodeSeq<N> {
    type Item = N;
    type IntoIter = std::vec::IntoIter<N>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, N> IntoIterator for &'a NodeSeq<N> {
    type Item = &'a N;
    type IntoIter = std::slice::Iter<'a, N>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<N> FromIterator<N> for NodeSeq<N> {
    fn from_iter<I: IntoIterator<Item = N>>(iter: I) -> Self {
        NodeSeq(iter.into_iter().collect())
    }
}

impl<N> Extend<N> for NodeSeq<N> {
    fn extend<I: IntoIterator<Item = N>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<N> Index<usize> for NodeSeq<N> {
    type Output = N;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

// Add Debug implementations
impl fmt::Debug for ParseOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseOptions")
            .field("tab_width", &self.tab_width)
            .field("first_line_behavior", &self.first_line_behavior)
            .field("encoding", &self.encoding)
            .field("convention", &self.convention)
            .finish()
    }
}

// Add Display for enums
impl fmt::Display for FirstLineBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotScript => write!(f, "not-script"),
            Self::Check => write!(f, "check"),
            Self::Script => write!(f, "script"),
        }
    }
}
```

### 3. Enhanced Builder Pattern (Day 4-5)

```rust
// src/parse_options.rs
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Debug, Clone)]
#[builder(doc)]
pub struct ParseOptions {
    #[builder(default = TabWidth::default())]
    tab_width: TabWidth,
    
    #[builder(default = FirstLineBehavior::NotScript)]
    first_line_behavior: FirstLineBehavior,
    
    #[builder(default = EncodingMode::Normal)]
    encoding: EncodingMode,
    
    #[builder(default = SourceConvention::LineColumn)]
    convention: SourceConvention,
    
    #[builder(default = QuirkSettings::default())]
    quirks: QuirkSettings,
}

// Usage:
let options = ParseOptions::builder()
    .tab_width(TabWidth::new(8).unwrap())
    .encoding(EncodingMode::Strict)
    .build();

// QuirkSettings builder
#[derive(TypedBuilder, Debug, Clone)]
pub struct QuirkSettings {
    #[builder(default = false)]
    flatten_times: bool,
    
    #[builder(default = false)]
    infix_binary_at: bool,
    
    #[builder(default = false)]
    infix_binary_pipe: bool,
}
```

### 4. ParseResult Ergonomics (Day 6)

```rust
// src/parse_result.rs
impl<T> ParseResult<T> {
    /// Check if parsing succeeded without any errors
    pub fn is_ok(&self) -> bool {
        self.fatal_issues.is_empty()
    }
    
    /// Check if there are any issues (fatal or non-fatal)
    pub fn has_issues(&self) -> bool {
        !self.fatal_issues.is_empty() || !self.non_fatal_issues.is_empty()
    }
    
    /// Check if there are warnings (non-fatal issues)
    pub fn has_warnings(&self) -> bool {
        !self.non_fatal_issues.is_empty()
    }
    
    /// Get all issues (fatal and non-fatal)
    pub fn issues(&self) -> impl Iterator<Item = &Issue> {
        self.fatal_issues.iter().chain(&self.non_fatal_issues)
    }
    
    /// Convert to Result, failing if there are fatal issues
    pub fn into_result(self) -> Result<T, Vec<Issue>> {
        if self.fatal_issues.is_empty() {
            Ok(self.syntax)
        } else {
            Err(self.fatal_issues)
        }
    }
    
    /// Get the syntax tree, regardless of issues
    pub fn syntax(&self) -> &T {
        &self.syntax
    }
    
    /// Take the syntax tree, consuming the ParseResult
    pub fn into_syntax(self) -> T {
        self.syntax
    }
}

// Add From trait for easy error handling
impl<T: Default> From<Vec<Issue>> for ParseResult<T> {
    fn from(issues: Vec<Issue>) -> Self {
        ParseResult {
            syntax: T::default(),
            fatal_issues: issues,
            non_fatal_issues: vec![],
            unsafe_character_encoding: None,
        }
    }
}
```

### 5. Consistent Error Handling (Day 7)

```rust
// Replace panicking functions with Result variants
impl NodeSeq<Token<TokenStr>> {
    /// Try to get a single token, returning an error if not exactly one
    pub fn try_single(self) -> Result<Token<TokenStr>, NodeSeqError> {
        match self.0.len() {
            0 => Err(NodeSeqError::Empty),
            1 => Ok(self.0.into_iter().next().unwrap()),
            n => Err(NodeSeqError::Multiple(n)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeSeqError {
    #[error("Expected single item, but sequence was empty")]
    Empty,
    
    #[error("Expected single item, but found {0} items")]
    Multiple(usize),
}

// Consistent API for all parsing functions
pub fn tokenize(input: &str, opts: &ParseOptions) -> Result<ParseResult<NodeSeq<Token<TokenStr>>>, EncodingError> {
    let bytes = input.as_bytes();
    tokenize_bytes(bytes, opts)
}
```

### 6. Source Location Improvements (Day 8)

```rust
// src/source.rs
impl Location {
    /// Create a location at the start of a file
    pub fn start() -> Self {
        Location { line: 1, column: 1 }
    }
    
    /// Advance by a number of characters
    pub fn advance(&mut self, chars: usize) {
        self.column += chars;
    }
    
    /// Move to next line
    pub fn next_line(&mut self) {
        self.line += 1;
        self.column = 1;
    }
}

impl Add<usize> for Location {
    type Output = Location;
    
    fn add(mut self, chars: usize) -> Self::Output {
        self.column += chars;
        self
    }
}

impl Span {
    /// Create a span at a single point
    pub fn point(loc: Location) -> Self {
        Span::at(loc)
    }
    
    /// Create a span between two locations
    pub fn between(start: Location, end: Location) -> Self {
        Span { start, end }
    }
    
    /// Check if this span contains a location
    pub fn contains(&self, loc: Location) -> bool {
        self.start <= loc && loc <= self.end
    }
}
```

### 7. Type Aliases (Day 9)

```rust
// src/lib.rs - Public type aliases
pub type TokenSeq<'i> = NodeSeq<Token<TokenStr<'i>>>;
pub type CstResult<'i> = ParseResult<Cst<TokenStr<'i>>>;
pub type CstSeqResult<'i> = ParseResult<CstSeq<TokenStr<'i>>>;
pub type AstResult = ParseResult<Ast>;
pub type AstSeqResult = ParseResult<NodeSeq<Ast>>;

// Internal aliases
type ParserResult<T> = Result<T, ParseError>;
```

### 8. Documentation and Examples (Day 10)

```rust
/// Parse Wolfram Language code with custom options
/// 
/// # Examples
/// 
/// ```
/// use wolfram_parser::{parse_cst, ParseOptions, TabWidth};
/// 
/// let code = "f[x_, y_] := x + y";
/// let options = ParseOptions::builder()
///     .tab_width(TabWidth::new(2).unwrap())
///     .build();
///     
/// let result = parse_cst(code, &options);
/// 
/// if result.is_ok() {
///     println!("Parsed successfully: {:?}", result.syntax());
/// } else {
///     for issue in result.issues() {
///         eprintln!("Issue: {}", issue);
///     }
/// }
/// ```
pub fn parse_cst(input: &str, opts: &ParseOptions) -> CstResult<'_> {
    // ...
}
```

## Migration Strategy

### Phase 1: Add New APIs (Days 1-5)
- Implement newtypes
- Add trait implementations
- Create builders

### Phase 2: Deprecate Old APIs (Days 6-8)
- Mark old methods as deprecated
- Provide migration guide
- Update examples

### Phase 3: Documentation (Days 9-10)
- Add comprehensive examples
- Update API documentation
- Create migration guide

## Success Metrics

1. **Type Safety**: No more raw numeric types in public API
2. **Discoverability**: All types implement expected traits
3. **Consistency**: Uniform error handling patterns
4. **Documentation**: Examples for all major functions
5. **Zero Breaking Changes**: All improvements are additive

## Testing Strategy

```rust
#[test]
fn test_newtype_validation() {
    assert!(TabWidth::new(0).is_none());
    assert!(TabWidth::new(8).is_some());
    
    assert!(ConfidenceLevel::new(-0.1).is_none());
    assert!(ConfidenceLevel::new(0.5).is_some());
    assert!(ConfidenceLevel::new(1.1).is_none());
}

#[test]
fn test_builder_ergonomics() {
    let opts = ParseOptions::builder()
        .tab_width(TabWidth::new(2).unwrap())
        .build();
    
    assert_eq!(opts.tab_width().get(), 2);
}

#[test]
fn test_nodeseq_traits() {
    let seq: NodeSeq<i32> = vec![1, 2, 3].into_iter().collect();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[1], 2);
    
    let doubled: NodeSeq<i32> = seq.iter().map(|x| x * 2).collect();
    assert_eq!(doubled[0], 2);
}
```