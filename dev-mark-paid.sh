#!/usr/bin/env bash
# =============================================================================
# 🔧 dev-mark-paid.sh — Marchează o comandă ca plătită (dev mode)
# =============================================================================
# Stripe nu poate trimite webhook-uri la localhost.
# Folosește asta DUPĂ ce ai plătit pe Stripe pentru a marca comanda.
#
# Folosire:
#   1. Plătești pe Stripe → ești redirecționat înapoi la /orders
#   2. Rulezi: bash dev-mark-paid.sh
#      → găsește ultima comandă unpaid și o marchează paid
#
#   Sau manual cu Order ID:
#     bash dev-mark-paid.sh <ORDER_UUID>
# =============================================================================

BASE="${1:-http://localhost:3001}"

if [ -n "$2" ]; then
    ORDER_ID="$2"
else
    echo "🔍 Caut ultima comandă unpaid..."
    ORDERS=$(curl -s -b /tmp/cookies.txt "$BASE/orders" 2>/dev/null | grep -oP 'order_id[" :]+\K[a-f0-9-]+' | head -3)
    if [ -z "$ORDERS" ]; then
        echo "❌ Nu am găsit comenzi. Loghează-te întâi."
        echo "   Folosește: bash dev-mark-paid.sh <ORDER_ID>"
        echo "   ORDER_ID îl găsești în pagina /orders"
        exit 1
    fi
    ORDER_ID=$(echo "$ORDERS" | head -1)
    echo "   Găsit: $ORDER_ID"
fi

echo "📦 Marchează comanda $ORDER_ID ca plătită..."

# Simulează webhook-ul Stripe direct
# 🔑 Headere: Origin (CSRF) + stripe-signature (skip verify când STRIPE_WEBHOOK_SECRET ne setat)
curl -s -X POST "$BASE/stripe/webhook" \
  -H "Content-Type: application/json" \
  -H "Origin: $BASE" \
  -H "stripe-signature: t=1,v1=dev_mode" \
  -H "x-stripe-webhook-type: checkout.session.completed" \
  -d "{
    \"type\": \"checkout.session.completed\",
    \"data\": {
        \"object\": {
            \"id\": \"manual_dev_$(date +%s)\",
            \"metadata\": {
                \"order_id\": \"$ORDER_ID\"
            }
        }
    }
}" 2>&1

echo ""
echo "✅ Comanda $ORDER_ID marcată ca plătită!"
echo "   Vezi: $BASE/orders"
