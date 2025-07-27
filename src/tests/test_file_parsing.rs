#[cfg(test)]
mod tests {
    use crate::{parse_file_ast, parse_file_cst, tokenize_file, ParseOptions};
    use std::fs;

    #[test]
    fn test_parse_regular_file() {
        // Test parsing a regular .m file
        let test_content = "f[x_] := x + 1\ng[y_] := y^2";
        let temp_file = "/tmp/test_regular.m";
        
        fs::write(temp_file, test_content).expect("Failed to write test file");
        
        let opts = ParseOptions::default();
        
        // Test CST parsing
        let cst_result = parse_file_cst(temp_file, &opts).expect("Failed to parse CST from file");
        assert!(!cst_result.syntax.0.is_empty(), "Should have parsed some CST nodes");
        
        // Test AST parsing
        let ast_result = parse_file_ast(temp_file, &opts).expect("Failed to parse AST from file");
        assert!(!ast_result.syntax.0.is_empty(), "Should have parsed some AST nodes");
        
        // Test tokenization
        let tokens = tokenize_file(temp_file, &opts).expect("Failed to tokenize file");
        assert!(!tokens.0.is_empty(), "Should have tokenized some tokens");
        
        // Clean up
        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_parse_paclet_file() {
        // Test parsing a paclet-encoded file if it exists
        let paclet_file = "/Users/David.Girardo/Documents/wolfram-parser-rust/paclet-decode/QunatityUnits.m";
        
        if fs::metadata(paclet_file).is_ok() {
            let opts = ParseOptions::default();
            
            // Test CST parsing - should automatically decode the paclet
            let cst_result = parse_file_cst(paclet_file, &opts).expect("Failed to parse CST from paclet file");
            assert!(!cst_result.syntax.0.is_empty(), "Should have parsed some CST nodes from paclet");
            
            // Test AST parsing
            let ast_result = parse_file_ast(paclet_file, &opts).expect("Failed to parse AST from paclet file");
            assert!(!ast_result.syntax.0.is_empty(), "Should have parsed some AST nodes from paclet");
            
            // Test tokenization
            let tokens = tokenize_file(paclet_file, &opts).expect("Failed to tokenize paclet file");
            assert!(!tokens.0.is_empty(), "Should have tokenized some tokens from paclet");
            
            println!("✅ Successfully parsed paclet file with {} CST nodes, {} AST nodes, {} tokens", 
                     cst_result.syntax.0.len(), ast_result.syntax.0.len(), tokens.0.len());
        } else {
            println!("ℹ️  Packlet test file not found, skipping paclet test");
        }
    }
    
    #[test]
    fn test_file_vs_string_parsing_equivalence() {
        // Test that file parsing gives same result as string parsing for regular files
        let test_content = "f[x_] := x + 1";
        let temp_file = "/tmp/test_equivalence.m";
        
        fs::write(temp_file, test_content).expect("Failed to write test file");
        
        let opts = ParseOptions::default();
        
        // Parse from file
        let file_cst = parse_file_cst(temp_file, &opts).expect("Failed to parse CST from file");
        let file_ast = parse_file_ast(temp_file, &opts).expect("Failed to parse AST from file");
        let file_tokens = tokenize_file(temp_file, &opts).expect("Failed to tokenize file");
        
        // Parse from string
        let string_cst = crate::parse_cst_seq(test_content, &opts);
        let string_ast = crate::parse_ast_seq(test_content, &opts);
        let string_tokens = crate::tokenize(test_content, &opts);
        
        // Results should be equivalent in structure
        assert_eq!(file_cst.syntax.0.len(), string_cst.syntax.0.len(), "CST node count should match");
        assert_eq!(file_ast.syntax.0.len(), string_ast.syntax.0.len(), "AST node count should match");
        assert_eq!(file_tokens.0.len(), string_tokens.0.len(), "Token count should match");
        
        // Clean up
        fs::remove_file(temp_file).ok();
        
        println!("✅ File parsing produces equivalent results to string parsing");
    }
}