# Dependency Upgrade Log

**Date:** 2026-06-17 | **Project:** toon_rust (tru) | **Language:** Rust | **Toolchain:** nightly

Part of the ecosystem-wide library-update pass (cass + franken* siblings),
executed bottom-up in dependency order (leaves first). toon_rust is a leaf
(its only franken dep is `asupersync =0.3.4`, optional).

## Summary
- _in progress_

## Outdated at start (cargo outdated -R)
| Dependency | Current | Latest | Kind | Bump |
|---|---|---|---|---|
| chrono | 0.4.44 | 0.4.45 | normal | patch |
| insta | 1.47.2 | 1.48.0 | dev | minor |
| js-sys | 0.3.98 | 0.3.102 | optional | patch |
| wasm-bindgen | 0.2.121 | 0.2.125 | optional | patch |
| vergen-gix | 9.1.0 | 10.0.0 | build | **major** |

## Updates

### chrono: 0.4.44 → 0.4.45  •  insta: 1.47.2 → 1.48.0
- **Kind:** patch / minor (caret-compatible, via `cargo update`)
- **Breaking:** None
- **Tests:** ✓ ~250 tests pass

### vergen-gix: 9.1.0 → 10.0.0  (build-dependency, **major**)
- **Breaking changes found (research + compile):**
  1. Standalone `BuildBuilder`/`CargoBuilder`/`RustcBuilder` removed → use
     `Build::builder()` / `Cargo::builder()` / `Rustc::builder()` (config
     struct + `builder()` method). `Emitter` unchanged.
  2. v10's bon-based `.build()` returns the value directly, not a `Result` —
     removed the `?` on the three builder calls.
  - MSRV raised to 1.95 (satisfied: repo is on nightly).
- **Migration:** updated `build.rs` imports + the three builder calls.
- **Tests:** ✓ ~250 tests pass after migration.
- Note: js-sys/wasm-bindgen (wasm-only, optional) are not in the default
  resolution; their patch bumps land when the `wasm` feature is built.

## Summary
- **Updated:** 3 (chrono, insta, vergen-gix incl. a build.rs migration)
- **Lockfile-only (wasm-gated):** js-sys, wasm-bindgen (pending a wasm build)
- **Failed:** 0  •  **Needs attention:** 0
- toon_rust dependency update **complete**; full test suite green.
