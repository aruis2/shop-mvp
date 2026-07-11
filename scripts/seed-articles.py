#!/usr/bin/env python3
"""seed-articles.py — Inserează articolele din articles/ în baza de date.
Folosește psql cu dollar-quoting pentru a evita problemele de escaping.

Rulează: python3 scripts/seed-articles.py
"""

import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://postgres:123123@localhost:5432/test",
)
ARTICLES_DIR = Path(__file__).resolve().parent.parent / "articles"


def extract_yaml(filepath: Path, key: str) -> str | None:
    """Extrage o valoare YAML dintre frontmatter-ul unui fișier .md."""
    with open(filepath, encoding="utf-8") as f:
        lines = f.readlines()

    in_frontmatter = False
    for line in lines:
        stripped = line.strip()
        if stripped == "---":
            if not in_frontmatter:
                in_frontmatter = True
                continue
            else:
                break
        if in_frontmatter and stripped.startswith(f"{key}:"):
            val = stripped[len(key) + 1:].strip().strip('"').strip("'")
            return val
    return None


def extract_content(filepath: Path) -> str:
    """Extrage conținutul după frontmatter."""
    with open(filepath, encoding="utf-8") as f:
        content = f.read()
    # Elimină frontmatter-ul (primul --- ... ---)
    content = re.sub(r"^---\n.*?\n---\n", "", content, count=1, flags=re.DOTALL)
    return content.strip()


def make_slug(title: str) -> str:
    """Generează slug din titlu."""
    slug = title.lower()
    slug = re.sub(r"[^a-z0-9-]", "-", slug)
    slug = re.sub(r"-+", "-", slug)
    slug = slug.strip("-")
    return slug


def calc_reading_time(content: str) -> int:
    """Calculează timpul de citire în minute (200 cuvinte/minut)."""
    words = len(content.split())
    return max(1, (words + 199) // 200)


def tags_for_file(basename: str) -> str:
    """Determină tag-urile pe baza numelui fișierului."""
    name = basename.lower()
    if "psd2" in name:
        return "{standard,security,payments}"
    if "cis" in name:
        return "{standard,security,controls}"
    if "iso" in name:
        return "{standard,security,management}"
    if "eidas" in name:
        return "{standard,security,identity}"
    if "wcag" in name:
        return "{standard,accessibility,security}"
    if "api" in name:
        return "{standard,security,api}"
    return "{standard,security}"


def main() -> int:
    print("🌱 Seeding articles into knowledge base...")
    print(f"📂 DB: {DB_URL}")
    print()

    files = sorted(ARTICLES_DIR.glob("standard-*.md"))
    count = 0

    for filepath in files:
        basename = filepath.name
        print(f"📄 {basename}")

        title = extract_yaml(filepath, "title")
        description = extract_yaml(filepath, "description") or ""
        content = extract_content(filepath)
        if not title:
            print(f"   ⚠️  Fără titlu, skip")
            continue

        slug = make_slug(title)
        rt = calc_reading_time(content)
        tags = tags_for_file(basename)
        cat = "{standard,security}"
        diff = "intermediate"

        # Construim SQL cu dollar-quoting
        sql = f"""INSERT INTO articles (title, slug, summary, content, category_path, tags, difficulty, reading_time_minutes)
VALUES (
    $article_body${title}$article_body$,
    '{slug}',
    $article_body${description}$article_body$,
    $article_body${content}$article_body$,
    '{cat}',
    '{tags}',
    '{diff}',
    {rt}
)
ON CONFLICT (slug) DO UPDATE
SET title = EXCLUDED.title,
    summary = EXCLUDED.summary,
    content = EXCLUDED.content,
    updated_at = NOW();
"""

        # Scriem SQL-ul într-un fișier temporar
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".sql", delete=False, encoding="utf-8"
        ) as tmp:
            tmp.write(sql)
            tmp_path = tmp.name

        try:
            result = subprocess.run(
                ["psql", DB_URL, "-q", "-v", "ON_ERROR_STOP=1", "-f", tmp_path],
                capture_output=True,
                text=True,
            )
            if result.returncode != 0:
                print(f"   ❌ Eroare: {result.stderr.strip()}")
                return 1
            print(f"   ✅ slug={slug} ({rt} min)")
            count += 1
        finally:
            os.unlink(tmp_path)

    print(f"\n🎉 {count} articole inserate/actualizate!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
