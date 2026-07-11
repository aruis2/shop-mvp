#!/usr/bin/env bash
# 🚀 Pornește shop-mvp în background cu debug maxim
# Folosire:
#   ./start.sh          → pornește în background
#   ./start.sh stop     → oprește
#   ./start.sh logs     → tail -f logurile
set -euo pipefail
cd "$(dirname "$0")"

PIDFILE="logs/shop-mvp.pid"
LOGFILE="logs/run-dev.log"

case "${1:-start}" in
    start)
        mkdir -p logs
        if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
            echo "⚠️  Serverul rulează deja (PID $(cat "$PIDFILE")). Folosește ./start.sh stop"
            exit 1
        fi
        echo "🚀 Pornesc shop-mvp în background..."
        RUST_LOG=debug RUST_BACKTRACE=full \
        cargo run -p shop-mvp >> "$LOGFILE" 2>&1 &
        PID=$!
        echo $PID > "$PIDFILE"
        echo "   PID: $PID"
        echo "   Log: $LOGFILE"
        echo "   Rulează: tail -f $LOGFILE"
        ;;
    stop)
        if [ -f "$PIDFILE" ]; then
            PID=$(cat "$PIDFILE")
            echo "🛑 Oprește PID $PID..."
            kill -9 "$PID" 2>/dev/null || true
            rm -f "$PIDFILE"
            echo "   Oprit."
        else
            echo "⚠️  Niciun PID salvat. Încerc pkill..."
            pkill -f "shop-mvp" 2>/dev/null || echo "   Niciun proces găsit."
        fi
        ;;
    logs)
        exec tail -f "$LOGFILE" 2>/dev/null || echo "📭 Nu există $LOGFILE. Rulează ./start.sh"
        ;;
    status)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
            echo "✅ Serverul rulează (PID $(cat "$PIDFILE"))"
        else
            echo "❌ Serverul NU rulează"
        fi
        ;;
    *)
        echo "Folosire: $0 {start|stop|logs|status}"
        exit 1
        ;;
esac
