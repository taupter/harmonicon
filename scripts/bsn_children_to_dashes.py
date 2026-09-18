#!/usr/bin/env python3
"""Rewrite `bsn!` child lists from Bevy 0.19's shape to Bevy 0.20's.

Bevy 0.20 deprecated both the parentheses that grouped one child's
components and the commas that separated siblings:

    Children [                    Children [
        (                             Text("a")
            Text("a")                 TextColor(RED)
            TextColor(RED)            --
        ),               =>           Text("b")
        (                             TextColor(BLUE)
            Text("b")             ]
            TextColor(BLUE)
        ),
    ]

Both still compile in 0.20, but only with a deprecation warning — and CI
runs `cargo clippy --all-targets -- -D warnings`, so they have to go.

Splitting on commas is only correct at the child list's own nesting depth:
a comma inside `TextFont { font_size: px(15.0), .. }`, inside a call's
argument list, or inside a nested `Children [ ... ]` is not a separator.
So this walks the bracket tracking depth and skipping string/char
literals, rather than pattern-matching text.

Usage: scripts/bsn_children_to_dashes.py [--check] <file.rs>...
"""

import re
import sys

OPEN = {"(": ")", "[": "]", "{": "}"}
CLOSE = {v: k for k, v in OPEN.items()}
CHAR_LITERAL = re.compile(r"'(?:\\.|[^\\'])'")


def _skip_literal(s: str, i: int) -> int:
    """Index just past a string or char literal starting at `s[i]`.

    Returns `i` unchanged when `s[i]` doesn't open one — notably for `'`,
    which starts a lifetime (`'a`) far more often than a char here.
    """
    if s[i] == '"':
        j = i + 1
        while j < len(s) and s[j] != '"':
            j += 2 if s[j] == "\\" else 1
        return j
    if s[i] == "'":
        m = CHAR_LITERAL.match(s, i)
        if m:
            return m.end() - 1
    return i


def scan_balanced(s: str, start: int) -> int:
    """Index just past the delimiter opened at `start`."""
    depth = 0
    i = start
    while i < len(s):
        c = s[i]
        i = _skip_literal(s, i)
        if s[i] == c:
            if c in OPEN:
                depth += 1
            elif c in CLOSE:
                depth -= 1
                if depth == 0:
                    return i + 1
        i += 1
    raise ValueError(f"unbalanced delimiter opened at {start}")


def split_entries(body: str) -> list[str]:
    """Split a child list body at its own depth-0 commas."""
    entries, depth, last = [], 0, 0
    i = 0
    while i < len(body):
        c = body[i]
        i = _skip_literal(body, i)
        if body[i] == c:
            if c in OPEN:
                depth += 1
            elif c in CLOSE:
                depth -= 1
            elif c == "," and depth == 0:
                entries.append(body[last:i])
                last = i + 1
        i += 1
    entries.append(body[last:])
    return entries


def dedent(text: str, columns: int) -> str:
    """Remove up to `columns` leading spaces from every line."""
    out = []
    for line in text.split("\n"):
        removable = min(columns, len(line) - len(line.lstrip(" ")))
        out.append(line[removable:])
    return "\n".join(out)


def strip_group_parens(entry: str) -> str:
    """Unwrap `( ... )` around a whole child, preserving its indentation."""
    lead = entry[: len(entry) - len(entry.lstrip())]
    rest = entry[len(lead) :].rstrip()
    if not (rest.startswith("(") and rest.endswith(")")):
        return entry
    # Only when those parens wrap the *entire* entry: `(a) (b)` must stay,
    # and a call like `foo(x)` never reaches here (it doesn't start with a
    # paren) but `(foo)(bar)` would.
    if scan_balanced(rest, 0) != len(rest):
        return entry
    inner = rest[1:-1]
    if "\n" not in inner:
        return lead + inner.strip()
    # `lead` already supplies the first line's indentation, so drop what
    # dedent left on it.
    return lead + dedent(inner.strip("\n"), 4).lstrip(" ")


def child_indent(entry: str) -> str:
    """The leading whitespace of `entry`'s first non-blank line.

    The `--` separators line up with the children they sit between, so a
    reader sees one column of siblings rather than a ragged list.
    """
    for line in entry.split("\n"):
        if line.strip():
            return line[: len(line) - len(line.lstrip(" "))]
    return ""


def rewrite_body(body: str) -> str:
    """Rewrite one child list's contents (everything inside its `[ ]`)."""
    entries = split_entries(body)
    # A trailing comma leaves an empty final entry; that comma is a
    # separator to drop, not a child.
    if len(entries) > 1 and not entries[-1].strip():
        entries = entries[:-1]
    if not entries:
        return body
    rebuilt = [transform(strip_group_parens(e)) for e in entries]
    if len(rebuilt) == 1:
        inner = rebuilt[0].rstrip()
    else:
        sep = f"\n{child_indent(rebuilt[0])}--"
        inner = sep.join(r.rstrip() for r in rebuilt)
    return inner + body[len(body.rstrip()) :]


def transform(src: str) -> str:
    """Rewrite every `Children [ ... ]` in `src`, recursing into nested ones."""
    out, i = [], 0
    for m in re.finditer(r"\bChildren\s*\[", src):
        if m.start() < i:
            continue  # already consumed as part of an enclosing list
        open_at = src.index("[", m.start())
        end = scan_balanced(src, open_at)
        out.append(src[i : open_at + 1])
        out.append(rewrite_body(src[open_at + 1 : end - 1]))
        i = end - 1
    out.append(src[i:])
    return "".join(out)


def main() -> int:
    args = sys.argv[1:]
    check = "--check" in args
    paths = [a for a in args if a != "--check"]
    if not paths:
        print(__doc__)
        return 2
    dirty = 0
    for path in paths:
        with open(path) as f:
            src = f.read()
        new = transform(src)
        if transform(new) != new:
            print(f"ERROR: {path}: rewrite is not idempotent, leaving it alone")
            return 3
        if new == src:
            continue
        dirty += 1
        if check:
            print(f"would rewrite {path}")
        else:
            with open(path, "w") as f:
                f.write(new)
            print(f"rewrote {path}")
    if not dirty:
        print("no bsn child lists needed rewriting")
    return 1 if (check and dirty) else 0


if __name__ == "__main__":
    sys.exit(main())
