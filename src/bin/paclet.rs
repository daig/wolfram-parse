use clap::{Arg, Command};
use std::fs;
use std::path::Path;
use wolfram_parser::paclet;

fn main() {
    let matches = Command::new("paclet")
        .version("1.0.0")
        .author("Wolfram Parser Rust")
        .about("Encode and decode Wolfram Language paclet files")
        .subcommand(
            Command::new("decode")
                .about("Decode a paclet file to readable Wolfram Language")
                .arg(
                    Arg::new("input")
                        .help("Input paclet file (.m)")
                        .required(true)
                        .index(1)
                )
                .arg(
                    Arg::new("output")
                        .help("Output file (optional, defaults to input with _decoded suffix)")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                )
        )
        .subcommand(
            Command::new("encode")
                .about("Encode a Wolfram Language file as a paclet")
                .arg(
                    Arg::new("input")
                        .help("Input Wolfram Language file (.m)")
                        .required(true)
                        .index(1)
                )
                .arg(
                    Arg::new("output")
                        .help("Output paclet file (optional, defaults to input with _encoded suffix)")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                )
        )
        .subcommand(
            Command::new("check")
                .about("Check if a file is a paclet and show header information")
                .arg(
                    Arg::new("input")
                        .help("Input file to check")
                        .required(true)
                        .index(1)
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("decode", sub_matches)) => {
            let input_path = sub_matches.get_one::<String>("input").unwrap();
            let output_path = sub_matches
                .get_one::<String>("output")
                .map(|s| s.as_str())
                .unwrap_or_else(|| {
                    // Default output: input_decoded.m
                    let path = Path::new(input_path);
                    let stem = path.file_stem().unwrap().to_str().unwrap();
                    let extension = path.extension().unwrap_or_default().to_str().unwrap();
                    Box::leak(format!("{}_decoded.{}", stem, extension).into_boxed_str())
                });

            if let Err(e) = decode_file(input_path, output_path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(("encode", sub_matches)) => {
            let input_path = sub_matches.get_one::<String>("input").unwrap();
            let output_path = sub_matches
                .get_one::<String>("output")
                .map(|s| s.as_str())
                .unwrap_or_else(|| {
                    // Default output: input_encoded.m
                    let path = Path::new(input_path);
                    let stem = path.file_stem().unwrap().to_str().unwrap();
                    let extension = path.extension().unwrap_or_default().to_str().unwrap();
                    Box::leak(format!("{}_encoded.{}", stem, extension).into_boxed_str())
                });

            if let Err(e) = encode_file(input_path, output_path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(("check", sub_matches)) => {
            let input_path = sub_matches.get_one::<String>("input").unwrap();
            
            if let Err(e) = check_file(input_path) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("No subcommand provided. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}

fn decode_file(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Decoding paclet file: {} -> {}", input_path, output_path);
    
    // Read input file
    let content = fs::read_to_string(input_path)?;
    
    // Check if it's a paclet
    if let Some(header) = paclet::detect_paclet_header(&content) {
        println!("Detected paclet header: version {}, variant {}", header.version, header.variant);
        
        if !header.is_supported() {
            return Err(format!("Unsupported paclet version {}{}", header.version, header.variant).into());
        }
        
        // Decode the paclet
        let decoded = paclet::decode_paclet(&content)?;
        
        // Write decoded content
        fs::write(output_path, decoded)?;
        
        println!("✅ Successfully decoded paclet file");
        println!("   Input size: {} bytes", content.len());
        println!("   Output size: {} bytes", fs::metadata(output_path)?.len());
        
    } else {
        return Err("Input file is not a paclet (no valid paclet header found)".into());
    }
    
    Ok(())
}

fn encode_file(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Encoding file as paclet: {} -> {}", input_path, output_path);
    
    // For now, we'll use the external encoder from the paclet-decode directory
    // In the future, we could integrate the Rust encoder directly
    
    let paclet_dir = "/Users/David.Girardo/Documents/wolfram-parser-rust/paclet-decode";
    let encoder_path = format!("{}/encoder", paclet_dir);
    
    // Check if encoder exists
    if !Path::new(&encoder_path).exists() {
        // Try to compile it
        println!("Compiling Rust encoder...");
        let compile_result = std::process::Command::new("rustc")
            .args(&[
                &format!("{}/encoder.rs", paclet_dir),
                "-o", &encoder_path
            ])
            .current_dir(paclet_dir)
            .output()?;
            
        if !compile_result.status.success() {
            return Err(format!("Failed to compile encoder: {}", 
                String::from_utf8_lossy(&compile_result.stderr)).into());
        }
    }
    
    // Run the encoder
    println!("Running encoder...");
    let result = std::process::Command::new(&encoder_path)
        .args(&[input_path, output_path])
        .output()?;
        
    if !result.status.success() {
        return Err(format!("Encoder failed: {}", 
            String::from_utf8_lossy(&result.stderr)).into());
    }
    
    println!("✅ Successfully encoded file as paclet");
    println!("   Input size: {} bytes", fs::metadata(input_path)?.len());
    println!("   Output size: {} bytes", fs::metadata(output_path)?.len());
    
    Ok(())
}

fn check_file(input_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking file: {}", input_path);
    
    let content = fs::read_to_string(input_path)?;
    let file_size = content.len();
    
    println!("File size: {} bytes", file_size);
    
    if let Some(header) = paclet::detect_paclet_header(&content) {
        println!("🎯 This is a paclet file!");
        println!("   Header version: {}", header.version);
        println!("   Header variant: {}", header.variant);
        println!("   Supported: {}", if header.is_supported() { "✅ Yes" } else { "❌ No" });
        
        if header.is_supported() {
            // Try to decode and show stats
            match paclet::decode_paclet(&content) {
                Ok(decoded) => {
                    println!("   Decoded size: {} bytes", decoded.len());
                    println!("   Compression ratio: {:.1}x", file_size as f64 / decoded.len() as f64);
                    
                    // Show a preview of the decoded content
                    let preview = if decoded.len() > 200 {
                        format!("{}...", &decoded[..200])
                    } else {
                        decoded
                    };
                    println!("   Preview: {}", preview.replace('\n', "\\n"));
                }
                Err(e) => {
                    println!("   ❌ Failed to decode: {}", e);
                }
            }
        }
    } else {
        println!("📄 This is a regular Wolfram Language file (not a paclet)");
        
        // Show a preview of the content
        let preview = if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content
        };
        println!("   Preview: {}", preview.replace('\n', "\\n"));
    }
    
    Ok(())
}