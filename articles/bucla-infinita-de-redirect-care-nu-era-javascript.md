# Bucla Infinită de Redirect Care Nu Era JavaScript

## Călătoria de Debugging Care Ne-a Costat Ore

### Context

Construiam o aplicație web Rust+Axum cu zero JavaScript. HTMX fusese eliminat. Totul era randare server-side cu form POST + 302 redirect (pattern PRG). Autentificarea era prin JWT în cookie HttpOnly, verificată server-side.

Într-o zi, un utilizator a raportat că după ce s-a autentificat cu un cont non-admin și a încercat să acceseze `/admin`, a rămas blocat într-o buclă infinită de redirect. Tab-ul browserului se învârtea la nesfârșit între `/login?redirect=/admin` și `/admin`.

### Căutarea

Am petrecut ore căutând bug-ul. Unde am căutat?

- **În Network tab-ul browserului** — vedeam 302-uri, script redirect-uri, cookie setat, cookie șters
- **În fișierele JavaScript** — poate `window.location.replace()` cauza problema?
- **În headerele HTMX** — poate `hx-redirect` interfera?
- **În script-ul de auth client-side** — poate `localStorage` era corupt?

Eram convinși că problema e pe client. La urma urmei, redirect-ul se întâmpla în browser. Serverul "doar trimitea răspunsuri."

### Cauza Reală

Bug-ul era în `verify_or_redirect()` din `admin.rs`:

```rust
async fn verify_or_redirect(...) -> Result<User, Html<String>> {
    verify_admin(headers, q, auth).await
        .map_err(|_| render_admin_redirect(bp, &rp))
}
```

`verify_admin` întoarce două erori diferite:
- `UNAUTHORIZED` — utilizatorul nu are niciun token
- `FORBIDDEN` — utilizatorul are token dar rolul lui nu este "admin"

Dar `verify_or_redirect` trata **ambele erori identic**: `|_|` — tiparul wildcard. Ambele cazuri duceau la un redirect către `/login?redirect=/admin`.

### Bucla

1. Utilizatorul cu `role = "user"` navighează la `/admin`
2. `verify_or_redirect` detectează token-ul, cheamă `verify_admin`
3. `verify_admin` vede `user.role != "admin"`, întoarce `FORBIDDEN`
4. `verify_or_redirect` prinde eroarea cu `|_|` și redirectează la `/login?redirect=/admin`
5. Utilizatorul este DEJA autentificat (are un token valid), așa că pagina de login detectează asta și îl trimite înapoi la `/admin`
6. Pasul 2 — buclă infinită

### De Ce N-am Putut Găsi Bug-ul

Există cinci motive distincte pentru care acest bug era invizibil pentru abordarea noastră de debugging:

#### 1. Redirect-ul Era un Script, Nu un HTTP Redirect

Paginile de admin foloseau `render_admin_redirect()` care întoarce:

```html
<script>window.location.replace('/login?redirect=/admin');</script>
```

Ăsta NU este un 302 redirect. Este o pagină HTML cu JavaScript care rulează în browser. Network tab arată un răspuns 200 OK (HTML-ul), apoi script-ul rulează și navighează. Între pasul 4 și pasul 5, nu există niciun HTTP redirect pe care să-l vezi în Network tab — este o navigare client-side declanșată de JavaScript.

#### 2. Pagina de Login Are Propriul Ei Redirect

Când pagina de login (handler-ul `login_page`) este încărcată și utilizatorul are deja un token valid, întoarce imediat:

```rust
if s.auth.verify_token(token).await.is_ok() {
    let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
    return Ok(redirect_html(&dest));
}
```

Asta trimite utilizatorul înapoi la `/admin`. Deci fluxul era:

```
/admin → [script redirect] → /login?redirect=/admin → [serverul detectează auth, redirectează] → /admin → [script redirect] → ...
```

Redirect-ul paginii de login este un 302 FOUND legitim, care ESTE vizibil în Network tab. Dar arată ca un "ești deja autentificat, iată redirect-ul tău" normal — nu ca o eroare.

#### 3. Eroarea Era Tăcută — Fără Log, Fără Stack Trace

`verify_admin` întoarce `Err((StatusCode::FORBIDDEN, "Admin: acces interzis"))`. Dar `verify_or_redirect` înghite asta cu `map_err(|_| ...)`. `_` aruncă la gunoi atât codul de status cât și mesajul de eroare. Nu există `tracing::warn!()`, niciun log, nimic.

Dacă am fi adăugat o singură linie:

```rust
.map_err(|(status, msg)| {
    tracing::warn!("verify_admin a eșuat: {} {}", status, msg);
    render_admin_redirect(bp, &rp)
})
```

Am fi văzut `FORBIDDEN Admin: acces interzis` în log-urile serverului imediat.

#### 4. Ținta Redirect-ului Era Aceeași cu Sursa Erorii

În multe bug-uri de buclă de redirect, bucla implică URL-uri diferite (A → B → C → A). Aici, bucla era:

```
/admin → /login?redirect=/admin → /admin → /login?redirect=/admin → ...
```

Doar DOUĂ URL-uri. Fiecare URL părea legitim:
- `/admin` — o pagină validă pe care ai putea dori s-o vizitezi
- `/login?redirect=/admin` — o pagină de login validă cu un parametru de redirect

Niciun URL nu avea un mesaj de eroare. Nu exista `?error=` în URL. Pentru browser, asta era o secvență perfect normală de încărcări de pagini.

#### 5. Modelul Mental Era Greșit

Gândeam problema în termeni de "autentificare" (ai un token valid?) când problema reală era "autorizare" (ai rolul potrivit?).

- `UNAUTHORIZED` (401) = "Nu știu cine ești" → redirect la login ✅
- `FORBIDDEN` (403) = "Știu cine ești, dar nu ai voie aici" → ar trebui să redirecționeze la home cu eroare, NU la login

Acestea sunt concepte fundamental diferite. Dar codul le trata la fel.

### Fix-ul

```rust
match verify_admin(headers, q, auth).await {
    Ok(user) => Ok(user),
    Err((status, msg)) => {
        if status == StatusCode::FORBIDDEN {
            // Autentificat dar nu admin → home cu eroare, nu login (previne bucla)
            let dest = format!("{}/?error={}", bp, msg.replace(' ', "%20"));
            Err(Html(format!("<script>window.location.replace('{dest}');</script>")))
        } else {
            // Neautentificat → redirect la login
            Err(render_admin_redirect(bp, &rp))
        }
    }
}
```

### Lecții Învățate

1. **`|_|` este un miros de cod în handling-ul de erori** — mai ales când potrivești pe `Result<T, E>` unde `E` poartă informație. Întotdeauna loghează sau inspectează eroarea înainte s-o arunci.

2. **Redirect-urile prin script sunt invizibile la debugging de rețea** — dacă serverul întoarce un 200 cu un `<script>window.location.replace(...)</script>`, nu vei vedea asta ca un redirect în Network tab. Folosește Performance tab sau adaugă logging.

3. **Separă autentificarea de autorizare** — coduri de HTTP diferite (401 vs 403) există dintr-un motiv. Tratează-le diferit.

4. **Când același bug se reproduce constant, cauza e deterministă** — dacă un utilizator rămâne mereu blocat într-o buclă de redirect, serverul trimite răspunsuri deterministe. Bug-ul e în codul serverului, nu într-o race condition sau o întâmplare client-side.

5. **Într-o arhitectură zero-JS, TOATĂ logica de redirect e în server** — dacă ceva redirectează incorect, e pentru că serverul a trimis redirect-ul greșit. Caută în codul serverului mai întâi.

### Meta-Lecția

Am ales o abordare "web modern" (HTMX + JavaScript redirects) tocmai pentru a evita complexitatea client-side. Dar când serverul a trimis un redirect bazat pe script (pasul 4 al buclei), am uitat de propria noastră arhitectură și ne-am uitat în cod JavaScript pentru bug. Ironia e că am eliminat HTMX și JavaScript ca să simplificăm debugging-ul — dar apoi am dat vina pe JavaScript pentru un bug care era în Rust de la început.

**Redirect-ul era în Rust. Doar se întâmpla să fie executat printr-un tag de script.**
