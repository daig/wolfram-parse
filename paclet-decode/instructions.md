

Based on the source code in [https://github.com/s-macke/mathematica-encode/blob/master/mcm_encode.py](https://github.com/s-macke/mathematica-encode/blob/master/mcm_encode.py), which implements the encoding process for Mathematica's Encode function, the decoding algorithm is the reverse of that logic. The script uses a fixed Huffman table (a dictionary mapping characters to bitstrings) optimized for Wolfram Language code, Huffman compresses the plain text, pads the bitstream if necessary, and then encodes groups of 13 bits into pairs of printable ASCII characters using base-95 with an offset of 33 (characters from '!' = 33 to '~' = 126, giving 95 symbols).

The header "(_!1N!_)" indicates version 1 and table variant 'N', which corresponds to the table in the script. The body is the encoded data.

To implement the decoder in Rust (or any language), follow these specific steps. The engineer should open the mcm_encode.py file and extract the Huffman table (the huff dictionary) from it—it's a Python dict like huff = {' ': '0010', '\n': '0011', ...} covering ASCII characters relevant to code (e.g., letters, digits, symbols like '(', ')', '=', etc.). This table is crucial and must be hardcoded in the Rust code as a map from char to &str (bitstring).

### 1. Read and Validate the File

- Load the entire file as a string (assume UTF-8/ASCII, as it's printable).
- Check that the first 8 characters are exactly "(_!1N!_)". If not, error out (wrong format or variant).
- Extract the body as the substring from index 8 to the end.

### 2. Convert Body to Bitstream

- The body consists of pairs of characters (even length; if odd, it's invalid or truncated).
- Each pair encodes 13 bits.
- Define BASE = 95, OFFSET = 33.
- Initialize an empty string or Vec<bool> for the bitstream.</bool>
- For each pair (index i = 0 to body.len()/2 - 1):
    - c1 = body.char_at(2*i)
    - c2 = body.char_at(2*i + 1)
    - If c1 or c2 is not in 33-127, error (invalid char).
    - a = c1 as u32 - OFFSET
    - b = c2 as u32 - OFFSET
    - If a >= BASE or b >= BASE, error.
    - value = a * BASE + b // u32, max 94*95 + 94 = 9024
    - Convert value to a 13-bit binary string, padded with leading zeros if needed: format value as binary with width 13 (MSB first).
        - In Rust: let bit_str = format!("{:013b}", value);
    - Append these 13 bits to the bitstream (as string "010101..." or as bits).

Note: The encoding packs bits MSB first, so decoding does the same.

### 3. Build the Reverse Huffman Mapping

- From the huff dict in mcm_encode.py, create a reverse mapping: bitstring (String) -> char.
- In Rust, use a HashMap<String, char>.
- For each entry in huff (char: bitstr), insert (bitstr.to_string(), char) into the map.
- Alternatively (recommended for efficiency and to handle prefix codes), build a binary trie (tree):
    - Struct Node { left: Option<Box<node>>, right: Option&#x3C;Box<node>>, val: Option<char> }</char></node></node>
    - For each huff entry:
        - Start at root.
        - For each bit in bitstr:
            - If '0', go/create left.
            - If '1', go/create right.
        - Set val = char at leaf.
- This ensures prefix-free decoding.

### 4. Decode the Bitstream

- Initialize an output string.
- If using HashMap:
    - Initialize current_code as empty String.
    - For each bit in the bitstream (as '0' or '1'):
        - current_code += bit;
        - If current_code is in the reverse map:
            - output += reverse_map[&current_code]
            - current_code = ""
    - At the end, if current_code is not empty, it may be padding zeros; ignore (or error if strict).
- If using tree:
    - Start at root.
    - For each bit in bitstream:
        - If bit == '0', move to left; else to right.
        - If no child, error (invalid bitstream).
        - If after move, current.val is Some(ch):
            - output += ch
            - Reset current to root.
    - At end, if not at root, ignore padding.
- The output is the decoded plain text .m code.

### 5. Edge Cases and Validation

- Padding: Encoding pads the bitstream with zeros to multiple of 13 bits, so decoding may have trailing bits that don't form a code—stop when no more valid paths or end of stream.
- Empty file: After header, empty body decodes to empty string.
- Invalid chars or values > 2^13-1 (8191), but since 95^2=9025 > 8192, some values may represent >13 bits, but in practice, encoding ensures value < 8192 (by padding), so value can be up to 8191.
    - If value >= 8192, it might be invalid; check and error.
- Table completeness: The huff table from the script covers all needed chars; if a code is missing during encoding, it uses a fallback (check script for any escape mechanisms, but typically it's fixed for common chars).
- Newline and special chars: The table includes \n (10), \r (13), etc.
- Performance: For large files, use BitVec or byte array for bitstream; tree is O(1) per bit.

### Rust-Specific Implementation Tips

- Use std::collections::HashMap for reverse map, or implement a Trie struct.
- For bitstream, use bitvec crate (add to Cargo.toml: bitvec = "1") for BitVec<u8, Msb0>.
- Read file: use std::fs::read_to_string.
- Hardcode the huff table: let huff: &[(&char, &str)] = &[(&' ', "0010"), (&'\n', "0011"), ...]; // Copy from mcm_encode.py
- Then build reverse: let mut reverse_huff = HashMap::new(); for &(ch, bits) in huff { reverse_huff.insert(bits.to_string(), *ch); }
- Or for tree: define struct HuffmanNode { ... } as in previous pseudocode.
- Error handling: Use Result<String, String> for the decode function.

### Deriving or Verifying the Table

If the table in mcm_encode.py is not directly usable or for verification:

- Use chosen-plaintext: In Mathematica, Encode a known string (e.g., "a b\n"), get the encoded file, then adjust the table if needed.
- The table is fixed for 'N', and the script matches it via reverse engineering.

This reverses the encoding exactly as per the script. Test with an encoded file from Mathematica: decode and compare to original. If the variant matches 'N', it should work; for other letters, a different table may be needed (the repo may have multiple scripts).