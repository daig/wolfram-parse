use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

// ─── JSON schemas ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LongName {
    name: String,
    category: String,
    codepoint: CodepointValue,
    #[serde(default)]
    ascii: Vec<String>,
    #[serde(default)]
    extra: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum CodepointValue {
    Int(u32),
    Special(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PrecedenceEntry {
    Primary {
        name: String,
        value: PrecValue,
        assoc: String,
    },
    Alias {
        name: String,
        same_as: String,
    },
}

/// Precedence value: either a concrete integer or the string "next".
#[derive(Deserialize)]
#[serde(untagged)]
enum PrecValue {
    Int(u32),
    /// The string `"next"` — means "increment from the previous level".
    /// The inner string is always `"next"` and isn't read; only the
    /// variant discriminant matters.
    Next(String),
}

// ─── helpers ─────────────────────────────────────────────────────────

/// Nominal integer codepoint for sorting. Special entries get negative-ish
/// sentinel values that place them at the correct position in the WL data
/// (they appear at 0x0022 and 0x005c respectively).
fn nominal_codepoint(entry: &LongName) -> i64 {
    match &entry.codepoint {
        CodepointValue::Int(n) => *n as i64,
        CodepointValue::Special(s) => match s.as_str() {
            "StringMeta_DoubleQuote" => 0x0022,
            "StringMeta_Backslash" => 0x005c,
            _ => panic!("Unknown special codepoint: {s}"),
        },
    }
}

fn is_special(entry: &LongName) -> bool {
    matches!(&entry.codepoint, CodepointValue::Special(_))
}

fn const_name(name: &str) -> String {
    format!("CODEPOINT_LONGNAME_{}", name.to_uppercase())
}

/// Rust expression for a CodePoint value
fn codepoint_expr(entry: &LongName) -> String {
    match &entry.codepoint {
        CodepointValue::Int(n) => format!("CodePoint::Char('\\u{{{:04x}}}')", n),
        CodepointValue::Special(s) => format!("CodePoint::{s}"),
    }
}

/// Rust expression for a char literal (only valid for non-special entries)
fn char_literal(n: u32) -> String {
    format!("'\\u{{{:04x}}}'", n)
}

// ─── generators ──────────────────────────────────────────────────────

fn generate_long_names_constants(out_dir: &Path, entries: &[LongName]) {
    let path = out_dir.join("long_names_constants.rs");
    let mut f = BufWriter::new(fs::File::create(path).unwrap());

    // Count constants
    let raw_count = entries.iter().filter(|e| e.category == "raw").count();
    let nsl_count = entries
        .iter()
        .filter(|e| e.category == "letterlike" && e.extra.contains(&"not_strange_letterlike".to_string()))
        .count();
    let punct_count = entries
        .iter()
        .filter(|e| e.category == "punctuation")
        .count();
    let ws_count = entries
        .iter()
        .filter(|e| e.category == "whitespace")
        .count();
    let nl_count = entries
        .iter()
        .filter(|e| e.category == "newline")
        .count();
    // MB_NEWLINE includes CodePoint::CRLF prepended, so +1
    let nl_mb_count = nl_count + 1;
    let uninterp_count = entries
        .iter()
        .filter(|e| e.category == "uninterpretable")
        .count();

    writeln!(f, "// Array size constants").unwrap();
    writeln!(f, "pub const LONGNAMES_COUNT: usize = {};", entries.len()).unwrap();
    writeln!(f, "pub const RAWLONGNAMES_COUNT: usize = {raw_count};").unwrap();
    writeln!(f, "pub const MBNOTSTRANGELETTERLIKECODEPOINTS_COUNT: usize = {nsl_count};").unwrap();
    writeln!(f, "pub const MBPUNCTUATIONCODEPOINTS_COUNT: usize = {punct_count};").unwrap();
    writeln!(f, "pub const MBWHITESPACECODEPOINTS_COUNT: usize = {ws_count};").unwrap();
    writeln!(f, "pub const MBNEWLINECODEPOINTS_COUNT: usize = {nl_mb_count};").unwrap();
    writeln!(f, "pub const MBUNINTERPRETABLECODEPOINTS_COUNT: usize = {uninterp_count};").unwrap();

    // Character constants
    for entry in entries {
        let cname = const_name(&entry.name);
        match &entry.codepoint {
            CodepointValue::Int(n) => {
                writeln!(f, "pub const {cname}: char = {};", char_literal(*n)).unwrap();
            }
            CodepointValue::Special(s) => {
                writeln!(f, "pub const {cname}: CodePoint = CodePoint::{s};").unwrap();
            }
        }
    }
}

fn generate_long_names_mappings(out_dir: &Path, entries: &[LongName]) {
    let path = out_dir.join("long_names_mappings.rs");
    let mut f = BufWriter::new(fs::File::create(&path).unwrap());

    // Build the PHF map: longname string -> CodePoint expression
    let mut phf_map = phf_codegen::Map::new();
    for entry in entries {
        let expr = codepoint_expr(entry);
        phf_map.entry(&entry.name, &expr);
    }

    writeln!(
        f,
        "pub static LONGNAME_TO_CODEPOINT_MAP: phf::Map<&'static str, CodePoint> = {};",
        phf_map.build()
    )
    .unwrap();

    // Build sorted CODEPOINT_TO_LONGNAME_MAP array
    // Sort by nominal codepoint (matching upstream order for binary search)
    let mut sorted: Vec<&LongName> = entries.iter().collect();
    sorted.sort_by_key(|e| nominal_codepoint(e));

    writeln!(
        f,
        "\npub const CODEPOINT_TO_LONGNAME_MAP: [(CodePoint, &str); LONGNAMES_COUNT] = ["
    )
    .unwrap();
    for entry in &sorted {
        let expr = codepoint_expr(entry);
        writeln!(f, "\t({expr}, \"{}\"),", entry.name).unwrap();
    }
    writeln!(f, "];").unwrap();

    // Wrapper functions
    writeln!(f).unwrap();
    writeln!(f, "/// Look up a CodePoint by its long name string").unwrap();
    writeln!(f, "pub fn longname_to_codepoint(longname: &str) -> Option<CodePoint> {{").unwrap();
    writeln!(f, "    LONGNAME_TO_CODEPOINT_MAP.get(longname).copied()").unwrap();
    writeln!(f, "}}").unwrap();
    writeln!(f).unwrap();
    writeln!(
        f,
        "/// Look up a long name string by its CodePoint using binary search"
    )
    .unwrap();
    writeln!(
        f,
        "pub fn codepoint_to_longname(codepoint: CodePoint) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(
        f,
        "    CODEPOINT_TO_LONGNAME_MAP.binary_search_by_key(&codepoint, |&(cp, _)| cp)"
    )
    .unwrap();
    writeln!(f, "        .ok()").unwrap();
    writeln!(f, "        .map(|index| CODEPOINT_TO_LONGNAME_MAP[index].1)").unwrap();
    writeln!(f, "}}").unwrap();
}

fn generate_long_names_character_classes(out_dir: &Path, entries: &[LongName]) {
    let path = out_dir.join("long_names_character_classes.rs");
    let mut f = BufWriter::new(fs::File::create(path).unwrap());

    // RAW_SET: names of all raw category entries, sorted lexicographically
    let mut raw_names: Vec<&str> = entries
        .iter()
        .filter(|e| e.category == "raw")
        .map(|e| e.name.as_str())
        .collect();
    raw_names.sort();

    writeln!(f, "pub const RAW_SET: [&str; RAWLONGNAMES_COUNT] = [").unwrap();
    for name in &raw_names {
        writeln!(f, "\t\"{name}\",").unwrap();
    }
    writeln!(f, "];").unwrap();

    // MB_NOT_STRAGE_LETTERLIKE_CODE_POINTS: letterlike with not_strange_letterlike extra, sorted by codepoint
    let mut nsl: Vec<u32> = entries
        .iter()
        .filter(|e| {
            e.category == "letterlike"
                && e.extra.contains(&"not_strange_letterlike".to_string())
                && !is_special(e)
        })
        .map(|e| match &e.codepoint {
            CodepointValue::Int(n) => *n,
            _ => unreachable!(),
        })
        .collect();
    nsl.sort();

    writeln!(
        f,
        "\npub const MB_NOT_STRAGE_LETTERLIKE_CODE_POINTS: [char; MBNOTSTRANGELETTERLIKECODEPOINTS_COUNT] = ["
    )
    .unwrap();
    for cp in &nsl {
        writeln!(f, "\t{},", char_literal(*cp)).unwrap();
    }
    writeln!(f, "];").unwrap();

    // MB_PUNCTUATION_CODE_POINTS: all punctuation, sorted by codepoint
    let mut punct: Vec<u32> = entries
        .iter()
        .filter(|e| e.category == "punctuation" && !is_special(e))
        .map(|e| match &e.codepoint {
            CodepointValue::Int(n) => *n,
            _ => unreachable!(),
        })
        .collect();
    punct.sort();

    writeln!(
        f,
        "\npub const MB_PUNCTUATION_CODE_POINTS: [char; MBPUNCTUATIONCODEPOINTS_COUNT] = ["
    )
    .unwrap();
    for cp in &punct {
        writeln!(f, "\t{},", char_literal(*cp)).unwrap();
    }
    writeln!(f, "];").unwrap();

    // MB_WHITESPACE_CODE_POINTS: all whitespace, sorted by codepoint
    let mut ws: Vec<u32> = entries
        .iter()
        .filter(|e| e.category == "whitespace" && !is_special(e))
        .map(|e| match &e.codepoint {
            CodepointValue::Int(n) => *n,
            _ => unreachable!(),
        })
        .collect();
    ws.sort();

    writeln!(
        f,
        "\npub const MB_WHITESPACE_CODE_POINTS: [char; MBWHITESPACECODEPOINTS_COUNT] = ["
    )
    .unwrap();
    for cp in &ws {
        writeln!(f, "\t{},", char_literal(*cp)).unwrap();
    }
    writeln!(f, "];").unwrap();

    // MB_NEWLINE_CODE_POINTS: CRLF prepended, then newline entries sorted by codepoint
    // Uses CodePoint enum, not char
    let mut nl: Vec<u32> = entries
        .iter()
        .filter(|e| e.category == "newline" && !is_special(e))
        .map(|e| match &e.codepoint {
            CodepointValue::Int(n) => *n,
            _ => unreachable!(),
        })
        .collect();
    nl.sort();

    writeln!(
        f,
        "\npub const MB_NEWLINE_CODE_POINTS: [CodePoint; MBNEWLINECODEPOINTS_COUNT] = ["
    )
    .unwrap();
    writeln!(f, "\tCodePoint::CRLF,").unwrap();
    for cp in &nl {
        writeln!(f, "\tCodePoint::Char({}),", char_literal(*cp)).unwrap();
    }
    writeln!(f, "];").unwrap();

    // MB_UNINTERPRETABLE_CODE_POINTS: all uninterpretable, sorted by codepoint
    let mut uninterp: Vec<u32> = entries
        .iter()
        .filter(|e| e.category == "uninterpretable" && !is_special(e))
        .map(|e| match &e.codepoint {
            CodepointValue::Int(n) => *n,
            _ => unreachable!(),
        })
        .collect();
    uninterp.sort();

    writeln!(
        f,
        "\npub const MB_UNINTERPRETABLE_CODE_POINTS: [char; MBUNINTERPRETABLECODEPOINTS_COUNT] = ["
    )
    .unwrap();
    for cp in &uninterp {
        writeln!(f, "\t{},", char_literal(*cp)).unwrap();
    }
    writeln!(f, "];").unwrap();
}

/// Only these 6 names appear in the existing `asciiReplacements()` lookup.
/// They must be emitted sorted by codepoint so binary search works.
const ASCII_REPLACEMENT_NAMES: &[&str] =
    &["Not", "Degree", "PlusMinus", "CenterDot", "Times", "Divide"];

fn generate_long_names_character_sets(out_dir: &Path, entries: &[LongName]) {
    let path = out_dir.join("long_names_character_sets.rs");
    let mut f = BufWriter::new(fs::File::create(path).unwrap());

    // Collect entries that belong to the ASCII replacements subset, sorted by codepoint.
    let mut subset: Vec<&LongName> = entries
        .iter()
        .filter(|e| ASCII_REPLACEMENT_NAMES.contains(&e.name.as_str()))
        .collect();
    subset.sort_by_key(|e| nominal_codepoint(e));

    writeln!(f, "pub static ASCII_REPLACEMENTS_MAP: &[(char, &[&str])] = &[").unwrap();
    for entry in &subset {
        let cname = const_name(&entry.name);
        let replacements: String = entry
            .ascii
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(f, "\t({cname}, [{replacements}, ].as_slice()),").unwrap();
    }
    writeln!(f, "];").unwrap();
}

fn generate_long_names_operators(out_dir: &Path, entries: &[LongName]) {
    let path = out_dir.join("long_names_operators.rs");
    let mut f = BufWriter::new(fs::File::create(path).unwrap());

    writeln!(f, "pub fn LongNameCodePointToOperator(c: char) -> TokenKind {{").unwrap();
    writeln!(f, "\tmatch c {{").unwrap();

    for entry in entries {
        if entry.category != "punctuation" {
            continue;
        }
        // Only non-special entries (actual chars) go in the match
        if is_special(entry) {
            continue;
        }
        let cname = const_name(&entry.name);
        // TokenKind variant: LongName_ + original name
        let variant = format!("LongName_{}", entry.name);
        writeln!(f, "\t\t{cname} => return {variant},").unwrap();
    }

    writeln!(f, "\t\t_ => panic!(\"Need to add operator\"),").unwrap();
    writeln!(f, "\t}}").unwrap();
    writeln!(f, "}}").unwrap();
}

fn generate_precedence_values(out_dir: &Path, entries: &[PrecedenceEntry]) {
    let path = out_dir.join("precedence_values.rs");
    let mut f = BufWriter::new(fs::File::create(path).unwrap());

    let mut counter: u32 = 0;
    let mut resolved: HashMap<String, u32> = HashMap::new();

    writeln!(f, "impl Precedence {{").unwrap();

    for entry in entries {
        match entry {
            PrecedenceEntry::Primary { name, value, assoc } => {
                let prec = match value {
                    PrecValue::Int(n) => {
                        counter = *n;
                        *n
                    }
                    PrecValue::Next(_) => {
                        counter += 1;
                        counter
                    }
                };
                let assoc_bit: u32 = if assoc == "right" { 1 } else { 0 };
                let encoded = (prec << 1) | assoc_bit;
                let assoc_name = if assoc == "right" {
                    "Associativity`Right"
                } else {
                    "Associativity`NonRight"
                };
                writeln!(
                    f,
                    "\tpub const {name}: Precedence = Precedence::new({encoded}); // prec: {prec}, assoc: {assoc_name}"
                )
                .unwrap();
                resolved.insert(name.clone(), encoded);
            }
            PrecedenceEntry::Alias { name, same_as } => {
                let encoded = *resolved
                    .get(same_as)
                    .unwrap_or_else(|| panic!("Alias {name} references unknown {same_as}"));
                let prec = encoded >> 1;
                let assoc_name = if encoded & 1 == 1 {
                    "Associativity`Right"
                } else {
                    "Associativity`NonRight"
                };
                writeln!(
                    f,
                    "\tpub const {name}: Precedence = Precedence::new({encoded}); // prec: {prec}, assoc: {assoc_name}"
                )
                .unwrap();
                resolved.insert(name.clone(), encoded);
            }
        }
    }

    writeln!(f, "}}").unwrap();
}

// ─── main ────────────────────────────────────────────────────────────

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let gen_dir = Path::new(&out_dir).join("generated");
    fs::create_dir_all(&gen_dir).unwrap();

    // Watch data files for changes
    println!("cargo:rerun-if-changed=data/long_names.json");
    println!("cargo:rerun-if-changed=data/precedence.json");

    // Load long names
    let long_names_json = fs::read_to_string("data/long_names.json")
        .expect("Failed to read data/long_names.json");
    let long_names: Vec<LongName> =
        serde_json::from_str(&long_names_json).expect("Failed to parse long_names.json");

    // Load precedence
    let prec_json = fs::read_to_string("data/precedence.json")
        .expect("Failed to read data/precedence.json");
    let precedence: Vec<PrecedenceEntry> =
        serde_json::from_str(&prec_json).expect("Failed to parse precedence.json");

    // Generate all files
    generate_long_names_constants(&gen_dir, &long_names);
    generate_long_names_mappings(&gen_dir, &long_names);
    generate_long_names_character_classes(&gen_dir, &long_names);
    generate_long_names_character_sets(&gen_dir, &long_names);
    generate_long_names_operators(&gen_dir, &long_names);
    generate_precedence_values(&gen_dir, &precedence);
}
