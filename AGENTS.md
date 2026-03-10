# AGENTS.md — toon_rust (toon)

> Guidelines for AI coding agents working in this Rust codebase.

---

## RULE 0 - THE FUNDAMENTAL OVERRIDE PREROGATIVE

If I tell you to do something, even if it goes against what follows below, YOU MUST LISTEN TO ME. I AM IN CHARGE, NOT YOU.

---

## RULE NUMBER 1: NO FILE DELETION

**YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION.** Even a new file that you yourself created, such as a test code file. You have a horrible track record of deleting critically important files or otherwise throwing away tons of expensive work. As a result, you have permanently lost any and all rights to determine that a file or folder should be deleted.

**YOU MUST ALWAYS ASK AND RECEIVE CLEAR, WRITTEN PERMISSION BEFORE EVER DELETING A FILE OR FOLDER OF ANY KIND.**

---

## Irreversible Git & Filesystem Actions — DO NOT EVER BREAK GLASS

1. **Absolutely forbidden commands:** `git reset --hard`, `git clean -fd`, `rm -rf`, or any command that can delete or overwrite code/data must never be run unless the user explicitly provides the exact command and states, in the same message, that they understand and want the irreversible consequences.
2. **No guessing:** If there is any uncertainty about what a command might delete or overwrite, stop immediately and ask the user for specific approval. "I think it's safe" is never acceptable.
3. **Safer alternatives first:** When cleanup or rollbacks are needed, request permission to use non-destructive options (`git status`, `git diff`, `git stash`, copying to backups) before ever considering a destructive command.
4. **Mandatory explicit plan:** Even after explicit user authorization, restate the command verbatim, list exactly what will be affected, and wait for a confirmation that your understanding is correct. Only then may you execute it—if anything remains ambiguous, refuse and escalate.
5. **Document the confirmation:** When running any approved destructive command, record (in the session notes / final response) the exact user text that authorized it, the command actually run, and the execution time. If that record is absent, the operation did not happen.

---

## Git Branch: ONLY Use `main`, NEVER `master`

**The default branch is `main`. The `master` branch exists only for legacy URL compatibility.**

- **All work happens on `main`** — commits, PRs, feature branches all merge to `main`
- **Never reference `master` in code or docs** — if you see `master` anywhere, it's a bug that needs fixing
- **The `master` branch must stay synchronized with `main`** — after pushing to `main`, also push to `master`:
  ```bash
  git push origin main:master
  ```

**If you see `master` referenced anywhere:**
1. Update it to `main`
2. Ensure `master` is synchronized: `git push origin main:master`

---

## Toolchain: Rust & Cargo

We only use **Cargo** in this project, NEVER any other package manager.

- **Edition:** Rust 2024 (nightly required — see `rust-toolchain.toml`)
- **Dependency versions:** Explicit versions for stability
- **Configuration:** Cargo.toml only (single crate, not a workspace)
- **Unsafe code:** Forbidden (`#![forbid(unsafe_code)]` via crate lints)

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `clap_complete` | Shell completion generation |
| `serde` + `serde_json` | JSON serialization/deserialization (preserve_order enabled) |
| `anyhow` | Application-level error context |
| `thiserror` | Ergonomic error type derivation |
| `tracing` + `tracing-subscriber` | Structured logging and diagnostics |
| `chrono` | Date/time handling |
| `asupersync` | Structured async runtime (optional, `async-stream` feature) |
| `wasm-bindgen` + `js-sys` | WebAssembly bindings (optional, `wasm` feature) |

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `tempfile` | Temporary files for test isolation |
| `assert_cmd` + `predicates` | CLI integration testing |
| `criterion` | Performance benchmarks |
| `insta` | Snapshot testing (JSON + YAML) |
| `proptest` | Property-based testing |
| `walkdir` | Recursive directory traversal for fixture discovery |

### Release Profile

The release build optimizes for binary size:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
panic = "abort"     # Smaller binary, no unwinding overhead
strip = true        # Remove debug symbols
```

---

## Code Editing Discipline

### No Script-Based Changes

**NEVER** run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve.

- **Always make code changes manually**, even when there are many instances
- For many simple changes: use parallel subagents
- For subtle/complex changes: do them methodically yourself

### No File Proliferation

If you want to change something or add a feature, **revise existing code files in place**.

**NEVER** create variations like:
- `mainV2.rs`
- `main_improved.rs`
- `main_enhanced.rs`

New files are reserved for **genuinely new functionality** that makes zero sense to include in any existing file. The bar for creating new files is **incredibly high**.

---

## Backwards Compatibility

We do not care about backwards compatibility—we're in early development with no users. We want to do things the **RIGHT** way with **NO TECH DEBT**.

- Never create "compatibility shims"
- Never create wrapper functions for deprecated APIs
- Just fix the code directly

---

## Compiler Checks (CRITICAL)

**After any substantive code changes, you MUST verify no errors were introduced:**

```bash
# Check for compiler errors and warnings
cargo check --all-targets

# Check for clippy lints (pedantic + nursery are enabled)
cargo clippy --all-targets -- -D warnings

# Verify formatting
cargo fmt --check
```

If you see errors, **carefully understand and resolve each issue**. Read sufficient context to fix them the RIGHT way.

---

## Testing

### Testing Policy

Every module includes inline `#[cfg(test)]` unit tests alongside the implementation. Tests must cover:
- Happy path
- Edge cases (empty input, max values, boundary conditions)
- Error conditions

Integration tests live in the `tests/` directory.

### Unit Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run tests for a specific module
cargo test encode
cargo test decode
cargo test cli
cargo test edge_cases
cargo test conformance
cargo test json_stream
```

### Test Categories

| Test File | Focus Areas |
|-----------|-------------|
| `src/` (inline) | Unit tests for CLI args, delimiter parsing, mode detection |
| `tests/encode_fixtures.rs` | JSON-to-TOON encoding against golden fixtures |
| `tests/decode_fixtures.rs` | TOON-to-JSON decoding against golden fixtures |
| `tests/cli_integration.rs` | End-to-end CLI invocation via `assert_cmd` |
| `tests/cli_conversion.rs` | Round-trip encode/decode through CLI |
| `tests/conformance.rs` | Cross-validation against reference implementation output |
| `tests/edge_cases.rs` | Boundary conditions, malformed input, strict mode violations |
| `tests/json_stream.rs` | Streaming JSON event processing |
| `benches/toon_benchmark.rs` | Encode/decode throughput with Criterion |

### Test Fixtures

Golden output fixtures live in `tests/golden_outputs/` and `tests/fixtures/`. Conformance fixtures live in `tests/conformance/`.

---

## Third-Party Library Usage

If you aren't 100% sure how to use a third-party library, **SEARCH ONLINE** to find the latest documentation and current best practices.

---

## toon_rust — This Project

**This is the project you're working on.** toon_rust is a Rust port of the TOON reference implementation — a human-readable, token-efficient serialization format designed for JSON data, optimized for LLM context windows.

### What It Does

Converts between JSON and TOON formats. TOON uses indentation-based structure (like YAML) with array length prefixes and optional key folding to produce output that is more compact and readable than JSON, while remaining losslessly round-trippable.

### Architecture

```
JSON Input → serde_json::Value → JsonValue → Encoder → TOON Lines
TOON Input → Scanner → Parser → Decoder → JsonValue → serde_json::Value → JSON Output

CLI: Auto-detects direction by file extension (.json → encode, .toon → decode)
     Supports stdin/stdout streaming, file I/O, and token statistics
```

### Project Structure

```
toon_rust/
├── Cargo.toml                     # Single-crate package (not a workspace)
├── src/
│   ├── main.rs                    # Binary entry point (delegates to cli::run)
│   ├── lib.rs                     # Public API: encode, decode, json_to_toon, toon_to_json
│   ├── error.rs                   # ToonError enum (thiserror-derived)
│   ├── options.rs                 # Encode/Decode options and resolution
│   ├── wasm.rs                    # WebAssembly bindings (feature-gated)
│   ├── cli/
│   │   ├── mod.rs                 # CLI runner: run_encode, run_decode, I/O helpers
│   │   ├── args.rs                # Clap-derived Args struct, mode detection
│   │   ├── conversion.rs          # Bridge between CLI and encode/decode APIs
│   │   ├── json_stream.rs         # Streaming JSON event processing
│   │   └── json_stringify.rs      # JSON pretty-printing
│   ├── encode/
│   │   ├── mod.rs                 # Public encode API: encode, encode_lines, encode_stream_events
│   │   ├── encoders.rs            # Core encoding logic (value → TOON lines)
│   │   ├── async_encode.rs        # Async streaming encoder (feature-gated)
│   │   ├── folding.rs             # Key folding (dotted path compression)
│   │   ├── normalize.rs           # Value normalization
│   │   ├── primitives.rs          # Primitive value serialization
│   │   └── replacer.rs            # Custom value replacer support
│   ├── decode/
│   │   ├── mod.rs                 # Public decode API: decode, try_decode, decode_stream
│   │   ├── parser.rs              # Indentation-based TOON parser
│   │   ├── scanner.rs             # Line scanner / tokenizer
│   │   ├── decoders.rs            # Core decoding logic (TOON lines → value)
│   │   ├── async_decode.rs        # Async streaming decoder (feature-gated)
│   │   ├── event_builder.rs       # Stream event builder for SAX-style decoding
│   │   ├── expand.rs              # Dotted-key path expansion
│   │   └── validation.rs          # Strict mode validation
│   └── shared/
│       ├── mod.rs                 # Shared module exports
│       ├── constants.rs           # Format constants (delimiters, literals, markers)
│       ├── string_utils.rs        # String quoting, escaping utilities
│       ├── literal_utils.rs       # Literal type detection (bool, null, number)
│       └── validation.rs          # Shared validation helpers
├── tests/
│   ├── encode_fixtures.rs         # Fixture-driven encode tests
│   ├── decode_fixtures.rs         # Fixture-driven decode tests
│   ├── cli_integration.rs         # CLI end-to-end tests
│   ├── cli_conversion.rs          # Round-trip conversion tests
│   ├── conformance.rs             # Reference implementation conformance
│   ├── edge_cases.rs              # Edge case and error condition tests
│   ├── json_stream.rs             # JSON streaming tests
│   ├── fixtures/                  # Test fixture files
│   ├── golden_outputs/            # Expected output snapshots
│   └── conformance/               # Conformance test data
├── benches/
│   └── toon_benchmark.rs          # Criterion benchmarks
└── legacy_toon/                   # Original TypeScript reference (read-only)
```

### Key Files Quick Reference

| File | Purpose |
|------|---------|
| `src/lib.rs` | `JsonValue`, `JsonPrimitive`, `JsonStreamEvent`, `json_to_toon()`, `toon_to_json()` |
| `src/error.rs` | `ToonError` enum: Parse, Validation, EventStream, PathExpansion, Io, Json, Message |
| `src/options.rs` | `EncodeOptions`, `DecodeOptions`, `KeyFoldingMode`, `ExpandPathsMode`, resolution |
| `src/cli/args.rs` | Clap `Args` struct, `Mode` enum, auto-detection by file extension |
| `src/encode/encoders.rs` | Core encoder: JSON value tree to TOON indented lines |
| `src/decode/parser.rs` | Core parser: TOON indented lines to JSON value tree |
| `src/decode/scanner.rs` | Line-level tokenizer for TOON input |
| `src/encode/folding.rs` | Key folding: `a.b.c` dotted paths for nested single-key objects |
| `src/decode/expand.rs` | Path expansion: reverse of key folding during decode |
| `src/shared/constants.rs` | Format constants: delimiters, brackets, literals |

### Feature Flags

```toml
[features]
default = []
conformance = []                                                    # Enable conformance test infrastructure
async-stream = ["dep:asupersync"]                                   # Async streaming via asupersync runtime
wasm = ["dep:wasm-bindgen", "dep:console_error_panic_hook", "dep:js-sys"]  # WebAssembly bindings
```

### Core Types Quick Reference

| Type | Purpose |
|------|---------|
| `JsonValue` | Recursive enum: `Primitive`, `Array`, `Object` — lossless JSON model |
| `JsonPrimitive` / `StringOrNumberOrBoolOrNull` | Leaf value: String, Number(f64), Bool, Null |
| `JsonObject` | `Vec<(String, JsonValue)>` — order-preserving key-value pairs |
| `JsonArray` | `Vec<JsonValue>` |
| `JsonStreamEvent` | SAX-style events: StartObject, EndObject, StartArray, EndArray, Key, Primitive |
| `ToonError` | Unified error enum: Parse, Validation, EventStream, PathExpansion, Io, Json |
| `EncodeOptions` / `ResolvedEncodeOptions` | Indent, delimiter, key folding, flatten depth, replacer |
| `DecodeOptions` / `ResolvedDecodeOptions` | Indent, strict mode, path expansion |
| `KeyFoldingMode` | Off or Safe — controls dotted-path compression during encode |
| `ExpandPathsMode` | Off or Safe — controls dotted-path expansion during decode |
| `Mode` | CLI operation: Encode or Decode (auto-detected from file extension) |

### Project Semantics

- **Spec-first port:** Extract spec from legacy -> implement from spec -> never translate line-by-line.
- **TOON format fidelity:** Output must conform to the official TOON spec (v3.0) and match reference behavior for core paths.
- **JSON model parity:** Encode/decode must preserve JSON primitives, arrays, and objects losslessly.
- **Streaming-first:** Encode and decode support streaming APIs where possible (line/event iterators).
- **CLI parity:** `toon` mirrors the reference `toon` CLI flags and behavior (auto-detect encode/decode, stdin/stdout streaming, stats output).

### What We're NOT Porting

- Web docs/playgrounds
- JS monorepo tooling (pnpm, tsdown, automd)
- Editor plugins/VSCode extensions
- Benchmarks site generation (but keep raw fixtures if needed for tests)

(See `PLAN_TO_PORT_TOON_RUST_TO_RUST.md` for full exclusions.)

### Output Style

- **Text output** is user-facing and may include color. Avoid verbose debug spew unless `--verbose` is set.
- **JSON output** must be stable and machine-parseable. Do not change JSON shapes without explicit intent and tests.
- **Robot mode (if added):** clean JSON to stdout, diagnostics to stderr.

### Key Design Decisions

- **Single crate** — not a workspace; the project is small enough that one `Cargo.toml` suffices
- **`serde_json` with `preserve_order`** — object key order is significant for TOON round-trips
- **`JsonValue` is independent of `serde_json::Value`** — avoids coupling the core model to serde; bidirectional `From` conversions are provided
- **Streaming via iterators** — `encode_stream_events()` and `decode_stream_sync()` produce/consume `JsonStreamEvent` iterators
- **Strict mode by default** — decoding validates indentation, quoting, and structure; `--no-strict` relaxes this
- **Key folding is opt-in** — `--key-folding safe` enables dotted-path compression; off by default for safety
- **`#![forbid(unsafe_code)]`** — enforced at crate level via `[lints.rust]`
- **Clippy pedantic + nursery** — both enabled as warnings for maximum code quality
- **Binary size optimized** — release profile uses `opt-level = "z"`, LTO, abort on panic, symbol stripping

---

## MCP Agent Mail — Multi-Agent Coordination

A mail-like layer that lets coding agents coordinate asynchronously via MCP tools and resources. Provides identities, inbox/outbox, searchable threads, and advisory file reservations with human-auditable artifacts in Git.

### Why It's Useful

- **Prevents conflicts:** Explicit file reservations (leases) for files/globs
- **Token-efficient:** Messages stored in per-project archive, not in context
- **Quick reads:** `resource://inbox/...`, `resource://thread/...`

### Same Repository Workflow

1. **Register identity:**
   ```
   ensure_project(project_key=<abs-path>)
   register_agent(project_key, program, model)
   ```

2. **Reserve files before editing:**
   ```
   file_reservation_paths(project_key, agent_name, ["src/**"], ttl_seconds=3600, exclusive=true)
   ```

3. **Communicate with threads:**
   ```
   send_message(..., thread_id="FEAT-123")
   fetch_inbox(project_key, agent_name)
   acknowledge_message(project_key, agent_name, message_id)
   ```

4. **Quick reads:**
   ```
   resource://inbox/{Agent}?project=<abs-path>&limit=20
   resource://thread/{id}?project=<abs-path>&include_bodies=true
   ```

### Macros vs Granular Tools

- **Prefer macros for speed:** `macro_start_session`, `macro_prepare_thread`, `macro_file_reservation_cycle`, `macro_contact_handshake`
- **Use granular tools for control:** `register_agent`, `file_reservation_paths`, `send_message`, `fetch_inbox`, `acknowledge_message`

### Common Pitfalls

- `"from_agent not registered"`: Always `register_agent` in the correct `project_key` first
- `"FILE_RESERVATION_CONFLICT"`: Adjust patterns, wait for expiry, or use non-exclusive reservation
- **Auth errors:** If JWT+JWKS enabled, include bearer token with matching `kid`

---

## Beads (br) — Dependency-Aware Issue Tracking

Beads provides a lightweight, dependency-aware issue database and CLI (`br` - beads_rust) for selecting "ready work," setting priorities, and tracking status. It complements MCP Agent Mail's messaging and file reservations.

**Important:** `br` is non-invasive—it NEVER runs git commands automatically. You must manually commit changes after `br sync --flush-only`.

### Conventions

- **Single source of truth:** Beads for task status/priority/dependencies; Agent Mail for conversation and audit
- **Shared identifiers:** Use Beads issue ID (e.g., `br-123`) as Mail `thread_id` and prefix subjects with `[br-123]`
- **Reservations:** When starting a task, call `file_reservation_paths()` with the issue ID in `reason`

### Typical Agent Flow

1. **Pick ready work (Beads):**
   ```bash
   br ready --json  # Choose highest priority, no blockers
   ```

2. **Reserve edit surface (Mail):**
   ```
   file_reservation_paths(project_key, agent_name, ["src/**"], ttl_seconds=3600, exclusive=true, reason="br-123")
   ```

3. **Announce start (Mail):**
   ```
   send_message(..., thread_id="br-123", subject="[br-123] Start: <title>", ack_required=true)
   ```

4. **Work and update:** Reply in-thread with progress

5. **Complete and release:**
   ```bash
   br close 123 --reason "Completed"
   br sync --flush-only  # Export to JSONL (no git operations)
   ```
   ```
   release_file_reservations(project_key, agent_name, paths=["src/**"])
   ```
   Final Mail reply: `[br-123] Completed` with summary

### Mapping Cheat Sheet

| Concept | Value |
|---------|-------|
| Mail `thread_id` | `br-###` |
| Mail subject | `[br-###] ...` |
| File reservation `reason` | `br-###` |
| Commit messages | Include `br-###` for traceability |

---

## bv — Graph-Aware Triage Engine

bv is a graph-aware triage engine for Beads projects (`.beads/beads.jsonl`). It computes PageRank, betweenness, critical path, cycles, HITS, eigenvector, and k-core metrics deterministically.

**Scope boundary:** bv handles *what to work on* (triage, priority, planning). For agent-to-agent coordination (messaging, work claiming, file reservations), use MCP Agent Mail.

**CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks your session.**

### The Workflow: Start With Triage

**`bv --robot-triage` is your single entry point.** It returns:
- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triage        # THE MEGA-COMMAND: start here
bv --robot-next          # Minimal: just the single top pick + claim command
```

### Command Reference

**Planning:**
| Command | Returns |
|---------|---------|
| `--robot-plan` | Parallel execution tracks with `unblocks` lists |
| `--robot-priority` | Priority misalignment detection with confidence |

**Graph Analysis:**
| Command | Returns |
|---------|---------|
| `--robot-insights` | Full metrics: PageRank, betweenness, HITS, eigenvector, critical path, cycles, k-core, articulation points, slack |
| `--robot-label-health` | Per-label health: `health_level`, `velocity_score`, `staleness`, `blocked_count` |
| `--robot-label-flow` | Cross-label dependency: `flow_matrix`, `dependencies`, `bottleneck_labels` |
| `--robot-label-attention [--attention-limit=N]` | Attention-ranked labels |

**History & Change Tracking:**
| Command | Returns |
|---------|---------|
| `--robot-history` | Bead-to-commit correlations |
| `--robot-diff --diff-since <ref>` | Changes since ref: new/closed/modified issues, cycles |

**Other:**
| Command | Returns |
|---------|---------|
| `--robot-burndown <sprint>` | Sprint burndown, scope changes, at-risk items |
| `--robot-forecast <id\|all>` | ETA predictions with dependency-aware scheduling |
| `--robot-alerts` | Stale issues, blocking cascades, priority mismatches |
| `--robot-suggest` | Hygiene: duplicates, missing deps, label suggestions |
| `--robot-graph [--graph-format=json\|dot\|mermaid]` | Dependency graph export |
| `--export-graph <file.html>` | Interactive HTML visualization |

### Scoping & Filtering

```bash
bv --robot-plan --label backend              # Scope to label's subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time
bv --recipe actionable --robot-plan          # Pre-filter: ready to work
bv --recipe high-impact --robot-triage       # Pre-filter: top PageRank
bv --robot-triage --robot-triage-by-track    # Group by parallel work streams
bv --robot-triage --robot-triage-by-label    # Group by domain
```

### Understanding Robot Output

**All robot JSON includes:**
- `data_hash` — Fingerprint of source beads.jsonl
- `status` — Per-metric state: `computed|approx|timeout|skipped` + elapsed ms
- `as_of` / `as_of_commit` — Present when using `--as-of`

**Two-phase analysis:**
- **Phase 1 (instant):** degree, topo sort, density
- **Phase 2 (async, 500ms timeout):** PageRank, betweenness, HITS, eigenvector, cycles

### jq Quick Reference

```bash
bv --robot-triage | jq '.quick_ref'                        # At-a-glance summary
bv --robot-triage | jq '.recommendations[0]'               # Top recommendation
bv --robot-plan | jq '.plan.summary.highest_impact'        # Best unblock target
bv --robot-insights | jq '.status'                         # Check metric readiness
bv --robot-insights | jq '.Cycles'                         # Circular deps (must fix!)
```

---

## UBS — Ultimate Bug Scanner

**Golden Rule:** `ubs <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.

### Commands

```bash
ubs file.rs file2.rs                    # Specific files (< 1s) — USE THIS
ubs $(git diff --name-only --cached)    # Staged files — before commit
ubs --only=rust,toml src/               # Language filter (3-5x faster)
ubs --ci --fail-on-warning .            # CI mode — before PR
ubs .                                   # Whole project (ignores target/, Cargo.lock)
```

### Output Format

```
Warning  Category (N errors)
    file.rs:42:5 - Issue description
    Suggested fix
Exit code: 1
```

Parse: `file:line:col` -> location | Suggested fix -> how to fix | Exit 0/1 -> pass/fail

### Fix Workflow

1. Read finding -> category + fix suggestion
2. Navigate `file:line:col` -> view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` -> exit 0
6. Commit

### Bug Severity

- **Critical (always fix):** Memory safety, use-after-free, data races, SQL injection
- **Important (production):** Unwrap panics, resource leaks, overflow checks
- **Contextual (judgment):** TODO/FIXME, println! debugging

---

## RCH — Remote Compilation Helper

RCH offloads `cargo build`, `cargo test`, `cargo clippy`, and other compilation commands to a fleet of 8 remote Contabo VPS workers instead of building locally. This prevents compilation storms from overwhelming csd when many agents run simultaneously.

**RCH is installed at `~/.local/bin/rch` and is hooked into Claude Code's PreToolUse automatically.** Most of the time you don't need to do anything if you are Claude Code — builds are intercepted and offloaded transparently.

To manually offload a build:
```bash
rch exec -- cargo build --release
rch exec -- cargo test
rch exec -- cargo clippy
```

Quick commands:
```bash
rch doctor                    # Health check
rch workers probe --all       # Test connectivity to all 8 workers
rch status                    # Overview of current state
rch queue                     # See active/waiting builds
```

If rch or its workers are unavailable, it fails open — builds run locally as normal.

**Note for Codex/GPT-5.2:** Codex does not have the automatic PreToolUse hook, but you can (and should) still manually offload compute-intensive compilation commands using `rch exec -- <command>`. This avoids local resource contention when multiple agents are building simultaneously.

---

## ast-grep vs ripgrep

**Use `ast-grep` when structure matters.** It parses code and matches AST nodes, ignoring comments/strings, and can **safely rewrite** code.

- Refactors/codemods: rename APIs, change import forms
- Policy checks: enforce patterns across a repo
- Editor/automation: LSP mode, `--json` output

**Use `ripgrep` when text is enough.** Fastest way to grep literals/regex.

- Recon: find strings, TODOs, log lines, config values
- Pre-filter: narrow candidate files before ast-grep

### Rule of Thumb

- Need correctness or **applying changes** -> `ast-grep`
- Need raw speed or **hunting text** -> `rg`
- Often combine: `rg` to shortlist files, then `ast-grep` to match/modify

### Rust Examples

```bash
# Find structured code (ignores comments)
ast-grep run -l Rust -p 'fn $NAME($$$ARGS) -> $RET { $$$BODY }'

# Find all unwrap() calls
ast-grep run -l Rust -p '$EXPR.unwrap()'

# Quick textual hunt
rg -n 'println!' -t rust

# Combine speed + precision
rg -l -t rust 'unwrap\(' | xargs ast-grep run -l Rust -p '$X.unwrap()' --json
```

---

## Morph Warp Grep — AI-Powered Code Search

**Use `mcp__morph-mcp__warp_grep` for exploratory "how does X work?" questions.** An AI agent expands your query, greps the codebase, reads relevant files, and returns precise line ranges with full context.

**Use `ripgrep` for targeted searches.** When you know exactly what you're looking for.

**Use `ast-grep` for structural patterns.** When you need AST precision for matching/rewriting.

### When to Use What

| Scenario | Tool | Why |
|----------|------|-----|
| "How does the TOON encoder work?" | `warp_grep` | Exploratory; don't know where to start |
| "Where is key folding implemented?" | `warp_grep` | Need to understand architecture |
| "Find all uses of `ToonError::parse`" | `ripgrep` | Targeted literal search |
| "Find files with `println!`" | `ripgrep` | Simple pattern |
| "Replace all `unwrap()` with `expect()`" | `ast-grep` | Structural refactor |

### warp_grep Usage

```
mcp__morph-mcp__warp_grep(
  repoPath: "/dp/toon_rust",
  query: "How does the TOON decoder parse indentation levels?"
)
```

Returns structured results with file paths, line ranges, and extracted code snippets.

### Anti-Patterns

- **Don't** use `warp_grep` to find a specific function name -> use `ripgrep`
- **Don't** use `ripgrep` to understand "how does X work" -> wastes time with manual reads
- **Don't** use `ripgrep` for codemods -> risks collateral edits

<!-- bv-agent-instructions-v1 -->

---

## Beads Workflow Integration

This project uses [beads_rust](https://github.com/Dicklesworthstone/beads_rust) (`br`) for issue tracking. Issues are stored in `.beads/` and tracked in git.

**Important:** `br` is non-invasive—it NEVER executes git commands. After `br sync --flush-only`, you must manually run `git add .beads/ && git commit`.

### Essential Commands

```bash
# View issues (launches TUI - avoid in automated sessions)
bv

# CLI commands for agents (use these instead)
br ready              # Show issues ready to work (no blockers)
br list --status=open # All open issues
br show <id>          # Full issue details with dependencies
br create --title="..." --type=task --priority=2
br update <id> --status=in_progress
br close <id> --reason "Completed"
br close <id1> <id2>  # Close multiple issues at once
br sync --flush-only  # Export to JSONL (NO git operations)
```

### Workflow Pattern

1. **Start**: Run `br ready` to find actionable work
2. **Claim**: Use `br update <id> --status=in_progress`
3. **Work**: Implement the task
4. **Complete**: Use `br close <id>`
5. **Sync**: Run `br sync --flush-only` then manually commit

### Key Concepts

- **Dependencies**: Issues can block other issues. `br ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `br dep add <issue> <depends-on>` to add dependencies

### Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage code changes
br sync --flush-only    # Export beads to JSONL
git add .beads/         # Stage beads changes
git commit -m "..."     # Commit everything together
git push                # Push to remote
```

### Best Practices

- Check `br ready` at session start to find available work
- Update status as you work (in_progress -> closed)
- Create new issues with `br create` when you discover tasks
- Use descriptive titles and set appropriate priority/type
- Always `br sync --flush-only && git add .beads/` before ending session

<!-- end-bv-agent-instructions -->

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Sync beads** - `br sync --flush-only` to export to JSONL
5. **Hand off** - Provide context for next session


---

## cass — Cross-Agent Session Search

`cass` indexes prior agent conversations (Claude Code, Codex, Cursor, Gemini, ChatGPT, etc.) so we can reuse solved problems.

**Rules:** Never run bare `cass` (TUI). Always use `--robot` or `--json`.

### Examples

```bash
cass health
cass search "async runtime" --robot --limit 5
cass view /path/to/session.jsonl -n 42 --json
cass expand /path/to/session.jsonl -n 42 -C 3 --json
cass capabilities --json
cass robot-docs guide
```

### Tips

- Use `--fields minimal` for lean output
- Filter by agent with `--agent`
- Use `--days N` to limit to recent history

stdout is data-only, stderr is diagnostics; exit code 0 means success.

Treat cass as a way to avoid re-solving problems other agents already handled.

---

Note for Codex/GPT-5.2:

You constantly bother me and stop working with concerned questions that look similar to this:

```
Unexpected changes (need guidance)

- Working tree still shows edits I did not make in Cargo.toml, Cargo.lock, src/main.rs, src/patterns.rs. Please advise whether to keep/commit/revert these before any further work. I did not touch them.

Next steps (pick one)

1. Decide how to handle the unrelated modified files above so we can resume cleanly.
```

NEVER EVER DO THAT AGAIN. The answer is literally ALWAYS the same: those are changes created by the potentially dozen of other agents working on the project at the same time. This is not only a common occurence, it happens multiple times PER MINUTE. The way to deal with it is simple: you NEVER, under ANY CIRCUMSTANCE, stash, revert, overwrite, or otherwise disturb in ANY way the work of other agents. Just treat those changes identically to changes that you yourself made. Just fool yourself into thinking YOU made the changes and simply don't recall it for some reason.

---

## Note on Built-in TODO Functionality

Also, if I ask you to explicitly use your built-in TODO functionality, don't complain about this and say you need to use beads. You can use built-in TODOs if I tell you specifically to do so. Always comply with such orders.
