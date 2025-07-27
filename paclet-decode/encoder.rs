use std::fs;
use std::env;

// Huffman table extracted from encode.py - maps ASCII char codes (0-127) to bit strings
const HUFFMAN_TABLE: &[&str] = &[
    "1111110101100100000010", "1111110101100100000011", "1111110101100100000100", "1111110101100100000101",
    "1111110101100100000110", "1111110101100100000111", "1111110101100100001000", "1111110101100100001001",
    "1111110101100100001010", "00111", "10101", "1111110101100100001011", "1111110101100100001100",
    "1111110101100100001101", "1111110101100100001110", "1111110101100100001111", "1111110101100100010000",
    "1111110101100100010001", "1111110101100100010010", "1111110101100100010011", "1111110101100100010100",
    "1111110101100100010101", "1111110101100100010110", "1111110101100100010111", "1111110101100100011000",
    "1111110101100100011001", "1111110101100100011010", "1111110101100100011011", "1111110101100100011100",
    "1111110101100100011101", "1111110101100100011110", "1111110101100100011111", "110", "01100000101",
    "0000011", "10110110111", "101101101100", "1111110101101", "1111110100", "11111101011000", "0100100",
    "0100101", "11101100", "10111110", "11100", "1111011", "1110111", "1001111", "101100", "111100",
    "010110", "1011110", "1011010", "0101110", "0000010", "0110001", "0101111", "0100001", "1110101",
    "10111111", "101101101101", "010101", "01111100", "111111010100", "0111111100100", "010011101",
    "111011010", "0100110", "101101111", "011000010", "100111010", "011000000", "1001110110", "01111101",
    "011000001001", "111111010111", "101101100", "111111011", "101101110", "0111111101", "11111100",
    "0111111111", "01111110", "0100000", "01001111", "011111110011", "01111111000", "1001110111",
    "11111101011001001", "011000001000", "1111110101100101", "00101", "111111010101", "00110", "10011100",
    "1110100", "0110000011", "10001", "0101000", "011001", "1111010", "0001", "1001101", "1111101",
    "000010", "10100", "010011100", "111011011", "101110", "010001", "10010", "01101", "011110",
    "1011011010", "01110", "00100", "10000", "000011", "0101001", "011000011", "1111100", "1001100",
    "0111111110", "1111111", "0111111100101", "000000", "111111010110011", "111111010110010000000"
];

fn huffman_encode(body: &str) -> Result<String, String> {
    let mut result = Vec::new();
    
    for ch in body.chars() {
        let ascii_code = ch as u32;
        if ascii_code >= HUFFMAN_TABLE.len() as u32 {
            return Err(format!("Character '{}' (code {}) not in Huffman table", ch, ascii_code));
        }
        result.push(HUFFMAN_TABLE[ascii_code as usize]);
    }
    
    // Add EOT character (ASCII 4)
    result.push(HUFFMAN_TABLE[4]);
    
    let bits = result.join("");
    
    // Add trailing zeros to make multiple of 13 bits
    let padding = 13 - (bits.len() % 13);
    let padding = if padding == 13 { 0 } else { padding };
    
    Ok(format!("{}{}", bits, "0".repeat(padding)))
}

fn base95_encode(bits: &str) -> Result<String, String> {
    if bits.len() % 13 != 0 {
        return Err("Bit string length must be multiple of 13".to_string());
    }
    
    let mut result = String::new();
    let base = 95u32;
    let offset = 32u32;
    
    for chunk in bits.chars().collect::<Vec<char>>().chunks(13) {
        let bit_string: String = chunk.iter().collect();
        
        // Parse as binary and reverse (as done in Python encoder)
        let reversed_bits: String = bit_string.chars().rev().collect();
        let number = match u32::from_str_radix(&reversed_bits, 2) {
            Ok(n) => n,
            Err(_) => return Err(format!("Invalid binary string: {}", reversed_bits)),
        };
        
        let hi = number / base;
        let lo = number % base;
        
        result.push((hi + offset) as u8 as char);
        result.push((lo + offset) as u8 as char);
    }
    
    Ok(result)
}

fn split_into_lines(s: &str, line_length: usize) -> Vec<String> {
    s.chars()
        .collect::<Vec<char>>()
        .chunks(line_length)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

pub fn encode_paclet(content: &str) -> Result<String, String> {
    let bits = huffman_encode(content)?;
    let encoded_body = base95_encode(&bits)?;
    
    let mut result = String::from("(*!1N!*)mcm\n");
    let lines = split_into_lines(&encoded_body, 70);
    for line in lines {
        result.push_str(&line);
        result.push('\n');
    }
    
    Ok(result)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    
    match fs::read_to_string(input_file) {
        Ok(content) => {
            match encode_paclet(&content) {
                Ok(encoded) => {
                    match fs::write(output_file, encoded) {
                        Ok(_) => println!("Successfully encoded {} to {}", input_file, output_file),
                        Err(e) => {
                            eprintln!("Error writing output file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error encoding file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading input file: {}", e);
            std::process::exit(1);
        }
    }
}