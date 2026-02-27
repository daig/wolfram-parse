//! Smoke tests for parsing real .nb (Mathematica notebook) files.
//!
//! These tests parse .nb files from the SLL repository through the full
//! pipeline: text parse → AST → Expr conversion → box parse of cell contents.

use std::path::{Path, PathBuf};

use wolfram_expr::{ExprKind, Symbol};

use crate::{
    ast::Ast,
    box_parse::parse_box_expr,
    parse_ast_seq, ParseOptions,
};

/// Find all .nb files under a directory.
fn find_nb_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().map_or(false, |e| e == "nb") {
                    out.push(path);
                }
            }
        }
    }
    walk(dir, &mut files);
    files.sort();
    files
}

/// Result of parsing a single .nb file.
#[derive(Debug)]
struct NbParseResult {
    path: PathBuf,
    /// Stage 1: text parsing errors (fatal issues from parse_ast_seq)
    text_parse_fatal: usize,
    /// Number of Cell[BoxData[...]] cells found
    cells_found: usize,
    /// Number of cells that box-parsed without panic
    cells_parsed_ok: usize,
    /// Number of cells where box parsing panicked
    cells_panicked: usize,
    /// Descriptions of panics encountered
    panic_descriptions: Vec<String>,
}

/// Parse a single .nb file through the full pipeline.
fn parse_nb_file(path: &Path) -> NbParseResult {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return NbParseResult {
                path: path.to_path_buf(),
                text_parse_fatal: 1,
                cells_found: 0,
                cells_parsed_ok: 0,
                cells_panicked: 0,
                panic_descriptions: vec![format!("Read error: {e}")],
            };
        },
    };

    let opts = ParseOptions::default();
    let result = parse_ast_seq(&content, &opts);

    let text_parse_fatal = result.fatal_issues.len();

    // Walk the AST to find BoxData contents and try box parsing them
    let mut cells_found = 0;
    let mut cells_parsed_ok = 0;
    let mut cells_panicked = 0;
    let mut panic_descriptions = Vec::new();

    let box_data_exprs = extract_box_data_from_ast_nodes(&result.syntax.0);
    cells_found = box_data_exprs.len();

    for (i, box_expr) in box_data_exprs.iter().enumerate() {
        let result = std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| {
                let _cst = parse_box_expr(box_expr);
            }),
        );
        match result {
            Ok(()) => cells_parsed_ok += 1,
            Err(panic_val) => {
                cells_panicked += 1;
                let msg = if let Some(s) = panic_val.downcast_ref::<String>()
                {
                    s.clone()
                } else if let Some(s) = panic_val.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                panic_descriptions.push(format!(
                    "cell {i}: {msg}"
                ));
            },
        }
    }

    NbParseResult {
        path: path.to_path_buf(),
        text_parse_fatal,
        cells_found,
        cells_parsed_ok,
        cells_panicked,
        panic_descriptions,
    }
}

/// Recursively walk AST nodes to find arguments of BoxData[...] calls,
/// and convert them to wolfram_expr::Expr for box parsing.
fn extract_box_data_from_ast_nodes(
    nodes: &[Ast],
) -> Vec<wolfram_expr::Expr> {
    let mut results = Vec::new();
    for node in nodes {
        extract_box_data_recursive(node, &mut results);
    }
    results
}

fn extract_box_data_recursive(
    ast: &Ast,
    results: &mut Vec<wolfram_expr::Expr>,
) {
    match ast {
        Ast::Call { head, args, .. }
        | Ast::CallMissingCloser { head, args, .. } => {
            // Check if this is BoxData[content]
            if is_symbol_named(head, "BoxData") || is_symbol_named(head, "System`BoxData") {
                // Convert each arg to Expr and add
                for arg in args {
                    results.push(arg.to_expr());
                }
            }
            // Also recurse into args to find nested BoxData
            // (e.g. Cell[BoxData[...]] is a Call with Cell as head)
            extract_box_data_recursive(head, results);
            for arg in args {
                extract_box_data_recursive(arg, results);
            }
        },
        Ast::Group { children, .. } => {
            extract_box_data_recursive(&children.0, results);
            extract_box_data_recursive(&children.1, results);
            extract_box_data_recursive(&children.2, results);
        },
        Ast::Box { args, .. } => {
            for arg in args {
                extract_box_data_recursive(arg, results);
            }
        },
        Ast::SyntaxError { children, .. } => {
            for child in children {
                extract_box_data_recursive(child, results);
            }
        },
        Ast::AbstractSyntaxError { args, .. } => {
            for arg in args {
                extract_box_data_recursive(arg, results);
            }
        },
        Ast::GroupMissingCloser { children, .. }
        | Ast::GroupMissingOpener { children, .. } => {
            for child in children {
                extract_box_data_recursive(child, results);
            }
        },
        Ast::TagBox_GroupParen { group, .. } => {
            extract_box_data_recursive(&group.0, results);
            extract_box_data_recursive(&group.1, results);
            extract_box_data_recursive(&group.2, results);
        },
        Ast::PrefixNode_PrefixLinearSyntaxBang(children, _) => {
            extract_box_data_recursive(&children[0], results);
            extract_box_data_recursive(&children[1], results);
        },
        // Leaf and Error nodes have no children
        Ast::Leaf { .. } | Ast::Error { .. } | Ast::Code { .. } => {},
    }
}

/// Check if an AST node is a Symbol with the given name.
fn is_symbol_named(ast: &Ast, name: &str) -> bool {
    match ast {
        Ast::Leaf {
            kind: crate::tokenize::TokenKind::Symbol,
            input,
            ..
        } => input.to_str() == name,
        _ => false,
    }
}

#[test]
fn test_parse_all_sll_nb_files() {
    // .nb files can produce deeply nested ASTs; run with a larger stack.
    let result = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(test_parse_all_sll_nb_files_inner)
        .unwrap()
        .join();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

fn test_parse_all_sll_nb_files_inner() {
    let sll_dir = Path::new("/Users/david/Documents/SLL");
    let nb_files = find_nb_files(sll_dir);

    if nb_files.is_empty() {
        eprintln!("No .nb files found in {sll_dir:?}, skipping test");
        return;
    }

    eprintln!("Found {} .nb files", nb_files.len());

    let mut total_files = 0;
    let mut total_cells = 0;
    let mut total_ok = 0;
    let mut total_panicked = 0;
    let mut total_text_errors = 0;
    let mut problem_files: Vec<NbParseResult> = Vec::new();

    for path in &nb_files {
        let result = parse_nb_file(path);

        total_files += 1;
        total_cells += result.cells_found;
        total_ok += result.cells_parsed_ok;
        total_panicked += result.cells_panicked;
        if result.text_parse_fatal > 0 {
            total_text_errors += 1;
        }

        if result.cells_panicked > 0 || result.text_parse_fatal > 0 {
            problem_files.push(result);
        }
    }

    // Print summary
    eprintln!("\n=== .nb Parsing Summary ===");
    eprintln!("Files:         {total_files}");
    eprintln!("Cells found:   {total_cells}");
    eprintln!("Box parse OK:  {total_ok}");
    eprintln!("Box panicked:  {total_panicked}");
    eprintln!("Text errors:   {total_text_errors}");
    eprintln!();

    if !problem_files.is_empty() {
        eprintln!("=== Problem Files ===");
        for pf in &problem_files {
            let short_path = pf
                .path
                .strip_prefix(sll_dir)
                .unwrap_or(&pf.path);
            eprintln!(
                "\n{}:",
                short_path.display()
            );
            if pf.text_parse_fatal > 0 {
                eprintln!(
                    "  Text parse: {} fatal issue(s)",
                    pf.text_parse_fatal
                );
            }
            if pf.cells_panicked > 0 {
                eprintln!(
                    "  Box parse: {}/{} cells panicked",
                    pf.cells_panicked, pf.cells_found
                );
                for desc in &pf.panic_descriptions {
                    eprintln!("    - {desc}");
                }
            }
        }
        eprintln!();
    }

    // The test passes as long as there are no panics — we want to document
    // issues, not block on them. Text parse errors are expected for some
    // notebooks (e.g., binary content, unusual encodings).
    assert_eq!(
        total_panicked, 0,
        "{total_panicked} cells caused panics during box parsing"
    );
}
