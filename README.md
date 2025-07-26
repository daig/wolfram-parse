# Wolfram Parser

A high-performance parser for Wolfram Language input form syntax, written in Rust.

## Overview

This library implements a fully-featured parser for [Wolfram Language](https://wolfram.com/language) input form syntax. Given a string containing Wolfram Language code, it can produce:

- **Abstract Syntax Tree (AST)** - A high-level representation of the code structure
- **Concrete Syntax Tree (CST)** - A detailed representation including all syntactic elements
- **Token Stream** - A sequence of lexical tokens

## Features

- Fast, zero-copy parsing
- Complete support for Wolfram Language syntax
- Detailed source location tracking
- Graceful error recovery
- Unicode support
- Configurable parsing options

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
wolfram-parser = "0.1.0"
```

## Usage

### Basic Parsing

```rust
use wolfram_parser::{parse_ast, parse_cst, tokenize, ParseOptions};

fn main() {
    let input = "1 + 2 * 3";
    let opts = ParseOptions::default();
    
    // Parse to Abstract Syntax Tree
    let ast_result = parse_ast(input, &opts);
    println!("AST: {:#?}", ast_result.syntax);
    
    // Parse to Concrete Syntax Tree  
    let cst_result = parse_cst(input, &opts);
    println!("CST: {:#?}", cst_result.syntax);
    
    // Tokenize
    let token_result = tokenize(input, &opts);
    println!("Tokens: {:#?}", token_result.syntax);
}
```

### Parsing with Error Handling

```rust
use wolfram_parser::{parse_ast, ParseOptions};

let input = "f[x_, y_] := x + y";
let opts = ParseOptions::default();
let result = parse_ast(input, &opts);

// Check for issues
if !result.fatal_issues.is_empty() {
    eprintln!("Fatal parsing errors:");
    for issue in &result.fatal_issues {
        eprintln!("  {:?}", issue);
    }
}

if !result.non_fatal_issues.is_empty() {
    eprintln!("Warnings:");
    for issue in &result.non_fatal_issues {
        eprintln!("  {:?}", issue);
    }
}
```

## API

| Operation | Result | Function |
|-----------|--------|----------|
| Parse abstract syntax | `Ast` | `parse_ast()` |
| Parse concrete syntax | `Cst` | `parse_cst()` |
| Tokenization | `NodeSeq<Token>` | `tokenize()` |
| Parse abstract syntax sequence | `NodeSeq<Ast>` | `parse_ast_seq()` |
| Parse concrete syntax sequence | `NodeSeq<Cst>` | `parse_cst_seq()` |

### Byte Input Variants

For parsing raw bytes (with encoding detection):

- `parse_bytes_ast()`
- `parse_bytes_cst()` 
- `tokenize_bytes()`
- `parse_bytes_ast_seq()`
- `parse_bytes_cst_seq()`

## Building from Source

This is a standalone Rust project that can be built with standard Cargo commands:

```bash
# Build the library
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Rust Version

This project requires Rust 1.75 or later.

## Generated Code

This parser includes two generated files that contain parsing data:

- `src/generated/long_names_registration.rs` - Character constants and mappings for Wolfram Language special characters
- `src/generated/precedence_values.rs` - Operator precedence values

These files were generated from Wolfram Language data files and should not be manually edited. They contain fundamental parsing information such as:

- Unicode mappings for Wolfram Language named characters (e.g., `\[Alpha]` → α)
- Operator precedence and associativity rules
- Character classification data

## Architecture

The parser uses a hybrid tokenizer/parser architecture:

1. **Tokenizer** - Converts input bytes/characters into a stream of tokens
2. **Parser** - Builds syntax trees from the token stream using a Pratt parser
3. **Abstract** - Optionally converts concrete syntax to abstract syntax

The parsing pipeline:
```
Bytes → Characters → Tokens → CST → AST
```

## Testing

The project includes comprehensive test suites:

```bash
# Run all tests
cargo test

# Run specific test module
cargo test test_api

# Run tests with output
cargo test -- --nocapture
```

## Performance

This parser is designed for high performance:

- Zero-copy parsing where possible
- Efficient memory usage
- Optimized tokenization
- Optional features for additional optimizations

Run benchmarks with:
```bash
cargo bench
```

## License

This project is licensed under the MIT License.

## Origin

This is a standalone Rust version of the parser component from the [CodeParser](https://github.com/WolframResearch/codeparser) project, extracted and simplified for easier integration into Rust projects.