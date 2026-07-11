#!/bin/bash
# seed-articles.sh — Inserează articolele din articles/ în baza de date
# Folosește PostgreSQL dollar-quoting (\$\$...\$\$) pentru a evita problemele cu apostrofuri.
# Rulează: bash scripts/seed-articles.sh

set -euo pipefail

DB_URL="${DATABASE_URL:-postgresql://postgres:123123@localhost:5432/test}"
ARTICLES_DIR="articles"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🌱 Seeding articles into knowledge base..."
echo "📂 DB: $DB_URL"
echo ""

# Funcție: extrage o valoare YAML
extract_yaml() {
    local file="$1" key="$2"
    awk -v k="$key" '
        /^---$/ {c++; next}
        c==1 && index($0, k ":")==1 {
            sub(/^[^:]*:[[:space:]]*["\x27]?/, "", $0)
            sub(/["\x27]?[[:space:]]*$/, "", $0)
            print; exit
        }
        c>1 {exit}
    ' "$file"
}

# Funcție: extrage conținut după frontmatter
extract_content() {
    local file="$1"
    awk 'c>=2 {print} /^---$/ {c++}' "$file"
}

make_slug() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g;s/--*/-/g;s/^-//;s/-$//'
}

calc_reading_time() {
    echo $(( ($(echo "$1" | wc -w) + 199) / 200 ))
}

count=0
for file in "$PROJECT_DIR/$ARTICLES_DIR"/standard-*.md; do
    basename_file=$(basename "$file")
    echo "📄 $basename_file"

    title=$(extract_yaml "$file" "title")
    descr=$(extract_yaml "$file" "description" || true)
    content=$(extract_content "$file")
    slug=$(make_slug "$title")
    rt=$(calc_reading_time "$content")
    diff="intermediate"
    tags="{standard,security}"
    echo "$basename_file" | grep -qi "psd2" && tags="{standard,security,payments}"
    echo "$basename_file" | grep -qi "cis" && tags="{standard,security,controls}"
    echo "$basename_file" | grep -qi "iso" && tags="{standard,security,management}"
    echo "$basename_file" | grep -qi "eidas" && tags="{standard,security,identity}"
    echo "$basename_file" | grep -qi "wcag" && tags="{standard,accessibility,security}"
    echo "$basename_file" | grep -qi "api" && tags="{standard,security,api}"
    cat="{standard,security}"

    # Scriem SQL-ul într-un fișier temporar și-l executăm cu psql
    # Folosim dollar-quoting (\$\$...\$\$) pentru a evita escaping-ul caracterelor speciale
    # Notă: 'SQL' cu ghilimele previne expandarea variabilelor de bash ($$ → PID)
    tmpfile=$(mktemp /tmp/seed-article-XXXXXX.sql)
    cat > "$tmpfile" <<'ENDSQL'
        INSERT INTO articles (title, slug, summary, content, category_path, tags, difficulty, reading_time_minutes)
        VALUES (
            $$TITLE_PLACEHOLDER$$,
            'SLUG_PLACEHOLDER',
            $$SUMMARY_PLACEHOLDER$$,
            $$CONTENT_PLACEHOLDER$$,
            'CAT_PLACEHOLDER',
            'TAGS_PLACEHOLDER',
            'DIFF_PLACEHOLDER',
            RT_PLACEHOLDER
        )
        ON CONFLICT (slug) DO UPDATE
        SET title = EXCLUDED.title,
            summary = EXCLUDED.summary,
            content = EXCLUDED.content,
            updated_at = NOW();
ENDSQL

    # Înlocuim placeholder-ele cu valorile reale (după ce am scris fișierul)
    sed -i "s|TITLE_PLACEHOLDER|${title//\$/\\$}|g" "$tmpfile"
    sed -i "s|SUMMARY_PLACEHOLDER|${descr//\$/\\$}|g" "$tmpfile"
    # Pentru content, scriem direct cu printf în fișier
    printf '%s\n' "$content" | sed 's/[\&/]/\\&/g' > /tmp/seed-content-$$.txt
    sed -i '/CONTENT_PLACEHOLDER/{
        r /tmp/seed-content-$$.txt
        d
    }' "$tmpfile"
    sed -i "s|SLUG_PLACEHOLDER|$slug|g" "$tmpfile"
    sed -i "s|CAT_PLACEHOLDER|$cat|g" "$tmpfile"
    sed -i "s|TAGS_PLACEHOLDER|$tags|g" "$tmpfile"
    sed -i "s|DIFF_PLACEHOLDER|$diff|g" "$tmpfile"
    sed -i "s|RT_PLACEHOLDER|$rt|g" "$tmpfile"

    psql "$DB_URL" -q -v ON_ERROR_STOP=1 -f "$tmpfile"
    rm -f "$tmpfile" /tmp/seed-content-$$.txt

    echo "   ✅ slug=$slug ($rt min)"
    count=$((count + 1))
done

echo ""
echo "🎉 $count articole inserate/actualizate!"
