# Refactoring Analysis Report

> Automated analysis of `agent-browser` codebase identifying key refactoring opportunities.

---

## Executive Summary

The `agent-browser` CLI is a well-engineered Rust project (~37K LOC) with solid fundamentals. However, rapid feature growth has introduced technical debt in several areas. This analysis identifies **4 critical**, **8 high-priority**, and **12 medium-priority** refactoring points across the codebase.

**Top 3 impact areas:**
1. **God module decomposition** — `actions.rs` (7,389 LOC) and `commands.rs` (4,174 LOC) handle too many responsibilities
2. **Error handling system** — Pervasive `Result<T, String>` prevents error categorization and context chaining
3. **Code duplication** — 500+ instances of repeated patterns (browser checks, parameter extraction, selector parsing)

---

## 1. Architecture-Level Issues

### 1.1 God Object: `DaemonState` (CRITICAL)

**File:** `cli/src/native/actions.rs:158-199`

`DaemonState` contains 28 fields spanning every aspect of the daemon — browser management, event handling, network interception, recording/tracing, policy/auth, and stream server integration. Nearly every module receives `&mut DaemonState`, creating implicit coupling across all subsystems.

**Impact:** Any modification risks cascading failures. No separation between transport layer (CDP/WebDriver), domain logic (navigation, interaction), and cross-cutting concerns (policy, tracing, recording).

**Recommendation:** Decompose into focused manager structs:
```
DaemonState
├── BrowserContext       (browser, pages, sessions)
├── NetworkManager       (routes, fetch handler, origin headers, domain filter)
├── RecordingManager     (recording state, tracing state, HAR entries)
├── PolicyManager        (policy, confirmations)
└── StreamContext        (stream client, stream server)
```

### 1.2 No Error Type System (CRITICAL)

**Affected:** All 35+ modules

Every module uses `Result<T, String>` with no error categorization. Transient network errors look identical to config errors. Error context is lost during propagation.

**Specific problems:**
- `connection.rs:492-508` — Brittle string matching (`error.contains("os error 35")`) for error classification
- No context chain — when errors propagate, intermediate information is lost
- Inconsistent messages — some use "Failed to...", others expose raw technical details
- `actions.rs:405` — Malformed CDP events silently discarded with `.ok()`

**Recommendation:** Introduce a proper error enum:
```rust
enum AgentBrowserError {
    ConnectionError { source: io::Error, context: String },
    CdpError { code: i64, message: String },
    ValidationError { field: String, reason: String },
    BrowserNotLaunched,
    TimeoutError { operation: String, duration: Duration },
    // ...
}
```

### 1.3 Missing Trait Abstractions (HIGH)

Only one trait exists in the codebase (`BrowserBackend` in `webdriver/backend.rs`), yet the architecture has several natural polymorphism boundaries:

| Missing Trait | Benefit | Affected Modules |
|--------------|---------|-----------------|
| `DomAccessor` | Abstracts element queries from CDP specifics | element.rs, interaction.rs, snapshot.rs |
| `EventSubscriber` | Unified event pub/sub lifecycle | recording.rs, tracing.rs, network.rs, stream.rs |
| `LaunchStrategy` | Eliminates engine-specific branching | browser.rs (Chrome, Lightpanda, WebDriver) |
| `CommandRouter` | Replaces 1,271-line match statement | actions.rs |

### 1.4 Flag Propagation Ceremony (MEDIUM)

Adding a new CLI flag requires changes in 5 files:
1. `flags.rs` — Parsing logic
2. `connection.rs::DaemonOptions` — Struct field
3. `connection.rs::apply_daemon_env()` — Env var mapping
4. `native::daemon.rs` — Environment reading
5. `native::actions.rs` — Runtime behavior

**Recommendation:** Single source of truth config struct with derive macros for CLI parsing and env var binding.

---

## 2. File-Level Refactoring Points

### 2.1 `actions.rs` — 7,389 LOC (CRITICAL)

#### 2.1.1 Repeated Browser Manager Check (~120 occurrences)

```rust
let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
```

This exact pattern appears 120+ times throughout handler functions.

**Fix:** Extract helper:
```rust
fn get_browser(state: &DaemonState) -> Result<&BrowserManager, String> {
    state.browser.as_ref().ok_or_else(|| "Browser not launched".to_string())
}
```

#### 2.1.2 Repeated Parameter Extraction (~400+ occurrences)

```rust
let selector = cmd.get("selector").and_then(|v| v.as_str()).ok_or("Missing 'selector'")?;
```

**Fix:** Extract typed getters:
```rust
fn get_str_param<'a>(cmd: &'a Value, key: &str) -> Result<&'a str, String> {
    cmd.get(key).and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing '{}' parameter", key))
}

fn get_bool_param(cmd: &Value, key: &str) -> bool {
    cmd.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}
```

#### 2.1.3 Massive Functions

| Function | Lines | LOC | Issue |
|----------|-------|-----|-------|
| `parse_key_chord` | 2107-4380 | 2,273 | Should be split into key category handlers |
| `browser_metadata_from_version` | 5508-6448 | 940 | Deep nesting, should be table-driven |
| `build_role_selector` | 4381-5203 | 822 | Complex role mapping, extract to data structure |
| `error_response` | 6735-7389 | 654 | Mixes response builders with test code |
| `launch_options_from_env` | 1175-1621 | 446 | Repetitive env var parsing |
| `open_url_in_browser` | 1647-2106 | 459 | Platform-specific branching |

#### 2.1.4 Repeated Env Var Parsing

Lines 1176-1212 show repetitive patterns:
```rust
.map(|v| v == "1" || v == "true").unwrap_or(false)  // 10+ times
v.split([',', '\n']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()  // 5+ times
```

**Fix:** Helper functions `parse_bool_env()` and `parse_csv_env()`.

#### 2.1.5 Similar Handler Functions

Mouse event handlers (`handle_mousemove`, `handle_mousedown`, `handle_mouseup`) and check/uncheck handlers share nearly identical boilerplate. Could use a macro or generic handler.

### 2.2 `commands.rs` — 4,174 LOC (HIGH)

#### 2.2.1 Monolithic `parse_command` Function (1,271 lines)

Lines 74-1344 contain a single match statement handling 40+ commands. While helper functions exist (`parse_get`, `parse_set`, `parse_auth`), the main function remains enormous.

**Recommendation:** Command registry pattern — each command implements a `Command` trait with `parse()` and `name()` methods.

#### 2.2.2 Selector-Based Command Duplication

Commands `dblclick`, `hover`, `focus`, `check`, `uncheck` (lines 139-202) all follow:
```rust
let sel = rest.first().ok_or_else(|| ParseError::MissingArguments { ... })?;
Ok(json!({ "id": id, "action": "command", "selector": sel }))
```

**Fix:** `simple_selector_action(rest, id, action, usage)` helper.

#### 2.2.3 Flag-to-Value Extraction Pattern (10+ times)

```rust
let filter_idx = rest.iter().position(|&s| s == "--filter");
let filter = filter_idx.and_then(|i| rest.get(i + 1).copied());
```

**Fix:** `extract_flag_value(rest, "--filter")` helper.

#### 2.2.4 Duplicate Diff Parsing (~250 lines)

Lines 1349-1603: Snapshot diff and screenshot diff parse nearly identical flag combinations (`--baseline`, `--selector`, `--output`, `--depth`). Should be consolidated into shared diff option parser.

#### 2.2.5 Identical Start/Stop Subcommand Patterns

Trace, profiler, and record commands (lines 1018-1142) have identical structures:
```rust
const VALID: &[&str] = &["start", "stop"];
match sub { "start" => {...}, "stop" => {...}, _ => error }
```

**Fix:** Generic `start_stop_command(name, rest, id)` helper.

#### 2.2.6 Dialog Accept/Dismiss Duplication

Lines 992-1004: Only difference is the `"response"` field value. Should be a single parameterized arm.

### 2.3 `browser.rs` — 1,733 LOC (HIGH)

#### 2.3.1 Target Create + Attach Duplication (3 locations)

Identical CDP call sequence at lines 357-378, 675-696, and 746-767:
```rust
let result = self.client.send_command_typed("Target.createTarget", ...).await?;
let attach = self.client.send_command_typed("Target.attachToTarget", ...).await?;
```

**Fix:** Extract `create_and_attach_target(&self) -> Result<(String, String), String>`.

#### 2.3.2 PageInfo Construction Duplication (4 locations)

Lines 380-386, 403-409, 698-704, 772-778 all construct identical `PageInfo` structs.

**Fix:** `PageInfo::new_blank(target_id, session_id)` constructor.

#### 2.3.3 `launch()` Method — 113 Lines

Lines 199-312 mix engine validation, browser launching, manager initialization, and feature configuration. Extract `configure_browser_features()` for HTTPS errors, user agent, color scheme, and downloads configuration.

#### 2.3.4 Validation Logic Overlap

`validate_launch_options()` (lines 20-58) and `validate_lightpanda_options()` (lines 61-88) share duplicate checks for extensions, profile, and storage_state incompatibilities.

#### 2.3.5 Magic Values

| Value | Lines | Meaning |
|-------|-------|---------|
| `25_000` | 259 | Default timeout (ms) |
| `10_000` | 323 | Connection timeout (ms) |
| `600` | 1143 | Network poll timeout (ms) |
| `500` | 1190 | Idle detection threshold (ms) |
| `"about:blank"` | 362, 680, 701, 744 | Default page URL |
| `"page"` | 99, 144, 385, 408, 703 | Target type constant |

All should be named constants.

### 2.4 `output.rs` — Very Large (MEDIUM)

Extremely large file handling all output formatting and help text. Should be split into:
- `output/format.rs` — Response formatting logic
- `output/help.rs` — Help text content (which is mostly static data)

### 2.5 `connection.rs` (MEDIUM)

#### 2.5.1 Brittle Error Detection

Lines 492-508: Transient error detection via string matching:
```rust
fn is_transient_error(error: &str) -> bool {
    error.contains("os error 35")   // EAGAIN macOS
        || error.contains("os error 11")  // EAGAIN Linux
        // ... 11 more patterns
}
```

This breaks if error message formats change. Should use `io::ErrorKind` matching.

---

## 3. Cross-Cutting Patterns

### 3.1 JSON Path Repetition (~100+ occurrences)

Deep JSON access chains scattered everywhere:
```rust
tree_result.get("frameTree").and_then(|t| t.get("frame")).and_then(|f| f.get("id"))...
```

**Recommendation:** Consider a `json_path!()` macro or typed CDP response structs.

### 3.2 Inconsistent Color Usage

`color.rs` provides a clean API, but some modules use it while others use raw strings or skip coloring entirely.

### 3.3 Scattered Validation

Validation logic is spread across 5+ files (`validation.rs`, `browser.rs`, `policy.rs`, `flags.rs`, `commands.rs`) with inconsistent error types and return conventions.

---

## 4. Priority Matrix

### Critical (address first)
| # | Issue | Files | Est. Reduction |
|---|-------|-------|---------------|
| 1 | Introduce error type enum | All modules | Better debugging, no LOC reduction |
| 2 | Decompose `DaemonState` god object | actions.rs | Reduces coupling across 35+ modules |
| 3 | Extract parameter/browser helpers in actions.rs | actions.rs | ~500+ duplicate lines eliminated |
| 4 | Break down 2,273-line `parse_key_chord` | actions.rs | Better maintainability |

### High Priority
| # | Issue | Files | Est. Reduction |
|---|-------|-------|---------------|
| 5 | Consolidate selector commands in commands.rs | commands.rs | ~100 lines |
| 6 | Unify diff option parsing | commands.rs | ~250 lines |
| 7 | Extract target create+attach helper | browser.rs | ~60 lines |
| 8 | Extract flag parsing helper in commands.rs | commands.rs | ~100 lines |
| 9 | Generic start/stop command handler | commands.rs | ~80 lines |
| 10 | Extract `configure_browser_features()` | browser.rs | Better separation |
| 11 | Split output.rs into format + help | output.rs | Better organization |
| 12 | Define trait abstractions (DomAccessor, EventSubscriber) | native/ | Decoupling |

### Medium Priority
| # | Issue | Files | Est. Reduction |
|---|-------|-------|---------------|
| 13 | Env var parsing helpers | actions.rs | ~50 lines |
| 14 | Constants for magic values | browser.rs, actions.rs | Better readability |
| 15 | Consolidate validation patterns | 5+ files | Consistency |
| 16 | Typed CDP response structs | native/ | Type safety |
| 17 | JSON path macro or helper | multiple | ~200 lines |
| 18 | PageInfo constructor | browser.rs | ~40 lines |
| 19 | Dialog accept/dismiss consolidation | commands.rs | ~15 lines |
| 20 | Config struct with derive macros | flags.rs, connection.rs | Eliminates flag ceremony |
| 21 | Color usage consistency | multiple | UX consistency |
| 22 | Unified storage manager trait | state.rs, cookies.rs, storage.rs | Consistency |
| 23 | Mouse event handler macro | actions.rs | ~60 lines |
| 24 | Connection error kind matching | connection.rs | Robustness |

---

## 5. Estimated Impact

| Metric | Current | After Refactoring |
|--------|---------|-------------------|
| `actions.rs` LOC | 7,389 | ~5,500 (-25%) |
| `commands.rs` LOC | 4,174 | ~3,300 (-21%) |
| `browser.rs` LOC | 1,733 | ~1,500 (-13%) |
| Duplicate patterns | 500+ instances | ~100 instances |
| Error types | 1 (`String`) | 8-10 categorized variants |
| Trait abstractions | 1 | 5-6 |
| Max function length | 2,273 lines | <200 lines |

---

*Analysis generated for `agent-browser` v0.22.0*
