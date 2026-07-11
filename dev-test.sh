#!/usr/bin/env bash
# 🚀 Build + restart + test într-un singur pas
# Folosire: bash dev-test.sh           # rulează testele comportamentale
#           bash dev-test.sh unit      # rulează testele unitare
#           bash dev-test.sh all       # rulează toate testele
set -euo pipefail
cd "$(dirname "$0")"

echo "🔨 Build..."
cargo build -p shop-mvp 2>&1 | tail -3

echo "🛑 Oprește serverul vechi..."
kill $(lsof -ti :3001) 2>/dev/null || true
sleep 1

echo "🚀 Pornește serverul nou..."
RUST_LOG=info cargo run -p shop-mvp 2>&1 &
SERVER_PID=$!

# Așteaptă să fie gata
for i in $(seq 1 10); do
    sleep 1
    if curl -s -o /dev/null -w "" http://localhost:3001/ 2>/dev/null; then
        echo "✅ Server pornit (PID $SERVER_PID)"
        break
    fi
    if [ "$i" = "10" ]; then
        echo "❌ Serverul nu a pornit în 10s"
        exit 1
    fi
done

if [ "${1:-}" = "unit" ]; then
    echo "🧪 Rulez teste unitare..."
    cargo test -p shop-mvp 2>&1 | tail -5
elif [ "${1:-}" = "all" ]; then
    echo "🧪 Rulez toate testele..."
    cargo test -p shop-mvp 2>&1 | tail -5
    echo ""
    bash test-behavior.sh 2>&1
    echo ""
    bash test-curl.sh 2>&1
else
    echo "🧪 Rulez teste comportamentale..."
    bash test-behavior.sh 2>&1
fi
