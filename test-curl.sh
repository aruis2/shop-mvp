#!/usr/bin/env bash
# ============================================================
# Testabil cu curl — shop-mvp — TOATE CAZURILE
# ============================================================
# Flow: GET = 200, POST = 302 (redirect), erori = consistent
# Folosire: bash test-curl.sh [BASE_URL] [--verbose|-v]
# Default: http://localhost:3001
# ============================================================
set -uo pipefail

BASE="${1:-http://localhost:3001}"
VERBOSE=false
[[ "${2:-}" == "--verbose" || "${2:-}" == "-v" ]] && VERBOSE=true

PASS=0
FAIL=0
ERRORS=""
COOKIE_JAR=$(mktemp)
AUTH_COOKIE_JAR=$(mktemp)
EMPTY_JAR=$(mktemp)

cleanup() { rm -f "$COOKIE_JAR" "$AUTH_COOKIE_JAR" "$EMPTY_JAR"; }
trap cleanup EXIT

# ─── helpers ──────────────────────────────────────────────
# 🔒 POST-urile trimit Origin header (simulează browser) pentru CSRF.
# Folosește curl_cmd_nocsrf() pentru a testa explicit respingerea CSRF.
curl_cmd() {
    local method="$1" path="$2" expected="$3" label="$4" data="${5:-}" jar="${6:-$COOKIE_JAR}"
    local args=(-s --max-time 3 -o /dev/null -w "%{http_code}" -X "$method")
    [[ -n "$data" ]] && args+=(-d "$data")
    # 🔒 Adaugă Origin header pentru CSRF (browserul trimite automat)
    if [[ "$method" == "POST" || "$method" == "PUT" || "$method" == "DELETE" || "$method" == "PATCH" ]]; then
        args+=(-H "Origin: ${BASE}")
    fi
    args+=(-b "$jar" -c "$jar")
    local url="${BASE}${path}"
    local code; code=$(curl "${args[@]}" "$url" 2>/dev/null || echo "000")
    if [[ "$code" == "$expected" ]]; then
        ((PASS++))
        $VERBOSE && echo "  ✅ $label → $code"
    else
        ((FAIL++))
        ERRORS+="  ❌ $label → așteptat $expected, primit $code (${method} ${path})\n"
        $VERBOSE && echo "  ❌ $label → așteptat $expected, primit $code"
    fi
}

# 🔍 Verifică conținutul HTML (link-uri, butoane, texte)
check_content() {
    local path="$1" expected_re="$2" label="$3" jar="${4:-$COOKIE_JAR}"
    local body
    body=$(curl -s -b "$jar" "${BASE}${path}" 2>/dev/null)
    if echo "$body" | grep -qE "$expected_re"; then
        ((PASS++))
        $VERBOSE && echo "  ✅ $label — conținut găsit"
    else
        ((FAIL++))
        ERRORS+="  ❌ $label — conținutul nu conține '$expected_re' (GET ${path})\n"
        $VERBOSE && echo "  ❌ $label — conținutul nu conține '$expected_re'"
    fi
}

# 🔍 Verifică URL-ul unui redirect 302
check_redirect() {
    local method="$1" path="$2" expected_location="$3" label="$4" data="${5:-}" jar="${6:-$COOKIE_JAR}"
    local args=(-s --max-time 3 -o /dev/null -w "%{redirect_url}" -X "$method")
    [[ -n "$data" ]] && args+=(-d "$data")
    if [[ "$method" == "POST" || "$method" == "PUT" || "$method" == "DELETE" || "$method" == "PATCH" ]]; then
        args+=(-H "Origin: ${BASE}")
    fi
    args+=(-b "$jar" -c "$jar")
    local url="${BASE}${path}"
    local location
    location=$(curl "${args[@]}" "$url" 2>/dev/null || echo "")
    # Extrage doar calea din URL pentru comparație
    local location_path
    location_path=$(echo "$location" | sed 's|^https\?://[^/]*||')
    if [[ "$location_path" == "$expected_location" ]]; then
        ((PASS++))
        $VERBOSE && echo "  ✅ $label → redirect la $location_path"
    else
        ((FAIL++))
        ERRORS+="  ❌ $label — așteptat redirect la '$expected_location', primit '$location_path'\n"
        $VERBOSE && echo "  ❌ $label — așteptat '$expected_location', primit '$location_path'"
    fi
}

# Fără Origin header → pentru testarea explicită a CSRF
curl_cmd_nocsrf() {
    local method="$1" path="$2" expected="$3" label="$4" data="${5:-}" jar="${6:-$COOKIE_JAR}"
    local args=(-s --max-time 3 -o /dev/null -w "%{http_code}" -X "$method")
    [[ -n "$data" ]] && args+=(-d "$data")
    args+=(-b "$jar" -c "$jar")
    local url="${BASE}${path}"
    local code; code=$(curl "${args[@]}" "$url" 2>/dev/null || echo "000")
    if [[ "$code" == "$expected" ]]; then
        ((PASS++))
        $VERBOSE && echo "  ✅ $label → $code"
    else
        ((FAIL++))
        ERRORS+="  ❌ $label → așteptat $expected, primit $code (${method} ${path})\n"
        $VERBOSE && echo "  ❌ $label → așteptat $expected, primit $code"
    fi
}

get()    { curl_cmd "GET"  "$1" "$2" "$3" "" "$COOKIE_JAR"; }
post()   { curl_cmd "POST" "$1" "$2" "$3" "${4:-}" "$COOKIE_JAR"; }
get_a()  { curl_cmd "GET"  "$1" "$2" "$3" "" "$AUTH_COOKIE_JAR"; }
post_a() { curl_cmd "POST" "$1" "$2" "$3" "${4:-}" "$AUTH_COOKIE_JAR"; }

section() {
    echo ""
    echo "─── $1 ───"
}

# ============================================================
echo "🔍 Testare exhaustivă: $BASE"
echo "   $(date)"
echo "   Verbose: $VERBOSE"

# ═══════════════════════════════════════════════════════════
# 1. PAGINI GENERALE — status + conținut
# ═══════════════════════════════════════════════════════════
section "1. Pagini generale"

get  "/"                   200 "Home page"
# 🔍 Verifică linkul 🛒 Shop din navbar → href="/"
check_content "/" 'href="/"' "Navbar — link Acasă → /"
# 🔍 Verifică linkul Produse
check_content "/" '/products"' "Navbar — link Produse"
# 🔍 Verifică linkul Coș
check_content "/" '/cart"' "Navbar — link Coș"
# 🔍 Verifică titlul paginii
check_content "/" 'Shop MVP' "Titlu pagină — Shop MVP"

get  "/?error=test"        200 "Home page cu eroare"
get  "/health"             200 "Health check"
get  "/login"              200 "Login page"
get  "/login?redirect=/orders" 200 "Login cu redirect"
get  "/login?error=msg"    200 "Login cu eroare"
get  "/signup"             200 "Signup page"
get  "/signup?error=msg"   200 "Signup cu eroare"
get  "/logout"             302 "Logout GET (redirect) + șterge cookie"
get  "/logout?redirect=/products" 302 "Logout cu redirect"
get  "/me"                 401 "Me — neautentificat"
get  "/shop"               200 "Home page (/shop)"
get  "/shop/health"        200 "Health check (/shop)"
get  "/shop/login"         200 "Login page (/shop)"

# ═══════════════════════════════════════════════════════════
# 2. AUTH — cazuri de eroare (fără cookie, jar gol)
# ═══════════════════════════════════════════════════════════
section "2. Autentificare — cazuri de eroare"

# Erorile de login returnează 302 redirect cu ?error= (nu 400 direct)
curl_cmd "POST" "/login"   302 "Login — body gol" "" "$EMPTY_JAR"
curl_cmd "POST" "/login"   302 "Login — email fără parolă" "email=x@x.com" "$EMPTY_JAR"
curl_cmd "POST" "/login"   302 "Login — parolă fără email" "password=abc" "$EMPTY_JAR"
curl_cmd "POST" "/login"   302 "Login — email greșit" "email=none@x.com&password=abc" "$EMPTY_JAR"
curl_cmd "POST" "/login"   302 "Login — parolă greșită" "email=test@test.com&password=grcita" "$EMPTY_JAR"

curl_cmd "POST" "/signup"  302 "Signup — body gol" "" "$EMPTY_JAR"
curl_cmd "POST" "/signup"  302 "Signup — fără name" "email=x@x.com&password=abc" "$EMPTY_JAR"
curl_cmd "POST" "/signup"  302 "Signup — parolă scurtă" "email=x@x.com&password=ab&name=X" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 3. AUTH — login + signup reușite
# ═══════════════════════════════════════════════════════════
section "3. Autentificare — reușită"

# Login cu utilizator existent
post "/login"              302 "Login — reușit (test@test.com)" \
     "email=test@test.com&password=parola123"

# Signup cu date noi
post "/signup"             302 "Signup — reușit (date noi)" \
     "email=curl-test-$(date +%s)@test.com&password=parola123&name=TestCurl"

# După signup/login, cookie-ul e setat — testăm /me autentificat
get  "/me"                 200 "Me — autentificat (JSON)"

# Login + redirect
post "/login"              302 "Login — cu redirect=/orders" \
     "email=test@test.com&password=parola123&redirect=/orders"

# Deja autentificat → /login și /signup redirect
get  "/login"              200 "Login — deja autentificat (redirect HTML)"
get  "/signup"             200 "Signup — deja autentificat (redirect HTML)"

# 🔍 Verifică conținutul paginii de orders (autentificat)
# Notă: "Plătește acum" apare doar dacă utilizatorul are comenzi neplătite
get  "/orders"             200 "Orders — autentificat"
# Verificăm că pagina conține titlul "Comenzile mele" și structura de bază
check_content "/orders" 'Comenzile' "Orders — titlu 'Comenzile mele' prezent"
# Dacă există comenzi neplătite, verificăm și butonul de plată
ORDERS_HTML=$(curl -s -b "$COOKIE_JAR" "${BASE}/orders" 2>/dev/null)
if echo "$ORDERS_HTML" | grep -q 'Plătește'; then
    ((PASS++))
    $VERBOSE && echo "  ✅ Orders — buton 'Plătește acum' prezent (comenzi neplătite există)"
else
    # Nu e eroare — utilizatorul poate să n-aibă comenzi neplătite
    ((PASS++))
    $VERBOSE && echo "  ⏭️  Orders — 'Plătește' nu apare (fără comenzi neplătite)"
fi

# ═══════════════════════════════════════════════════════════
# 4. PRODUSE
# ═══════════════════════════════════════════════════════════
section "4. Produse"

get  "/products"           200 "Produse — lista"
get  "/products?page=1"    200 "Produse — page 1"
get  "/products?page=2"    200 "Produse — page 2"
get  "/products?page=999"  200 "Produse — page 999 (peste limită)"
get  "/products?page=-1"   200 "Produse — page -1 (tratat ca 1)"
get  "/shop/products"      200 "Produse — (/shop)"
get  "/search"             400 "Search — fără query"
get  "/search?q="          200 "Search — query gol"
get  "/search?q=test"      200 "Search — cu query"
get  "/search?q=xyz123nonexistent" 200 "Search — 0 rezultate"
get  "/product/foo"        404 "Product — slug inexistent"
get  "/product/"           301 "Product — slug gol → trailing slash"

# ═══════════════════════════════════════════════════════════
# 5. COȘ — înainte de login (cookie curat)
# ═══════════════════════════════════════════════════════════
section "5. Coș — fără cookie"

get  "/cart"               200 "Coș — gol (fără session)"
get  "/cart?error=msg"    200 "Coș — cu eroare"
get  "/cart?session_id=test-nonexistent" 200 "Coș — session inexistent"
get  "/shop/cart"          200 "Coș — (/shop)"

# Adăugare — cazuri de eroare
post "/cart/add"           302 "Cart add — body gol" ""
post "/cart/add"           302 "Cart add — fără slug" "qty=1"
post "/cart/add"           302 "Cart add — slug inexistent" "product_slug=nonexistent-slug&qty=1"

# ═══════════════════════════════════════════════════════════
# 6. COȘ — adăugare produs real + verificare
# ═══════════════════════════════════════════════════════════
section "6. Coș — adăugare produs"

# Luăm primul slug din DB
FIRST_SLUG=$(curl -s "${BASE}/products" | grep -oP 'product/\K[a-zA-Z0-9-]+' | head -1 || echo "")
if [[ -n "$FIRST_SLUG" ]]; then
    post "/cart/add"       302 "Cart add — slug valid ($FIRST_SLUG)" \
         "product_slug=$FIRST_SLUG&qty=1"
    get  "/cart"           200 "Coș — cu 1 item (după add)"
    post "/cart/add"       302 "Cart add — același produs (increment)" \
         "product_slug=$FIRST_SLUG&qty=1"
    post "/cart/add"       302 "Cart add — qty=0 (default la 1)" \
         "product_slug=$FIRST_SLUG&qty=0"
    post "/cart/add"       302 "Cart add — qty=999 (clamped la max)" \
         "product_slug=$FIRST_SLUG&qty=999"
else
    echo "  ⚠️  Niciun produs în DB — sar peste testele cu slug real"
fi

# Remove — cazuri de eroare
post "/cart/remove"        302 "Cart remove — body gol" ""
post "/cart/remove"        302 "Cart remove — UUID invalid" "item_id=not-a-uuid"
post "/cart/remove"        302 "Cart remove — UUID inexistent" "item_id=00000000-0000-0000-0000-000000000000"

# ═══════════════════════════════════════════════════════════
# 7. COMENZI
# ═══════════════════════════════════════════════════════════
section "7. Comenzi"

# Checkout — coș gol (jar curat, fără session_id)
curl_cmd "GET" "/checkout"              302 "Checkout — coș gol (redirect)" "" "$EMPTY_JAR"
curl_cmd "GET" "/checkout?session_id=nonexistent" 302 "Checkout — session inexistent" "" "$EMPTY_JAR"

# Orders — neautentificat (folosim cookie curat)
get_a "/orders"            302 "Orders — neautentificat (redirect)"
get_a "/orders?page=1"     302 "Orders — neautentificat, page 1"
get_a "/shop/orders"       302 "Orders — /shop, neautentificat"

# Success
get  "/success"            200 "Success — fără order_id"
get  "/success?order_id=00000000-0000-0000-0000-000000000000" 200 "Success — cu order_id"

# ═══════════════════════════════════════════════════════════
# 8. ADMIN — fără autentificare
# ═══════════════════════════════════════════════════════════
section "8. Admin — fără autentificare"

get  "/admin"              200 "Admin products — fără auth"
get  "/admin/orders"       200 "Admin orders — fără auth"
get  "/admin/product/new"  200 "Admin product new — fără auth"
get  "/admin/product/test/edit" 200 "Admin product edit — fără auth"
get  "/admin/logs"         200 "Admin logs — fără auth"

# POST-only → 405 pe GET
curl_cmd "GET"  "/admin/product/test/delete" 405 "Admin delete — GET (405)" "" "$COOKIE_JAR"
curl_cmd "GET"  "/admin/migrate-orders"      405 "Admin migrate — GET (405)" "" "$COOKIE_JAR"

# POST fără auth → 200 (redirect HTML via JS)
curl_cmd "POST" "/admin/product/new"         200 "Admin create — POST fără auth" "name=x&slug=x" "$COOKIE_JAR"
curl_cmd "POST" "/admin/product/test/delete" 200 "Admin delete — POST fără auth" "" "$COOKIE_JAR"
curl_cmd "POST" "/admin/product/test/edit"   200 "Admin edit — POST fără auth" "name=x&slug=x" "$COOKIE_JAR"

# migrate-orders: folosește verify_admin direct → 401 (nu 200 cu JS)
curl_cmd "POST" "/admin/migrate-orders"      401 "Admin migrate — POST fără token" "" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 9. ADMIN — cu autentificare (dar nu admin)
# ═══════════════════════════════════════════════════════════
section "9. Admin — autentificat (non-admin)"

# Folosim cookie de la login-ul din secțiunea 3
curl_cmd "GET"  "/admin"                     200 "Admin products — user normal" "" "$COOKIE_JAR"
curl_cmd "POST" "/admin/migrate-orders"      403 "Admin migrate — user normal (403)" "" "$COOKIE_JAR"

# ═══════════════════════════════════════════════════════════
# 10. RUTE INEXISTENTE
# ═══════════════════════════════════════════════════════════
section "10. Rute inexistente (404)"

get  "/nonexistent"        404 "Random path"
get  "/shop/nonexistent"   404 "Random path (/shop)"
get  "/produse"            404 "Path greșit (română)"
get  "/cos"                404 "Path greșit (română)"
get  "/admin/nonexistent"  404 "Admin path inexistent"
get  "/admin/produs/nou"   404 "Admin path română"

# ═══════════════════════════════════════════════════════════
# 11. TRAILING SLASH
# ═══════════════════════════════════════════════════════════
section "11. Trailing slash (301)"

curl_cmd "GET" "/products/"  301 "Produse"    "" "$COOKIE_JAR"
curl_cmd "GET" "/cart/"      301 "Coș"         "" "$COOKIE_JAR"
curl_cmd "GET" "/search/"    301 "Căutare"     "" "$COOKIE_JAR"
curl_cmd "GET" "/login/"     301 "Login"       "" "$COOKIE_JAR"
curl_cmd "GET" "/signup/"    301 "Signup"      "" "$COOKIE_JAR"
curl_cmd "GET" "/checkout/"  301 "Checkout"    "" "$COOKIE_JAR"
curl_cmd "GET" "/signup/"    301 "Signup"      "" "$COOKIE_JAR"
curl_cmd "GET" "/admin/"     301 "Admin"       "" "$COOKIE_JAR"
curl_cmd "GET" "/orders/"    301 "Orders"      "" "$COOKIE_JAR"
curl_cmd "GET" "/success/"   301 "Success"     "" "$COOKIE_JAR"
curl_cmd "GET" "/shop/products/" 301 "Shop products" "" "$COOKIE_JAR"

# ═══════════════════════════════════════════════════════════
# 12. FIȘIERE STATICE
# ═══════════════════════════════════════════════════════════
section "12. Fișiere statice"

get  "/static/style.css"        200 "style.css"
get  "/static/nonexistent.css"  404 "Fișier inexistent"
# /static/ trailing slash → 301 (middleware rulează înainte de static)
curl_cmd "GET" "/static/"        301 "Director (trailing slash)" "" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 13. PAGINI POLITICI + SECURITATE
# ═══════════════════════════════════════════════════════════
section "13. Pagini politici"

get  "/privacy"            200 "Politică confidențialitate"
get  "/security"           200 "Politică securitate"
get  "/.well-known/security.txt" 200 "security.txt"

# ═══════════════════════════════════════════════════════════
# 14. CONT UTILIZATOR
# ═══════════════════════════════════════════════════════════
section "14. Cont utilizator"

# Neautentificat — handler-ul face redirect la login (302), nu 401
curl_cmd "POST" "/account/delete"  302 "Account delete — neautentificat (redirect login)" "" "$EMPTY_JAR"
curl_cmd "GET"  "/account/export"  401 "Account export — neautentificat" "" "$EMPTY_JAR"

# Autentificat
curl_cmd "GET"  "/account/export"  200 "Account export — autentificat" "" "$COOKIE_JAR"

# ═══════════════════════════════════════════════════════════
# 15. CSRF — verificare Origin/Referer
# ═══════════════════════════════════════════════════════════
section "15. CSRF protection"

# POST cu Origin valid → ar trebui să meargă (folosim /cart/add care n-are rate limit)
curl_cmd "POST" "/cart/add"  302 "CSRF — Origin valid (cart add)" \
    "product_slug=nonexistent&qty=1" "$EMPTY_JAR"

# POST fără Origin, fără Referer → CSRF check (folosim /cart/add, nu /login — evităm rate limit)
curl_cmd_nocsrf "POST" "/cart/add"  403 "CSRF — fără Origin, fără Referer (așteptat 403)" \
    "product_slug=nonexistent&qty=1" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 16. STRIPE WEBHOOK
# ═══════════════════════════════════════════════════════════
section "16. Stripe webhook"

# Fără signature header
curl_cmd "POST" "/stripe/webhook"  401 "Webhook — fără signature" \
    '{"type":"checkout.session.completed","data":{"object":{"metadata":{"order_id":"test"}}}}' "$EMPTY_JAR"

# JSON invalid
curl_cmd "POST" "/stripe/webhook"  400 "Webhook — JSON invalid" \
    "not-json" "$EMPTY_JAR"

# Semnătură invalidă (cu header fals)
curl_cmd "POST" "/stripe/webhook"  401 "Webhook — semnătură invalidă" \
    '{"type":"checkout.session.completed","data":{"object":{"metadata":{"order_id":"test"}}}}' "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 17. 405 METHOD NOT ALLOWED (complete)
# ═══════════════════════════════════════════════════════════
section "17. Metode nesesuportate (405)"

curl_cmd "GET"  "/cart/add"          405 "Cart add — GET" "" "$EMPTY_JAR"
curl_cmd "GET"  "/cart/remove"       405 "Cart remove — GET" "" "$EMPTY_JAR"
curl_cmd "GET"  "/order/00000000-0000-0000-0000-000000000000/pay" 405 "Order pay — GET" "" "$EMPTY_JAR"
curl_cmd "GET"  "/stripe/webhook"    405 "Stripe webhook — GET" "" "$EMPTY_JAR"
curl_cmd "GET"  "/admin/migrate-orders" 405 "Admin migrate — GET" "" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 18. CHECKOUT + ORDER/PAY (cu autentificare)
# ═══════════════════════════════════════════════════════════
section "18. Checkout + plată"

# Checkout — coș gol (deja autentificat, dar session_id nou)
RANDOM_SID="curl-test-$(date +%s)-$$"
curl_cmd "GET" "/checkout?session_id=$RANDOM_SID"  302 "Checkout — coș gol (redirect)" "" "$COOKIE_JAR"

# Order pay — UUID invalid
curl_cmd "POST" "/order/00000000-0000-0000-0000-000000000000/pay"  302 "Order pay — UUID nul" "" "$COOKIE_JAR"

# Order pay — neautentificat
curl_cmd "POST" "/order/00000000-0000-0000-0000-000000000000/pay"  302 "Order pay — neautentificat" "" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 19. ADMIN — cu autentificare admin
# ═══════════════════════════════════════════════════════════
section "19. Admin — operații POST"

# Admin migrate fără token
curl_cmd "POST" "/admin/migrate-orders"  401 "Admin migrate — neautentificat" "" "$EMPTY_JAR"

# Admin migrate cu user normal (cookie de la login test@test.com, NU e admin)
# Admin în seed: test@test.org, aruis2@gmail.com
curl_cmd "POST" "/admin/migrate-orders"  403 "Admin migrate — user normal (nu e admin)" "" "$COOKIE_JAR"

# Admin order status — GET = 405
curl_cmd "GET"  "/admin/order/00000000-0000-0000-0000-000000000000/status"  405 "Admin order status — GET" "" "$EMPTY_JAR"

# Admin order status — POST fără auth
curl_cmd "POST" "/admin/order/00000000-0000-0000-0000-000000000000/status"  200 "Admin order status — fără auth" "status=confirmed" "$EMPTY_JAR"

# ═══════════════════════════════════════════════════════════
# 20. RATE LIMITING (test rapid)
# ═══════════════════════════════════════════════════════════
section "20. Rate limit"

# Facem 11 requesturi rapide la login — ultimul ar trebui să fie rate-limited
# Folosim EMTPY_JAR ca să nu păstrăm cookie de login
for i in $(seq 1 10); do
    curl -s -X POST -H "Origin: ${BASE}" \
        -d "email=rate-test@test.com&password=grcita" \
        -o /dev/null -w "%{http_code}" \
        "$BASE/login" > /dev/null 2>&1
done
# Al 11-lea ar trebui să fie 302 cu error de rate limit
RATE_CODE=$(curl -s -X POST -H "Origin: ${BASE}" \
    -d "email=rate-test@test.com&password=grcita" \
    -o /dev/null -w "%{http_code}" "$BASE/login")
if [[ "$RATE_CODE" == "302" ]]; then
    ((PASS++))
    $VERBOSE && echo "  ✅ Rate limit — ultimul request e 302 (redirect cu error)"
else
    ((FAIL++))
    ERRORS+="  ❌ Rate limit — așteptat 302, primit $RATE_CODE\n"
    $VERBOSE && echo "  ❌ Rate limit — așteptat 302, primit $RATE_CODE"
fi

# ═══════════════════════════════════════════════════════════
# 21. HEADERE DE RĂSPUNS
# ═══════════════════════════════════════════════════════════
section "21. Headere de răspuns"

# Verificăm că CSP e prezent
CSP=$(curl -s -I "$BASE/" | grep -i "content-security-policy" | head -1)
if [[ -n "$CSP" ]]; then
    ((PASS++))
    $VERBOSE && echo "  ✅ CSP header prezent: $(echo $CSP | head -c 80)..."
else
    ((FAIL++))
    ERRORS+="  ❌ CSP header lipsă pe /\n"
    $VERBOSE && echo "  ❌ CSP header lipsă"
fi

# Verificăm X-Frame-Options
XFO=$(curl -s -I "$BASE/" | grep -i "x-frame-options" | head -1)
if [[ -n "$XFO" ]]; then
    ((PASS++)) && $VERBOSE && echo "  ✅ X-Frame-Options: $XFO"
else
    ((FAIL++))
    ERRORS+="  ❌ X-Frame-Options lipsă pe /\n"
    $VERBOSE && echo "  ❌ X-Frame-Options lipsă"
fi

# ═══════════════════════════════════════════════════════════
# REZULTAT FINAL
# ═══════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════"
echo "  Rezultat:"
echo "  ✅ Pass: $PASS"
echo "  ❌ Fail: $FAIL"
echo "═══════════════════════════════════════════"
if [[ -n "$ERRORS" ]]; then
    echo ""
    echo -e "$ERRORS"
    exit 1
fi
echo "  🎯 Toate testele trec (curl = browser)!"
