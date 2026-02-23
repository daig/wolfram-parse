#[cfg(test)]
mod tests {
    use crate::{parse_file_ast, parse_file_cst, tokenize_file, ParseOptions};
    use std::fs;
    use std::path::PathBuf;

    /// Returns the path to the project's tests/files/ directory.
    fn test_files_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/files")
    }

    #[test]
    fn test_parse_regular_file() {
        let test_content = "f[x_] := x + 1\ng[y_] := y^2";
        let temp_file = std::env::temp_dir().join("wolfram_parse_test_regular.m");
        let temp_path = temp_file.to_str().unwrap();

        fs::write(&temp_file, test_content).expect("Failed to write test file");

        let opts = ParseOptions::default();

        let cst_result =
            parse_file_cst(temp_path, &opts).expect("Failed to parse CST from file");
        assert!(
            !cst_result.syntax.0.is_empty(),
            "Should have parsed some CST nodes"
        );

        let ast_result =
            parse_file_ast(temp_path, &opts).expect("Failed to parse AST from file");
        assert!(
            !ast_result.syntax.0.is_empty(),
            "Should have parsed some AST nodes"
        );

        let tokens = tokenize_file(temp_path, &opts).expect("Failed to tokenize file");
        assert!(!tokens.0.is_empty(), "Should have tokenized some tokens");

        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_parse_paclet_file() {
        let paclet_file = test_files_dir().join("QuantityUnits.m");
        let paclet_path = paclet_file.to_str().unwrap();

        let opts = ParseOptions::default();

        let cst_result = parse_file_cst(paclet_path, &opts)
            .expect("Failed to parse CST from paclet file");
        assert!(
            !cst_result.syntax.0.is_empty(),
            "Should have parsed some CST nodes from paclet"
        );

        let ast_result = parse_file_ast(paclet_path, &opts)
            .expect("Failed to parse AST from paclet file");
        assert!(
            !ast_result.syntax.0.is_empty(),
            "Should have parsed some AST nodes from paclet"
        );

        let tokens = tokenize_file(paclet_path, &opts)
            .expect("Failed to tokenize paclet file");
        assert!(
            !tokens.0.is_empty(),
            "Should have tokenized some tokens from paclet"
        );
    }

    #[test]
    fn test_file_vs_string_parsing_equivalence() {
        let test_content = "f[x_] := x + 1";
        let temp_file =
            std::env::temp_dir().join("wolfram_parse_test_equivalence.m");
        let temp_path = temp_file.to_str().unwrap();

        fs::write(&temp_file, test_content).expect("Failed to write test file");

        let opts = ParseOptions::default();

        let file_cst =
            parse_file_cst(temp_path, &opts).expect("Failed to parse CST from file");
        let file_ast =
            parse_file_ast(temp_path, &opts).expect("Failed to parse AST from file");
        let file_tokens =
            tokenize_file(temp_path, &opts).expect("Failed to tokenize file");

        let string_cst = crate::parse_cst_seq(test_content, &opts);
        let string_ast = crate::parse_ast_seq(test_content, &opts);
        let string_tokens = crate::tokenize(test_content, &opts);

        assert_eq!(
            file_cst.syntax.0.len(),
            string_cst.syntax.0.len(),
            "CST node count should match"
        );
        assert_eq!(
            file_ast.syntax.0.len(),
            string_ast.syntax.0.len(),
            "AST node count should match"
        );
        assert_eq!(
            file_tokens.0.len(),
            string_tokens.0.len(),
            "Token count should match"
        );

        fs::remove_file(&temp_file).ok();
    }
}
