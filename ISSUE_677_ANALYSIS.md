# Issue #677 Analysis: `--session-name` writes empty state on macOS

## Summary

The `--session-name` feature fails to persist browser state (cookies and
localStorage) across restarts. The generated state file contains only empty
arrays: `{ "cookies": [], "origins": [] }`.

## Root Causes

### Bug 1 (Primary): State saved when page is on `about:blank`

**File:** `cli/src/native/actions.rs:1676-1689`

When `handle_close()` fires, the browser tab may have already navigated to
`about:blank` or `chrome://newtab`. The `save_state` function evaluates
`location.origin` via JavaScript — on `about:blank` this returns the string
`"null"`, which triggers the empty-origins guard in `state.rs:90`:

```rust
if !origin.is_empty() && origin != "null" {
    vec![OriginStorage { ... }]
} else {
    vec![]  // origins becomes empty
}
```

**Result:** `origins` is always `[]` when the page is on a blank/internal URL.

### Bug 2: Only the current origin's storage is captured

**File:** `cli/src/native/state.rs:45-101`

`save_state` only captures localStorage/sessionStorage from the **currently
active page's origin**. If the user visited multiple origins (e.g. auth on one
domain, app on another), only the last-visited origin's storage is saved.

Playwright's `storageState()` captures **all origins** — this implementation
does not.

### Bug 3: CDP errors silently swallowed

**File:** `cli/src/native/cookies.rs:44`

`Network.getCookies` may return empty when the CDP session is closing. The
`unwrap_or_default()` silently converts errors/empty responses to `Vec::new()`:

```rust
let cookies: Vec<Cookie> = result
    .get("cookies")
    .and_then(|v| serde_json::from_value(v.clone()).ok())
    .unwrap_or_default();  // silently returns empty vec
```

On macOS (Apple Silicon), process teardown is faster, making this race more
likely.

## Why Other Approaches Work

| Approach | Why it works |
|----------|-------------|
| `--profile` | Uses Chrome's built-in user-data-dir persistence; no CDP calls needed |
| Manual `state save` | Invoked while the browser is actively on the target page |
| `--state` (load only) | Reads from a pre-existing file; no shutdown timing issues |

## Recommended Fixes

| Priority | Fix | Location |
|----------|-----|----------|
| P0 | Save state eagerly on navigation events, not just at shutdown | `actions.rs` |
| P1 | Capture all origins' localStorage (iterate targets or maintain origin registry) | `state.rs:45-101` |
| P1 | Log errors instead of silently defaulting to empty cookies | `cookies.rs:44` |
| P2 | Before saving at close, verify the page is on a real origin; if on `about:blank`, navigate back to last known URL | `actions.rs:1676-1689` |

## Relevant Code Paths

- **Save:** `state::save_state()` → `cookies::get_cookies()` + JS `Runtime.evaluate` → write JSON
- **Load:** `state::load_state()` → read JSON → `cookies::set_cookies()` + JS `localStorage.setItem()`
- **Auto-restore:** `try_auto_restore_state()` → `find_auto_state_file()` → `load_state()`
- **Close:** `handle_close()` → `save_state()` → `mgr.close()`
