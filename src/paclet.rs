//! Packlet decoding functionality for handling encoded Wolfram Language files.
//!
//! This module provides functionality to detect and decode paclet-encoded files,
//! which use a Huffman + base-95 encoding scheme to compress Wolfram Language source code.

use crate::error_handling::{ParseError, EncodingError};

/// Huffman table for paclet decoding - maps ASCII char codes (0-127) to bit strings
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

/// Huffman tree node for decoding
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

/// Build the Huffman tree for decoding
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
                _ => return root, // Invalid bit character, return partial tree
            }
        }
        
        current.value = Some(ascii_code as u8 as char);
    }
    
    root
}

/// Represents the version and table variant of a paclet header
#[derive(Debug, Clone, PartialEq)]
pub struct PackletHeader {
    pub version: char,
    pub variant: char,
}

impl PackletHeader {
    /// Check if this header version/variant combination is supported
    pub fn is_supported(&self) -> bool {
        self.version == '1' && self.variant == 'N'
    }
}

/// Detect if the input string contains a paclet header
/// 
/// Returns Some(PackletHeader) if a valid paclet header is found, None otherwise
pub fn detect_paclet_header(input: &str) -> Option<PackletHeader> {
    // Packlet header format: (*!{version}{variant}!*)
    if input.len() < 8 {
        return None;
    }
    
    // Check for the basic pattern
    if !input.starts_with("(*!") || !input[5..].starts_with("!*)") {
        return None;
    }
    
    // Extract version and variant characters
    let chars: Vec<char> = input.chars().collect();
    if chars.len() < 8 {
        return None;
    }
    
    let version = chars[3];
    let variant = chars[4];
    
    Some(PackletHeader { version, variant })
}

/// Decode a paclet-encoded string
/// 
/// This function assumes the input has already been verified to have a valid paclet header
pub fn decode_paclet(content: &str) -> Result<String, ParseError> {
    // Validate minimum length
    if content.len() < 11 {
        return Err(ParseError::Encoding(EncodingError::DecodeError {
            details: "File too short to be a valid paclet".to_string(),
        }));
    }
    
    // Extract and validate header
    let header = &content[0..8];
    let paclet_header = detect_paclet_header(content).ok_or_else(|| {
        ParseError::Encoding(EncodingError::DecodeError {
            details: format!("Invalid paclet header: {}", header),
        })
    })?;
    
    if !paclet_header.is_supported() {
        return Err(ParseError::Encoding(EncodingError::UnsupportedEncoding {
            encoding: format!("Packlet version {}{}",  paclet_header.version, paclet_header.variant),
        }));
    }
    
    // Check for "mcm" suffix after header
    if !content[8..].starts_with("mcm") {
        return Err(ParseError::Encoding(EncodingError::DecodeError {
            details: "Missing 'mcm' suffix in paclet header".to_string(),
        }));
    }
    
    // Extract body (skip header + "mcm" + newline)
    let body_start = if content.chars().nth(11) == Some('\n') { 12 } else { 11 };
    let body = &content[body_start..];
    
    // Remove newlines from body 
    let clean_body: String = body.chars().filter(|&c| c != '\n' && c != '\r').collect();
    
    // Validate body length
    if clean_body.len() % 2 != 0 {
        return Err(ParseError::Encoding(EncodingError::DecodeError {
            details: "Packlet body length is not even".to_string(),
        }));
    }
    
    // Convert body to bitstream
    let mut bitstream = String::new();
    let base = 95u32;
    let offset = 32u32;
    
    for chunk in clean_body.chars().collect::<Vec<char>>().chunks(2) {
        let c1 = chunk[0] as u32;
        let c2 = chunk[1] as u32;
        
        // Convert to offset values
        let a = c1 - offset;
        let b = c2 - offset;
        
        if a >= base || b >= base {
            return Err(ParseError::Encoding(EncodingError::DecodeError {
                details: format!("Invalid character values in paclet body: {} {} (chars: {} {})", 
                                a, b, c1 as u8 as char, c2 as u8 as char),
            }));
        }
        
        // Decode base-95 value and reverse bits (as per encoding algorithm)
        let value = a * base + b;
        let bits = format!("{:013b}", value);
        let reversed_bits: String = bits.chars().rev().collect();
        bitstream.push_str(&reversed_bits);
    }
    
    // Decode bitstream using Huffman tree
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
            _ => {
                return Err(ParseError::Encoding(EncodingError::DecodeError {
                    details: format!("Invalid bit character in bitstream: {}", bit),
                }));
            }
        }
        
        if let Some(ch) = current.value {
            // Check for EOT character (ASCII 4) - end of transmission
            if ch as u8 == 4 {
                break;
            }
            output.push(ch);
            current = &tree;
        }
    }
    
    Ok(output)
}

/// Try to decode input if it's a paclet, otherwise return the original input
/// 
/// This is the main integration function for the parser to use
pub fn maybe_decode_paclet(input: &str) -> Result<String, ParseError> {
    if let Some(header) = detect_paclet_header(input) {
        if header.is_supported() {
            // It's a supported paclet, decode it
            decode_paclet(input)
        } else {
            // It's a paclet but unsupported version
            Err(ParseError::Encoding(EncodingError::UnsupportedEncoding {
                encoding: format!("Packlet version {}{}", header.version, header.variant),
            }))
        }
    } else {
        // Not a paclet, return original input
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paclet_header_detection() {
        // Valid headers
        assert_eq!(detect_paclet_header("(*!1N!*)mcm"), Some(PackletHeader { version: '1', variant: 'N' }));
        assert_eq!(detect_paclet_header("(*!2A!*)mcm"), Some(PackletHeader { version: '2', variant: 'A' }));
        
        // Invalid headers
        assert_eq!(detect_paclet_header(""), None);
        assert_eq!(detect_paclet_header("(*!1N"), None);
        assert_eq!(detect_paclet_header("NotAPacklet"), None);
        assert_eq!(detect_paclet_header("(*!1N!"), None);
    }

    #[test]
    fn test_header_support() {
        let supported = PackletHeader { version: '1', variant: 'N' };
        let unsupported1 = PackletHeader { version: '2', variant: 'N' };
        let unsupported2 = PackletHeader { version: '1', variant: 'A' };
        
        assert!(supported.is_supported());
        assert!(!unsupported1.is_supported());
        assert!(!unsupported2.is_supported());
    }

    #[test]
    fn test_maybe_decode_paclet_passthrough() {
        let regular_content = "f[x_] := x + 1";
        let result = maybe_decode_paclet(regular_content).unwrap();
        assert_eq!(result, regular_content);
    }
}