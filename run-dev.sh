#!/usr/bin/env bash
# 🚀 Rulează shop-mvp în mod debug maxim
set -euo pipefail
cd "$(dirname "$0")"

# 🔨 Construiește CSS (zero JS, Tailwind static)
if command -v npx &>/dev/null && [ -f package.json ]; then
    npx @tailwindcss/cli -i shop-mvp/static/tailwind-input.css -o shop-mvp/static/style.css --minify 2>/dev/null
fi

RUST_LOG=debug RUST_BACKTRACE=full cargo run -p shop-mvp 2>&1 | tee logs/run-dev.log
