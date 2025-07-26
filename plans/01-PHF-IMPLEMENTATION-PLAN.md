# Plan 01: Perfect Hash Function (PHF) Implementation

## Overview
Replace binary search operations in lookup tables with compile-time perfect hash functions for O(1) lookups.

## Impact
**Performance**: HIGH - 5-10x speedup for character lookups
**Risk**: MEDIUM - Requires careful migration of generated code
**Effort**: MEDIUM - 2-3 weeks

## Current State Analysis

### Binary Search Usage (11 locations)
1. **longname_to_codepoint()** - 1,102 entries, hot path
2. **codepoint_to_longname()** - 1,102 entries  
3. **isMBPunctuation()** - 301 entries, hot path
4. **isMBNotStrangeLetterlike()** - 296 entries
5. **asciiReplacements()** - variable entries
6. **contains_raw()** - 37 entries
7. **isMBWhitespace()** - 20 entries
8. **isMBNewline()** - 6 entries
9. **isMBUninterpretable()** - 4 entries

### Match Statement (1 location)
10. **LongNameCodePointToOperator()** - 300+ arms

## Implementation Steps

### Step 1: Infrastructure Setup (2 days)
```toml
# Cargo.toml
[dependencies]
phf = { version = "0.11", features = ["macros"] }

[build-dependencies]
phf_codegen = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Create `build.rs`:
```rust
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    generate_phf_maps();
}
```

### Step 2: Create PHF Generation Script (3 days)

Create `src/generated/phf_generator.rs`:
```rust
// Parse existing arrays and generate PHF maps
pub fn generate_longname_map(out_path: &Path) {
    let mut file = BufWriter::new(File::create(out_path).unwrap());
    
    writeln!(&mut file, "use phf::{{phf_map, phf_set}};").unwrap();
    writeln!(&mut file, "use crate::CodePoint;").unwrap();
    
    // Generate LONGNAME_TO_CODEPOINT_PHF
    writeln!(&mut file, "pub static LONGNAME_TO_CODEPOINT_PHF: phf::Map<&'static str, CodePoint> = phf_map! {{").unwrap();
    
    // Parse existing LONGNAME_TO_CODEPOINT_MAP and generate entries
    for (name, codepoint) in parse_existing_map() {
        writeln!(&mut file, r#"    "{}" => CodePoint({}),"#, name, codepoint.0).unwrap();
    }
    
    writeln!(&mut file, "}};").unwrap();
}
```

### Step 3: Migrate High-Impact Maps (1 week)

#### 3.1 LONGNAME_TO_CODEPOINT_MAP
```rust
// Before (src/long_names.rs)
pub(crate) fn longname_to_codepoint(s: &str) -> Option<CodePoint> {
    let index = LONGNAME_TO_CODEPOINT_MAP
        .binary_search_by(|(name, _)| name.cmp(&s))
        .ok()?;
    Some(LONGNAME_TO_CODEPOINT_MAP[index].1)
}

// After
pub(crate) fn longname_to_codepoint(s: &str) -> Option<CodePoint> {
    LONGNAME_TO_CODEPOINT_PHF.get(s).copied()
}
```

#### 3.2 MB_PUNCTUATION_CODE_POINTS
```rust
// Before
pub(crate) fn isMBPunctuation(c: CodePoint) -> bool {
    MB_PUNCTUATION_CODE_POINTS
        .binary_search(&c)
        .is_ok()
}

// After
static MB_PUNCTUATION_PHF: phf::Set<u32> = phf_set! {
    0x00A1, 0x00A7, 0x00AB, // ... etc
};

pub(crate) fn isMBPunctuation(c: CodePoint) -> bool {
    MB_PUNCTUATION_PHF.contains(&c.as_u32())
}
```

### Step 4: Convert LongNameCodePointToOperator (3 days)

```rust
// Before: 300+ line match statement
// After:
static CODEPOINT_TO_OPERATOR_PHF: phf::Map<u32, TokenKind> = phf_map! {
    0x00AC => TokenKind::LongName_Not,
    0x00B1 => TokenKind::LongName_PlusMinus,
    // ... generated from existing match arms
};

pub(crate) fn LongNameCodePointToOperator(c: char) -> TokenKind {
    CODEPOINT_TO_OPERATOR_PHF
        .get(&(c as u32))
        .copied()
        .unwrap_or(TokenKind::Unknown)
}
```

### Step 5: Feature Flag for A/B Testing (2 days)

```toml
[features]
use_phf = []
```

```rust
#[cfg(feature = "use_phf")]
pub(crate) fn longname_to_codepoint(s: &str) -> Option<CodePoint> {
    LONGNAME_TO_CODEPOINT_PHF.get(s).copied()
}

#[cfg(not(feature = "use_phf"))]
pub(crate) fn longname_to_codepoint(s: &str) -> Option<CodePoint> {
    // Original binary search implementation
}
```

### Step 6: Benchmarking Suite (2 days)

Create `benches/phf_comparison.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_longname_lookup(c: &mut Criterion) {
    let test_names = vec!["Alpha", "Beta", "Gamma", "RightArrow", "Integral"];
    
    c.bench_function("binary_search_lookup", |b| {
        b.iter(|| {
            for name in &test_names {
                black_box(longname_to_codepoint_binary(name));
            }
        })
    });
    
    c.bench_function("phf_lookup", |b| {
        b.iter(|| {
            for name in &test_names {
                black_box(longname_to_codepoint_phf(name));
            }
        })
    });
}
```

## Migration Order (by impact)

1. **Week 1**: Infrastructure + LONGNAME_TO_CODEPOINT_MAP
2. **Week 2**: MB_PUNCTUATION_CODE_POINTS + LongNameCodePointToOperator
3. **Week 3**: Remaining maps + benchmarking + cleanup

## Success Metrics

1. **Performance**: 5-10x speedup in character lookup benchmarks
2. **Binary Size**: Less than 10% increase in compiled size
3. **Build Time**: Less than 20% increase in compilation time
4. **Test Coverage**: All existing tests pass without modification

## Rollback Plan

1. Feature flag allows instant rollback
2. Keep original arrays until PHF proven stable
3. Can selectively enable PHF for specific maps

## Dependencies

- No breaking changes to public API
- Requires updating build process
- May need to coordinate with upstream Wolfram code generation

## Testing Strategy

1. **Unit Tests**: Verify identical behavior for all lookups
2. **Property Tests**: Random lookup comparison between implementations
3. **Benchmark**: Compare performance on real Wolfram files
4. **Integration**: Full test suite with PHF enabled

## Code Generation Integration

The existing Wolfram Language scripts that generate the Rust code need modification:
1. Generate JSON intermediate format
2. Rust build.rs reads JSON and generates PHF maps
3. Maintain backward compatibility during transition

## Risks and Mitigations

1. **Risk**: CodePoint complexity for hashing
   - **Mitigation**: Use u32 representation for hashing
   
2. **Risk**: Build complexity increase
   - **Mitigation**: Clear documentation and examples
   
3. **Risk**: Generated code drift from upstream
   - **Mitigation**: Automated tests comparing outputs