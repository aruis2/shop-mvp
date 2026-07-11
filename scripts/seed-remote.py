#!/usr/bin/env python3
"""seed-remote.py — Inserează articolele standard lipsă pe S22 (senmut.org DB).
Pipe: python3 scripts/seed-remote.py | ssh -p 8022 u0_a481@192.168.1.4 "PGPASSWORD=123123 psql -h localhost -U postgres -d test"
"""

import re
import sys
from pathlib import Path

ARTICLES_DIR = Path(__file__).resolve().parent.parent / "articles"


def extract_yaml(filepath: Path, key: str) -> str | None:
    with open(filepath, encoding="utf-8") as f:
        lines = f.readlines()
    in_fm = False
    for line in lines:
        s = line.strip()
        if s == "---":
            if not in_fm:
                in_fm = True
                continue
            else:
                break
        if in_fm and s.startswith(f"{key}:"):
            val = s[len(key) + 1:].strip().strip('"').strip("'")
            return val
    return None


def extract_content(filepath: Path) -> str:
    with open(filepath, encoding="utf-8") as f:
        content = f.read()
    content = re.sub(r"^---\n.*?\n---\n", "", content, count=1, flags=re.DOTALL)
    return content.strip()


# Articolele de inserat (slug-ul e numele fișierului fără .md)
ARTICLES = [
    "standard-psd2-sca",
    "standard-iso-27001",
    "standard-eidas",
    "standard-wcag-21",
    "standard-owasp-api-top-10",
]

print("BEGIN;")

for slug in ARTICLES:
    filepath = ARTICLES_DIR / f"{slug}.md"
    if not filepath.exists():
        print(f"-- ⚠️  {slug}.md not found", file=sys.stderr)
        continue

    title = extract_yaml(filepath, "title") or slug
    description = extract_yaml(filepath, "description") or ""
    content = extract_content(filepath)
    reading_time = max(1, (len(content.split()) + 199) // 200)
    tags = "{standard,security}"

    if "psd2" in slug:
        tags = "{standard,security,payments}"
    elif "iso" in slug:
        tags = "{standard,security,management}"
    elif "eidas" in slug:
        tags = "{standard,security,identity}"
    elif "wcag" in slug:
        tags = "{standard,accessibility,security}"
    elif "api" in slug:
        tags = "{standard,security,api}"

    cat = "{standard,security}"
    diff = "intermediate"

    # Folosim dollar-quoting cu tag unic
    print(f"""
INSERT INTO articles (title, slug, summary, content, category_path, tags, difficulty, reading_time_minutes)
VALUES (
    $body${title}$body$,
    '{slug}',
    $body${description}$body$,
    $body${content}$body$,
    '{cat}',
    '{tags}',
    '{diff}',
    {reading_time}
)
ON CONFLICT (slug) DO UPDATE
SET title = EXCLUDED.title,
    summary = EXCLUDED.summary,
    content = EXCLUDED.content,
    updated_at = NOW();
-- ✅ {slug}
""")

print("COMMIT;")
