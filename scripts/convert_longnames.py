#!/usr/bin/env python3
"""Convert CodeParser/Data/LongNames.wl to data/long_names.json"""

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
WL_FILE = Path.home() / "Documents/codeparser/CodeParser/Data/LongNames.wl"

CATEGORY_MAP = {
    "RawCharacter": "raw",
    "LetterlikeCharacter": "letterlike",
    "PunctuationCharacter": "punctuation",
    "WhitespaceCharacter": "whitespace",
    "NewlineCharacter": "newline",
    "UninterpretableCharacter": "uninterpretable",
    "UnsupportedCharacter": "unsupported",
}

EXTRA_MAP = {
    "NotStrangeLetterlike": "not_strange_letterlike",
    "Special": "special",
    "System": "system",
    "comparison": "comparison",
    "arithmetic": "arithmetic",
    "logical": "logical",
    "group": "group",
}

# Special codepoints that map to CodePoint enum variants, not chars
SPECIAL_CODEPOINTS = {
    "RawDoubleQuote": "StringMeta_DoubleQuote",
    "RawBackslash": "StringMeta_Backslash",
}


def parse_wl_file(path: Path) -> list[dict]:
    text = path.read_text()

    # Extract the association body between <| and |>
    m = re.search(r"<\|(.*)\|>", text, re.DOTALL)
    if not m:
        raise ValueError("Could not find <| ... |> in WL file")
    body = m.group(1)

    entries = []
    # Match each entry: "Name" -> {Category, 16^^XXXX, <| metadata |>}
    pattern = re.compile(
        r'"(\w+)"\s*->\s*\{(\w+),\s*(16\^\^[0-9a-fA-F]+),\s*<\|(.*?)\|>\}',
        re.DOTALL,
    )

    for m in pattern.finditer(body):
        name = m.group(1)
        category_wl = m.group(2)
        codepoint_str = m.group(3)
        metadata_str = m.group(4).strip()

        category = CATEGORY_MAP.get(category_wl)
        if category is None:
            raise ValueError(f"Unknown category: {category_wl} for {name}")

        # Parse codepoint: 16^^XXXX -> integer
        hex_val = codepoint_str.replace("16^^", "")
        codepoint_int = int(hex_val, 16)

        entry: dict = {"name": name, "category": category}

        # Handle special codepoints
        if name in SPECIAL_CODEPOINTS:
            entry["codepoint"] = SPECIAL_CODEPOINTS[name]
        else:
            entry["codepoint"] = codepoint_int

        # Parse metadata
        if metadata_str:
            # Parse ASCIIReplacements
            ascii_match = re.search(
                r'"ASCIIReplacements"\s*->\s*\{(.*?)\}', metadata_str
            )
            if ascii_match:
                ascii_str = ascii_match.group(1)
                # Extract quoted strings, handling escaped quotes
                replacements = re.findall(r'"((?:[^"\\]|\\.)*)"', ascii_str)
                # Unescape
                replacements = [r.replace('\\"', '"') for r in replacements]
                entry["ascii"] = replacements

            # Parse Extra
            extra_match = re.search(r'"Extra"\s*->\s*\{(.*?)\}', metadata_str)
            if extra_match:
                extra_str = extra_match.group(1)
                extra_values = re.findall(r'"(\w+)"', extra_str)
                mapped = []
                for v in extra_values:
                    mapped_v = EXTRA_MAP.get(v)
                    if mapped_v is None:
                        raise ValueError(f"Unknown Extra value: {v} for {name}")
                    mapped.append(mapped_v)
                entry["extra"] = mapped

            # Parse Added
            added_match = re.search(
                r'"Added"\s*->\s*([0-9.]+)', metadata_str
            )
            if added_match:
                entry["added"] = added_match.group(1)

        entries.append(entry)

    return entries


def main():
    if not WL_FILE.exists():
        print(f"Error: WL file not found: {WL_FILE}", file=sys.stderr)
        sys.exit(1)

    entries = parse_wl_file(WL_FILE)
    print(f"Parsed {len(entries)} entries from {WL_FILE}", file=sys.stderr)

    output = REPO / "data" / "long_names.json"
    with open(output, "w") as f:
        json.dump(entries, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"Wrote {output}", file=sys.stderr)


if __name__ == "__main__":
    main()
