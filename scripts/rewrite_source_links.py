#!/usr/bin/env python3
"""Rewrite source file references in wiki docs to GitHub permalinks."""

import re
from pathlib import Path

REPO_URL = "https://github.com/l1veIn/dora-manager/blob/main"

# Patterns that match source file links
SOURCE_DIRS = r"(?:crates|nodes|web|tests|scripts|docs|\.github)"
ROOT_FILES = r"(?:Cargo\.toml|dev\.sh|rust-toolchain\.toml|simulate_clean_install\.sh|README\.md|README_zh\.md|PROJECT_CONSTITUTION\.md|CHANGELOG\.md|CLAUDE\.md|registry\.json)"

# Match [text](path) where path is a source file reference
link_re = re.compile(
    r'\[([^\]]*)\]\(((?:en/)?(?:' + SOURCE_DIRS + r')/[^\)]+)\)|'
    r'\[([^\]]*)\]\(((?:en/)?(?:' + ROOT_FILES + r'))(?:#[^\)]*)?\)'
)

def rewrite_link(match: re.Match) -> str:
    # Get the path from whichever group matched
    if match.group(2):
        text, path = match.group(1), match.group(2)
    else:
        text, path = match.group(3), match.group(4)

    # Strip en/ prefix
    if path.startswith("en/"):
        path = path[3:]

    # Keep line number anchor if present
    anchor = ""
    if "#" in path:
        path, anchor = path.split("#", 1)
        anchor = f"#{anchor}"

    gh_url = f"{REPO_URL}/{path}{anchor}"
    return f"[{text}]({gh_url})"


def process_file(filepath: Path) -> int:
    content = filepath.read_text(encoding="utf-8")
    new_content, count = link_re.subn(rewrite_link, content)
    if count > 0:
        filepath.write_text(new_content, encoding="utf-8")
    return count


def main():
    total = 0
    for md_file in sorted(Path("wiki/zh").rglob("*.md")):
        n = process_file(md_file)
        if n:
            print(f"  {md_file.relative_to('wiki/zh')}: {n} links rewritten")
            total += n
    print(f"\nTotal: {total} source links rewritten to GitHub permalinks")


if __name__ == "__main__":
    main()
