use wolfram_parser::{parse_ast, parse_cst, tokenize, ParseOptions};

fn main() {
    let input = "f[x_, y_] := x + y";
    let opts = ParseOptions::default();
    
    println!("Input: {}", input);
    println!();
    
    // Parse to AST
    let ast_result = parse_ast(input, &opts);
    println!("AST:");
    println!("{:#?}", ast_result.syntax);
    println!();
    
    // Parse to CST
    let cst_result = parse_cst(input, &opts);
    println!("CST:");
    println!("{:#?}", cst_result.syntax);
    println!();
    
    // Tokenize
    let tokens = tokenize(input, &opts);
    println!("First 5 tokens:");
    for (i, token) in tokens.0.iter().take(5).enumerate() {
        println!("  {}: {:?}", i, token);
    }
}