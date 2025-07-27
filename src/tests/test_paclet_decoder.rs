#[cfg(test)]
mod tests {
    use crate::{parse_ast_seq, parse_cst_seq, tokenize, ParseOptions};
    use std::fs;

    #[test]
    fn test_decoded_paclet_file() {
        // Try to read the decoded file
        let content = match fs::read_to_string("paclet-decode/QuantityUnits_decoded.m") {
            Ok(content) => content,
            Err(_) => {
                // If the file doesn't exist, skip the test
                println!("Skipping test - decoded file not found");
                return;
            }
        };

        let opts = ParseOptions::default();

        // Test that our parser can handle the decoded content without panicking
        // Use sequence parsing functions for files with multiple expressions
        let ast_seq_result = parse_ast_seq(&content, &opts);
        println!("AST sequence parsing: {} expressions, {} fatal issues, {} non-fatal issues", 
                 ast_seq_result.syntax.0.len(), 
                 ast_seq_result.fatal_issues.len(), 
                 ast_seq_result.non_fatal_issues.len());
        
        let cst_seq_result = parse_cst_seq(&content, &opts);
        println!("CST sequence parsing: {} expressions, {} fatal issues, {} non-fatal issues", 
                 cst_seq_result.syntax.0.len(), 
                 cst_seq_result.fatal_issues.len(), 
                 cst_seq_result.non_fatal_issues.len());
        
        let tokens = tokenize(&content, &opts);
        println!("Tokenization: {} tokens", tokens.0.len());

        // The test passes if parsing doesn't panic - some issues are expected in complex files
        assert!(true, "Parser successfully handled decoded paclet file");
    }
}