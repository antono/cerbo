#!/usr/bin/env python3
"""Fail if anything under core/src writes a file without going through fsio.

Every vault mutation must use `cerbo_core::fsio::write_atomic` — see the
durable-writes capability. `fsio.rs` is the one module allowed near the raw
APIs. Test modules are exempt: fixtures write files directly on purpose.

`core/clippy.toml` enforces the same rule during `cargo clippy`; this script is
the cheap, network-free version CI can run on its own.

Usage: check-atomic-writes.py [core/src directory]
"""

import re
import sys
from pathlib import Path

FORBIDDEN = re.compile(r"\b(?:std::)?fs::write\s*\(|\bFile::create\s*\(")
CFG_TEST = re.compile(r"^\s*#\[cfg\(test\)\]\s*$")
EXEMPT = {"fsio.rs"}


def strip_test_modules(lines):
    """Blank out every `#[cfg(test)] mod … { … }` block, keeping line numbers."""
    out = list(lines)
    i = 0
    while i < len(out):
        if not CFG_TEST.match(out[i]):
            i += 1
            continue
        # Walk forward to the module's opening brace, then match it.
        j = i
        while j < len(out) and "{" not in out[j]:
            j += 1
        if j >= len(out):
            break
        depth = 0
        while j < len(out):
            depth += out[j].count("{") - out[j].count("}")
            out[j] = ""
            j += 1
            if depth <= 0:
                break
        for k in range(i, j):
            out[k] = ""
        i = j
    return out


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "core/src")
    if not root.is_dir():
        print(f"check-atomic-writes: no such directory: {root}", file=sys.stderr)
        return 2

    offenders = []
    for path in sorted(root.rglob("*.rs")):
        if path.name in EXEMPT:
            continue
        lines = path.read_text().splitlines()
        for n, line in enumerate(strip_test_modules(lines), start=1):
            if FORBIDDEN.search(line):
                offenders.append(f"{path}:{n}: {line.strip()}")

    if offenders:
        print("Non-atomic vault writes found — use cerbo_core::fsio::write_atomic:",
              file=sys.stderr)
        for o in offenders:
            print(f"  {o}", file=sys.stderr)
        return 1

    print(f"check-atomic-writes: clean ({root})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
