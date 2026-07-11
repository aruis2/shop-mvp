# The Infinite Redirect Loop That Wasn't JavaScript

## The Debugging Journey That Cost Us Hours

### Context

We were building a zero-JS Rust+Axum web application. HTMX was removed. Everything was server-side rendering with form POST + 302 redirect (PRG pattern). Auth was handled by JWT in HttpOnly cookies, verified server-side.

One day, a user reported that after logging in with a non-admin account and trying to access `/admin`, they got stuck in an infinite redirect loop. The browser tab was spinning endlessly between `/login?redirect=/admin` and `/admin`.

### The Search

We spent hours searching for the bug. Where did we look?

- **The browser's Network tab** — we saw 302 redirects, script redirects, cookie being set, cookie being cleared
- **JavaScript files** — maybe the `window.location.replace()` was causing it?
- **HTMX headers** — maybe `hx-redirect` was interfering?
- **The client-side auth script** — maybe `localStorage` was corrupt?

We were convinced the problem was on the client side. After all, the redirect was happening in the browser. The server was "just sending responses."

### The Root Cause

The bug was in `verify_or_redirect()` in `admin.rs`:

```rust
async fn verify_or_redirect(...) -> Result<User, Html<String>> {
    verify_admin(headers, q, auth).await
        .map_err(|_| render_admin_redirect(bp, &rp))
}
```

`verify_admin` returns two different errors:
- `UNAUTHORIZED` — user has no token at all
- `FORBIDDEN` — user has a token but their role is not "admin"

But `verify_or_redirect` treated **both errors identically**: `|_|` — the wildcard pattern. Both cases resulted in a redirect to `/login?redirect=/admin`.

### The Loop

1. User with `role = "user"` navigates to `/admin`
2. `verify_or_redirect` detects their token, calls `verify_admin`
3. `verify_admin` sees `user.role != "admin"`, returns `FORBIDDEN`
4. `verify_or_redirect` catches the error with `|_|` and redirects to `/login?redirect=/admin`
5. User is ALREADY logged in (they have a valid token), so the login page detects this and redirects them back to `/admin`
6. Go to step 2 — infinite loop

### Why We Couldn't Find It

There are five distinct reasons this bug was invisible to our debugging approach:

#### 1. The Redirect Was a Script, Not an HTTP Redirect

The admin pages used `render_admin_redirect()` which returns:

```html
<script>window.location.replace('/login?redirect=/admin');</script>
```

This is NOT a 302 redirect. It's an HTML page with JavaScript that runs in the browser. The Network tab shows a 200 OK response (the HTML), then the script runs and navigates. Between step 4 and step 5, there's no HTTP redirect you can see in the Network tab — it's a client-side navigation triggered by JavaScript.

#### 2. The Login Page Has Its Own Redirect

When the login page (`login_page` handler) is loaded and the user already has a valid token, it immediately returns:

```rust
if s.auth.verify_token(token).await.is_ok() {
    let dest = q.redirect.clone().unwrap_or_else(|| format!("{}/", bp));
    return Ok(redirect_html(&dest));
}
```

This sends the user right back to `/admin`. So the flow was:

```
/admin → [script redirect] → /login?redirect=/admin → [server detects auth, redirects] → /admin → [script redirect] → ...
```

The login page's redirect is a proper 302 FOUND, which IS visible in the Network tab. But it looks like a normal "you're already logged in, here's your redirect" — not an error.

#### 3. The Error Was Silent — No Log, No Stack Trace

`verify_admin` returns `Err((StatusCode::FORBIDDEN, "Admin: acces interzis"))`. But `verify_or_redirect` swallows this with `map_err(|_| ...)`. The `_` discards both the status code AND the error message. There's no `tracing::warn!()`, no log entry, nothing.

If we had added a single line:

```rust
.map_err(|(status, msg)| {
    tracing::warn!("verify_admin failed: {} {}", status, msg);
    render_admin_redirect(bp, &rp)
})
```

We would have seen `FORBIDDEN Admin: acces interzis` in the server logs immediately.

#### 4. The Redirect Target Was the Same as the Error Source

In many redirect loop bugs, the loop involves different URLs (A → B → C → A). Here, the loop was:

```
/admin → /login?redirect=/admin → /admin → /login?redirect=/admin → ...
```

Only TWO URLs. Each URL looked legitimate:
- `/admin` — a valid page you might want to visit
- `/login?redirect=/admin` — a valid login page with a redirect parameter

Neither URL had an error message. There was no `?error=` in the URL. To the browser, this was a perfectly normal sequence of page loads.

#### 5. The Mental Model Was Wrong

We were thinking about the problem in terms of "authentication" (do you have a valid token?) when the real problem was "authorization" (do you have the right role?).

- `UNAUTHORIZED` (401) = "I don't know who you are" → redirect to login ✅
- `FORBIDDEN` (403) = "I know who you are, but you can't be here" → should redirect to home with error, NOT to login

These are fundamentally different concepts. But the code treated them the same.

### The Fix

```rust
match verify_admin(headers, q, auth).await {
    Ok(user) => Ok(user),
    Err((status, msg)) => {
        if status == StatusCode::FORBIDDEN {
            // Authenticated but not admin → home with error, not login (prevents loop)
            let dest = format!("{}/?error={}", bp, msg.replace(' ', "%20"));
            Err(Html(format!("<script>window.location.replace('{dest}');</script>")))
        } else {
            // Not authenticated → redirect to login
            Err(render_admin_redirect(bp, &rp))
        }
    }
}
```

### Lessons Learned

1. **`|_|` is a code smell in error handling** — especially when matching on `Result<T, E>` where `E` carries information. Always log or inspect the error before discarding it.

2. **Script redirects are invisible to network debugging** — if the server returns a 200 with a `<script>window.location.replace(...)</script>`, you won't see it as a redirect in the Network tab. Use the browser's Performance tab or add logging.

3. **Separate authentication from authorization** — different HTTP status codes (401 vs 403) exist for a reason. Handle them differently.

4. **When the same bug reproduces reliably, the cause is deterministic** — if a user always gets stuck in a redirect loop, the server is sending deterministic responses. The bug is in the server code, not in a race condition or client-side fluke.

5. **In a zero-JS architecture, ALL redirect logic is in the server** — if something redirects incorrectly, it's because the server sent the wrong redirect. Look at the server code first.

### The Meta-Lesson

We chose a "modern web" approach (HTMX + JavaScript redirects) precisely to avoid client-side complexity. But when the server sent a script-based redirect (step 4 of the loop), we forgot our own architecture and looked at JavaScript code for the bug. The irony is that we removed HTMX and JavaScript to simplify debugging — but then blamed JavaScript for a bug that was in Rust all along.

**The redirect was in Rust. We just happened to execute it via a script tag.**
