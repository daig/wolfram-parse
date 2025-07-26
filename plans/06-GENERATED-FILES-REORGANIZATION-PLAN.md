# Plan 06: Generated Files Reorganization

## Overview
Split the monolithic 308KB `long_names_registration.rs` into logical modules for better maintainability, faster compilation, and clearer organization.

## Impact
**Performance**: POSITIVE - Faster incremental compilation
**Risk**: LOW - Pure refactoring with re-exports
**Effort**: LOW - 3-4 days

## Current State

### File Sizes
- `long_names_registration.rs`: 4,538 lines, 308KB
- `precedence_values.rs`: 193 lines, 20KB

### Content Breakdown
```
long_names_registration.rs:
├── Character Constants (1,100 lines)
├── Lookup Maps (2,200 lines)
├── Character Classes (850 lines)
└── Operator Function (300 lines)
```

## Proposed Structure

```
src/generated/
├── mod.rs                      # Module root
├── precedence/
│   └── mod.rs                  # Precedence values (unchanged)
└── long_names/
    ├── mod.rs                  # Module organization
    ├── constants.rs            # Character constants
    ├── mappings.rs             # Lookup tables
    ├── character_classes.rs    # Classification arrays
    └── operators.rs            # Operator mapping
```

## Implementation Plan

### Step 1: Create Module Structure (Day 1)

```bash
# Create directory structure
mkdir -p src/generated/long_names
mkdir -p src/generated/precedence
```

```rust
// src/generated/mod.rs
//! Auto-generated data modules for the Wolfram Language parser
//! 
//! This module contains Unicode character mappings, operator precedence
//! values, and other generated lookup tables.

pub mod long_names;
pub mod precedence;

// Convenience re-exports for backward compatibility
pub use long_names::{
    longname_to_codepoint,
    codepoint_to_longname,
    LongNameCodePointToOperator,
    LONGNAME_TO_CODEPOINT_MAP,
    CODEPOINT_TO_LONGNAME_MAP,
};

pub use precedence::*;
```

```rust
// src/generated/long_names/mod.rs
//! Unicode long name character data and mappings

mod constants;
mod mappings;
mod character_classes;
mod operators;

// Re-export all public items
pub use constants::*;
pub use mappings::*;
pub use character_classes::*;
pub use operators::*;
```

### Step 2: Split Character Constants (Day 1)

```rust
// src/generated/long_names/constants.rs
//! Unicode character constants for Wolfram Language long names
//! 
//! Generated from CodeParser/Data/LongNames.wl

use crate::cst::CodePoint;

// Raw characters
pub const CODEPOINT_LONGNAME_RAWTAB: CodePoint = CodePoint(0x0009);
pub const CODEPOINT_LONGNAME_RAWLINEFEED: CodePoint = CodePoint(0x000A);
// ... (~1,100 more constants)

// Group by category for better organization:
// Mathematical operators
pub const CODEPOINT_LONGNAME_PLUS: CodePoint = CodePoint(0x002B);
pub const CODEPOINT_LONGNAME_MINUS: CodePoint = CodePoint(0x2212);
// ...

// Greek letters
pub const CODEPOINT_LONGNAME_ALPHA: CodePoint = CodePoint(0x03B1);
pub const CODEPOINT_LONGNAME_BETA: CodePoint = CodePoint(0x03B2);
// ...
```

### Step 3: Extract Lookup Tables (Day 2)

```rust
// src/generated/long_names/mappings.rs
//! Bidirectional mappings between long names and code points

use crate::cst::CodePoint;

/// Map from long name strings to their Unicode code points
pub const LONGNAME_TO_CODEPOINT_MAP: [(&str, CodePoint); 1102] = [
    ("AAcute", CodePoint(0x00C1)),
    ("ABar", CodePoint(0x0100)),
    // ... 1,100 more entries
];

/// Map from code points to their long name strings  
pub const CODEPOINT_TO_LONGNAME_MAP: [(CodePoint, &str); 1102] = [
    (CodePoint(0x00C1), "AAcute"),
    (CodePoint(0x0100), "ABar"),
    // ... 1,100 more entries
];

/// Characters that should be treated as raw
pub const RAW_SET: [CodePoint; 37] = [
    CodePoint(0x0009), // Tab
    CodePoint(0x000A), // LF
    // ... 35 more entries
];
```

### Step 4: Move Character Classification (Day 2)

```rust
// src/generated/long_names/character_classes.rs
//! Character classification arrays for lexical analysis

use crate::cst::CodePoint;

/// Letter-like characters that are not strange
pub const MB_NOT_STRAGE_LETTERLIKE_CODE_POINTS: [CodePoint; 296] = [
    CodePoint(0x00AA), // ª
    CodePoint(0x00BA), // º
    // ... 294 more entries
];

/// Punctuation characters
pub const MB_PUNCTUATION_CODE_POINTS: [CodePoint; 301] = [
    CodePoint(0x00A1), // ¡
    CodePoint(0x00A7), // §
    // ... 299 more entries
];

/// Whitespace characters
pub const MB_WHITESPACE_CODE_POINTS: [CodePoint; 20] = [
    CodePoint(0x0009), // Tab
    CodePoint(0x0020), // Space
    // ... 18 more entries
];

/// Newline characters
pub const MB_NEWLINE_CODE_POINTS: [CodePoint; 6] = [
    CodePoint(0x000A), // LF
    CodePoint(0x000D), // CR
    // ... 4 more entries
];

/// Uninterpretable characters
pub const MB_UNINTERPRETABLE_CODE_POINTS: [CodePoint; 4] = [
    CodePoint(0xFFF9), // Interlinear annotation anchor
    // ... 3 more entries
];

/// ASCII character replacements
pub const ASCII_REPLACEMENTS_MAP: &[(char, &[&str])] = &[
    (' ', &["\\[RawSpace]"]),
    ('!', &["\\[RawExclamation]"]),
    // ... more entries
];
```

### Step 5: Extract Operator Mapping (Day 3)

```rust
// src/generated/long_names/operators.rs
//! Mapping from Unicode characters to operator tokens

use crate::tokenize::TokenKind;
use super::constants::*;

/// Convert a long name code point to its corresponding operator token
pub fn LongNameCodePointToOperator(c: char) -> TokenKind {
    use TokenKind::*;
    
    match c {
        '\u{00AC}' => LongName_Not,           // ¬
        '\u{00B1}' => LongName_PlusMinus,     // ±
        '\u{00D7}' => LongName_Times,         // ×
        '\u{00F7}' => LongName_Divide,        // ÷
        // ... ~300 more cases
        _ => LongName_Unhandled,
    }
}
```

### Step 6: Update Imports (Day 3)

```rust
// Update src/lib.rs
pub(crate) mod generated;

// Update files that import from generated
// src/long_names.rs
use crate::generated::long_names::{
    LONGNAME_TO_CODEPOINT_MAP,
    CODEPOINT_TO_LONGNAME_MAP,
    // ... other imports
};
```

### Step 7: Move Precedence File (Day 4)

```bash
# Rename and move
mv src/generated/precedence_values.rs src/generated/precedence/mod.rs
```

```rust
// src/generated/precedence/mod.rs
//! Operator precedence values for the Wolfram Language parser

use crate::parse::precedence::Precedence;

pub const PRECEDENCE_LOWEST: Precedence = Precedence::new(0);
pub const PRECEDENCE_COMMA: Precedence = Precedence::new(10);
// ... rest of the file unchanged
```

### Step 8: Testing and Validation (Day 4)

```bash
# Run all tests to ensure nothing broke
cargo test

# Check that compilation still works
cargo build --release

# Verify benchmarks still pass
cargo bench
```

## Migration Benefits

### Compilation Performance
- **Before**: Any change to generated code recompiles 308KB file
- **After**: Changes only recompile affected module (50-100KB)
- **Parallel**: Different modules can compile simultaneously

### Code Organization
```
Before: 1 large file → Hard to navigate
After: 5 focused files → Clear purpose for each
```

### Development Experience
- Better IDE performance (smaller files)
- Easier to find specific constants or mappings
- Clear separation between different data types

## Backward Compatibility

All existing code continues to work unchanged:
```rust
// Old import still works
use crate::generated::long_names_registration::LONGNAME_TO_CODEPOINT_MAP;

// New specific import also available
use crate::generated::long_names::mappings::LONGNAME_TO_CODEPOINT_MAP;
```

## Generator Script Updates

Update the generation scripts to output the new structure:
```rust
// In the generator
fn generate_constants(out_dir: &Path) {
    let constants_path = out_dir.join("long_names/constants.rs");
    // Generate constants file
}

fn generate_mappings(out_dir: &Path) {
    let mappings_path = out_dir.join("long_names/mappings.rs");
    // Generate mappings file
}
// etc.
```

## Success Metrics

1. **No functionality changes** - All tests pass
2. **Faster incremental builds** - Measure before/after
3. **Better organization** - Logical file boundaries
4. **Maintained performance** - No runtime impact

## Risks and Mitigations

1. **Risk**: Import path changes
   - **Mitigation**: Comprehensive re-exports maintain compatibility
   
2. **Risk**: Generator script complexity
   - **Mitigation**: Incremental updates, test thoroughly
   
3. **Risk**: Missing exports
   - **Mitigation**: Run full test suite, check all imports