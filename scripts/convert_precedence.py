#!/usr/bin/env python3
"""Convert CodeParser/Data/Precedence.wl to data/precedence.json"""

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
WL_FILE = Path.home() / "Documents/codeparser/CodeParser/Data/Precedence.wl"


def wl_name_to_rust(wl_name: str) -> str:
    """Convert Precedence`Foo`Bar to SCREAMING_SNAKE_CASE Rust constant name.

    Examples:
        Precedence`Comma -> COMMA
        Precedence`LongName`InvisibleComma -> LONGNAME_INVISIBLECOMMA
        Precedence`Prefix`Bang -> PREFIX_BANG
        Precedence`Class`SetRelations -> CLASS_SETRELATIONS
        Precedence`Fake`EqualDot -> FAKE_EQUALDOT
        Precedence`Infix`Plus -> INFIX_PLUS
        Precedence`Infix`LongName`PlusMinus -> INFIX_LONGNAME_PLUSMINUS
    """
    # Remove the "Precedence`" prefix
    name = wl_name.replace("Precedence`", "")
    # Split on backtick
    parts = name.split("`")
    # Join with underscore and uppercase
    return "_".join(p.upper() for p in parts)


def parse_wl_file(path: Path) -> list[dict]:
    text = path.read_text()

    # Extract the association body between <| and |>
    m = re.search(r"<\|(.*)\|>", text, re.DOTALL)
    if not m:
        raise ValueError("Could not find <| ... |> in WL file")
    body = m.group(1)

    entries = []

    # Strip all WL comments (including multi-line) before line-by-line parsing
    body = re.sub(r"\(\*.*?\*\)", "", body, flags=re.DOTALL)

    for line in body.split("\n"):
        line = line.strip()
        if not line:
            continue
        # Remove trailing comma
        line = line.rstrip(",")
        if not line:
            continue

        # Try primary: Precedence`Name -> {value, Associativity`Dir}
        m = re.match(
            r"(Precedence`[\w`]+)\s*->\s*\{(\w+),\s*Associativity`(\w+)\}",
            line,
        )
        if m:
            wl_name = m.group(1)
            value_str = m.group(2)
            assoc_wl = m.group(3)

            rust_name = wl_name_to_rust(wl_name)

            if value_str == "Next":
                value = "next"
            else:
                value = int(value_str)

            assoc = "right" if assoc_wl == "Right" else "non_right"

            entries.append(
                {"name": rust_name, "value": value, "assoc": assoc}
            )
            continue

        # Try alias: Precedence`Name -> Precedence`OtherName
        m = re.match(
            r"(Precedence`[\w`]+)\s*->\s*(Precedence`[\w`]+)", line
        )
        if m:
            wl_name = m.group(1)
            target_wl = m.group(2)

            rust_name = wl_name_to_rust(wl_name)
            target_name = wl_name_to_rust(target_wl)

            entries.append({"name": rust_name, "same_as": target_name})
            continue

    return entries


def main():
    if not WL_FILE.exists():
        print(f"Error: WL file not found: {WL_FILE}", file=sys.stderr)
        sys.exit(1)

    entries = parse_wl_file(WL_FILE)
    print(f"Parsed {len(entries)} entries from {WL_FILE}", file=sys.stderr)

    output = REPO / "data" / "precedence.json"
    with open(output, "w") as f:
        json.dump(entries, f, indent=2)
        f.write("\n")

    print(f"Wrote {output}", file=sys.stderr)


if __name__ == "__main__":
    main()
