#!/usr/bin/env bash
# 🚀 Rulează shop-mvp în mod debug maxim
set -euo pipefail
cd "$(dirname "$0")"
RUST_LOG=debug RUST_BACKTRACE=full cargo run -p shop-mvp 2>&1 | tee logs/run-dev.log
