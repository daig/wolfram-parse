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

#[derive(Debug)]
struct HuffmanNode {
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
    value: Option<char>,
}

impl HuffmanNode {
    fn new() -> Self {
        HuffmanNode {
            left: None,
            right: None,
            value: None,
        }
    }
}

fn build_huffman_tree() -> HuffmanNode {
    let mut root = HuffmanNode::new();
    
    for (ascii_code, bit_string) in HUFFMAN_TABLE.iter().enumerate() {
        let mut current = &mut root;
        
        for bit in bit_string.chars() {
            match bit {
                '0' => {
                    if current.left.is_none() {
                        current.left = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.left.as_mut().unwrap();
                }
                '1' => {
                    if current.right.is_none() {
                        current.right = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.right.as_mut().unwrap();
                }
                _ => panic!("Invalid bit character: {}", bit),
            }
        }
        
        current.value = Some(ascii_code as u8 as char);
    }
    
    root
}

fn decode_paclet(content: &str) -> Result<String, String> {
    // Step 1: Validate header
    if content.len() < 11 {
        return Err("File too short".to_string());
    }
    
    let header = &content[0..11];
    if header != "(*!1N!*)mcm" {
        return Err(format!("Invalid header: expected '(*!1N!*)mcm', got '{}'", header));
    }
    
    // Extract body (skip header and first newline)
    let body_start = if content.chars().nth(11) == Some('\n') { 12 } else { 11 };
    let body = &content[body_start..];
    
    // Remove newlines from body 
    let clean_body: String = body.chars().filter(|&c| c != '\n' && c != '\r').collect();
    
    // Step 2: Convert body to bitstream
    if clean_body.len() % 2 != 0 {
        return Err("Body length is not even".to_string());
    }
    
    let mut bitstream = String::new();
    let base = 95u32;
    let offset = 32u32;
    
    for chunk in clean_body.chars().collect::<Vec<char>>().chunks(2) {
        let c1 = chunk[0] as u32;
        let c2 = chunk[1] as u32;
        
        // Convert to offset values (characters are in range 32-126, so offset values are 0-94)
        let a = c1 - offset;
        let b = c2 - offset;
        
        if a >= base || b >= base {
            return Err(format!("Values out of range: {} {} (chars: {} {})", a, b, c1 as u8 as char, c2 as u8 as char));
        }
        
        // Note: The Python encoder reverses bits, so we need to reverse them back
        let value = a * base + b;
        let bits = format!("{:013b}", value);
        let reversed_bits: String = bits.chars().rev().collect();
        bitstream.push_str(&reversed_bits);
    }
    
    // Step 3: Build Huffman tree and decode
    let tree = build_huffman_tree();
    let mut output = String::new();
    let mut current = &tree;
    
    for bit in bitstream.chars() {
        match bit {
            '0' => {
                if let Some(ref left) = current.left {
                    current = left;
                } else {
                    // Hit a dead end, might be padding
                    break;
                }
            }
            '1' => {
                if let Some(ref right) = current.right {
                    current = right;
                } else {
                    // Hit a dead end, might be padding
                    break;
                }
            }
            _ => return Err(format!("Invalid bit: {}", bit)),
        }
        
        if let Some(ch) = current.value {
            // Check for EOT character (ASCII 4)
            if ch as u8 == 4 {
                break;
            }
            output.push(ch);
            current = &tree;
        }
    }
    
    Ok(output)
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
            match decode_paclet(&content) {
                Ok(decoded) => {
                    match fs::write(output_file, decoded) {
                        Ok(_) => println!("Successfully decoded {} to {}", input_file, output_file),
                        Err(e) => {
                            eprintln!("Error writing output file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error decoding file: {}", e);
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