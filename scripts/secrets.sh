#!/bin/bash
# scripts/secrets.sh — Management secrets pentru shop-mvp
# Folosește `age` (https://github.com/FiloSottile/age) pentru criptare.
#
# Setup inițial:
#   bash scripts/secrets.sh init
#
# Criptează .env:
#   bash scripts/secrets.sh encrypt
#
# Decriptează .env:
#   bash scripts/secrets.sh decrypt
#
# Rotire secret (parolă DB, JWT, Stripe key):
#   bash scripts/secrets.sh rotate <cheie>
# ============================================================

set -euo pipefail

SECRETS_DIR="${SECRETS_DIR:-$(dirname "$0")/../secrets}"
ENV_FILE="${ENV_FILE:-$(dirname "$0")/../.env}"
PUB_KEY="${SECRETS_DIR}/age.pub"
PRIV_KEY="${SECRETS_DIR}/age.key"

mkdir -p "$SECRETS_DIR"

case "${1:-help}" in
    init)
        if command -v age-keygen &>/dev/null; then
            echo "🔑 Generare cheie age..."
            age-keygen -o "$PRIV_KEY"
            echo "✅ Cheie privată: $PRIV_KEY"
            echo "✅ Cheie publică:  $(cat "$PUB_KEY" 2>/dev/null || true)"
        else
            echo "❌ age-keygen negăsit. Instalează: cargo install age"
            exit 1
        fi
        ;;

    encrypt)
        if [ ! -f "$PRIV_KEY" ]; then
            echo "❌ Rulează 'secrets.sh init' prima dată"
            exit 1
        fi
        PUB=$(cat "${PUB_KEY}")
        echo "🔒 Criptare .env..."
        age -r "$PUB" -o "${SECRETS_DIR}/.env.age" "$ENV_FILE"
        echo "✅ ${SECRETS_DIR}/.env.age creat"
        ;;

    decrypt)
        if [ ! -f "$PRIV_KEY" ]; then
            echo "❌ Cheie privată negăsită: $PRIV_KEY"
            exit 1
        fi
        if [ ! -f "${SECRETS_DIR}/.env.age" ]; then
            echo "❌ Fișier criptat negăsit: ${SECRETS_DIR}/.env.age"
            exit 1
        fi
        echo "🔓 Decriptare .env..."
        age -d -i "$PRIV_KEY" -o "$ENV_FILE" "${SECRETS_DIR}/.env.age"
        echo "✅ .env restaurat"
        ;;

    rotate)
        KEY="${2:-}"
        if [ -z "$KEY" ]; then
            echo "🔁 Rotire toate secretelor..."
            # Generează valori noi
            NEW_JWT=$(uuidgen 2>/dev/null || openssl rand -hex 32)
            NEW_DB_PASS=$(openssl rand -base64 16 2>/dev/null || echo "postgres:123123")
            NEW_STRIPE_KEY="sk_test_$(openssl rand -hex 32 2>/dev/null || echo 'placeholder')"

            # Actualizează .env
            sed -i "s/^JWT_SECRET=.*/JWT_SECRET=$NEW_JWT/" "$ENV_FILE"
            sed -i "s/^STRIPE_SECRET_KEY=.*/STRIPE_SECRET_KEY=$NEW_STRIPE_KEY/" "$ENV_FILE"

            echo "✅ Secrete rotite:"
            echo "   JWT_SECRET → $NEW_JWT"
            echo "   STRIPE_SECRET_KEY → $NEW_STRIPE_KEY"
            echo "⚠️  DB_PASSWORD nu a fost schimbată — necesită DB restart"
        else
            echo "🔁 Rotire $KEY..."
            case "$KEY" in
                jwt|JWT_SECRET)
                    NEW=$(openssl rand -hex 32)
                    sed -i "s/^JWT_SECRET=.*/JWT_SECRET=$NEW/" "$ENV_FILE"
                    echo "   JWT_SECRET → $NEW" ;;
                stripe|STRIPE_SECRET_KEY)
                    NEW="sk_test_$(openssl rand -hex 32)"
                    sed -i "s/^STRIPE_SECRET_KEY=.*/STRIPE_SECRET_KEY=$NEW/" "$ENV_FILE"
                    echo "   STRIPE_SECRET_KEY → $NEW" ;;
                db|DATABASE_URL)
                    NEW=$(openssl rand -base64 16)
                    sed -i "s|postgres:123123|postgres:$NEW|" "$ENV_FILE"
                    echo "   DATABASE_URL → parolă nouă (necesită DB restart)" ;;
                deepseek|DEEPSEEK_API_KEY)
                    echo "⚠️  DEEPSEEK_API_KEY nu poate fi rotit automat" ;;
                *)
                    echo "❌ Cheie necunoscută: $KEY" ;;
            esac
        fi
        bash "$0" encrypt
        ;;

    check)
        echo "🔍 Verificare secrets..."
        [ -f "$PRIV_KEY" ] && echo "✅ Cheie privată: prezentă" || echo "❌ Cheie privată: lipsă"
        [ -f "${SECRETS_DIR}/.env.age" ] && echo "✅ .env.age: prezent" || echo "❌ .env.age: lipsă"
        [ -f "$ENV_FILE" ] && echo "✅ .env: prezent" || echo "❌ .env: lipsă"
        # Verifică dacă .env conține parole în plaintext (warning)
        if grep -qi "secret\|password\|key" "$ENV_FILE" 2>/dev/null; then
            echo "⚠️  .env conține secrete în plaintext!"
        fi
        ;;

    help|*)
        echo "Folosire: bash scripts/secrets.sh <comandă>"
        echo ""
        echo "Comenzi:"
        echo "  init      Generează cheie age pentru criptare"
        echo "  encrypt   Criptează .env → secrets/.env.age"
        echo "  decrypt   Decriptează secrets/.env.age → .env"
        echo "  rotate    Rotire toate secretele (JWT, Stripe, DB)"
        echo "  rotate <key>  Rotire doar o cheie (jwt|stripe|db)"
        echo "  check     Verifică stare secrets"
        ;;
esac
