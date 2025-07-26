# Plan 07: Deprecated Patterns Cleanup

## Overview
Clean up outdated Rust patterns to align with modern idiomatic Rust practices (2021 edition).

## Impact
**Performance**: NEUTRAL - Mostly syntactic changes
**Risk**: LOW - No functional changes
**Effort**: LOW - 2-3 days

## Identified Patterns to Clean Up

### 1. Unnecessary `ref` in Pattern Matching (9 occurrences)

#### Current Usage
```rust
// src/abstract_cst.rs
Some(ref input) => ...     // line 627
Some(ref children) => ...  // line 2019
Some(ref head) => ...      // line 2850
```

#### Modern Pattern
```rust
// Rust now automatically borrows in patterns
Some(input) => ...
Some(children) => ...
Some(head) => ...
```

### 2. Inline Module Declaration

#### Current
```rust
// src/lib.rs lines 95-98
pub(crate) mod generated {
    pub(crate) mod long_names_registration;
    pub(crate) mod precedence_values;
}
```

#### Modern Pattern
```rust
// src/generated/mod.rs
pub(crate) mod long_names_registration;
pub(crate) mod precedence_values;

// src/lib.rs
pub(crate) mod generated;
```

### 3. Manual Default Implementation

#### Current
```rust
// src/quirks.rs lines 128-132
impl Default for QuirkSettings {
    fn default() -> Self {
        Self::const_default()
    }
}
```

#### Analysis
This is actually intentional for const initialization, but could be documented better:
```rust
impl Default for QuirkSettings {
    /// Provides runtime default via const default for consistency
    fn default() -> Self {
        Self::const_default()
    }
}
```

### 4. Verbose Option/Result Handling

#### Current Patterns Found
```rust
// Common verbose patterns
match opt {
    Some(x) => Some(f(x)),
    None => None,
}

match result {
    Ok(x) => Ok(g(x)),
    Err(e) => Err(e),
}
```

#### Modern Patterns
```rust
// Use combinators
opt.map(f)
result.map(g)
opt.and_then(|x| Some(f(x)))
result.map_err(|e| CustomError::from(e))
```

### 5. Missing Feature Declarations

#### Current Issue
```toml
# Missing in Cargo.toml but used in code
# DIAGNOSTICS feature used 80 times
# USE_MATHLINK feature used 1 time
```

#### Fix
```toml
[features]
default = ["COMPUTE_SOURCE"]
COMPUTE_SOURCE = []
FAST_STRING_SCAN = []
DIAGNOSTICS = []      # Add this
USE_MATHLINK = []     # Add this
```

## Implementation Plan

### Step 1: Remove Unnecessary `ref` Keywords (Day 1 Morning)

```bash
# Create script to find and fix ref patterns
rg "Some\(ref \w+\)" --type rust -A 2 -B 2
```

Manual fixes in `src/abstract_cst.rs`:
```rust
// Line 627
- Some(ref input) => input.abstract_cast(),
+ Some(input) => input.abstract_cast(),

// Line 2019
- Some(ref children) => {
+ Some(children) => {

// Lines 2850-2851
- Some(ref head) => head.abstract_cst(),
- Some(ref body) => body.abstract_cst(),
+ Some(head) => head.abstract_cst(),
+ Some(body) => body.abstract_cst(),

// Lines 3220-3222
- Some(ref first) => first.has_dir(),
- Some(ref second) => second.has_dir(),
- Some(ref rest) => rest.has_dir(),
+ Some(first) => first.has_dir(),
+ Some(second) => second.has_dir(),
+ Some(rest) => rest.has_dir(),
```

### Step 2: Fix Feature Declarations (Day 1 Afternoon)

```toml
# Cargo.toml
[features]
default = ["COMPUTE_SOURCE"]
COMPUTE_SOURCE = []
FAST_STRING_SCAN = []
DIAGNOSTICS = []          # For diagnostic counters
USE_MATHLINK = []         # For MathLink integration
```

### Step 3: Module Organization (Day 1 Afternoon)

```bash
# Create proper module file
mkdir -p src/generated
echo "pub(crate) mod long_names_registration;" > src/generated/mod.rs
echo "pub(crate) mod precedence_values;" >> src/generated/mod.rs
```

Update `src/lib.rs`:
```rust
// Remove lines 95-98 and replace with:
pub(crate) mod generated;
```

### Step 4: Identify and Fix Verbose Patterns (Day 2)

Create a lint script to find common patterns:
```rust
// find_verbose_patterns.rs
use regex::Regex;

fn find_verbose_match_patterns(content: &str) -> Vec<LineMatch> {
    // Pattern for match Some(x) => Some(...)
    let pattern = Regex::new(r"match\s+\w+\s*\{\s*Some\(\w+\)\s*=>\s*Some\(").unwrap();
    // ... find and report
}
```

Common fixes:
```rust
// Before
match self.node_stack.last() {
    Some(node) => Some(node.clone()),
    None => None,
}

// After
self.node_stack.last().cloned()

// Before
match parse_result {
    Ok(ast) => Ok(process(ast)),
    Err(e) => Err(e),
}

// After  
parse_result.map(process)
```

### Step 5: Documentation Updates (Day 2)

Add clarifying comments where patterns are intentional:
```rust
// src/quirks.rs
impl Default for QuirkSettings {
    /// Uses const_default to ensure compile-time initialization
    /// This pattern is intentional for const contexts
    fn default() -> Self {
        Self::const_default()
    }
}
```

### Step 6: Automated Cleanup (Day 3)

Use `cargo clippy` with additional lints:
```toml
# .clippy.toml
warn = [
    "clippy::redundant_pattern_matching",
    "clippy::manual_map",
    "clippy::match_like_matches_macro",
    "clippy::needless_match",
]
```

Run cleanup:
```bash
cargo clippy --fix --allow-dirty --allow-staged
cargo fmt
```

### Step 7: Final Review and Testing (Day 3)

```bash
# Ensure no functionality changed
cargo test

# Check for any remaining patterns
rg "Some\(ref " --type rust
rg "match.*Some.*=>.*Some" --type rust

# Run lints
cargo clippy -- -W clippy::pedantic
```

## Specific Pattern Fixes

### For Loops (Keep as-is)
The `for _ in 0..n` patterns found are actually idiomatic for repeating an action n times:
```rust
// This is correct - no change needed
for _ in 0..3 {
    result.push(self.next_token());
}
```

### Manual Range Checks (Keep as-is)
The interval containment check is clear and correct:
```rust
// This is clear and efficient - no change needed
b_start >= a_start && b_end <= a_end
```

## Success Metrics

1. **Zero `ref` keywords** in pattern matching (except where necessary)
2. **All features declared** in Cargo.toml
3. **No clippy warnings** for addressed patterns
4. **All tests pass** without modification
5. **Cleaner code** that follows Rust 2021 idioms

## Testing Strategy

```rust
#[test]
fn test_pattern_matching_without_ref() {
    // Ensure borrowed values work correctly
    let data = Some(String::from("test"));
    match &data {
        Some(s) => assert_eq!(s, "test"),
        None => panic!(),
    }
    // data is still valid here
    assert!(data.is_some());
}
```

## Documentation

Update CONTRIBUTING.md or style guide:
```markdown
## Code Style Guidelines

### Pattern Matching
- Avoid `ref` in patterns unless necessary
- Use `&` on the matched expression instead

### Option/Result Handling  
- Prefer combinators (`map`, `and_then`) over manual matching
- Use `?` operator for error propagation

### Module Organization
- Place module declarations in separate `mod.rs` files
- Avoid inline module declarations in parent modules
```

## Risks and Mitigations

1. **Risk**: Breaking working code
   - **Mitigation**: Comprehensive test suite run after each change
   
2. **Risk**: Missing some patterns
   - **Mitigation**: Use automated tools (ripgrep, clippy) to find patterns
   
3. **Risk**: Style inconsistency
   - **Mitigation**: Run `cargo fmt` after all changes