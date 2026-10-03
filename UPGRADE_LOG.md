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

## v0.2.5 dependency maintenance (2026-10-03)

The frozen intake was f740e7b606f4a2795637e0a0c0df509d49951e6c.
The direct dependency inventory was checked against the registry on 2026-10-03.
Existing exact asupersync 0.5.0 and nightly-2026-08-31 pins were preserved.

The coupled WASM family advances wasm-bindgen 0.2.127 to 0.2.129, js-sys/web-sys
0.3.104 to 0.3.106, and wasm-bindgen-futures 0.4.77 to 0.4.79. Its official
changelog was reviewed; MSRV 1.81 remains below this package's 1.88. Updating
js-sys alone is invalid because these packages use exact companion versions.
The new Tokio entry belongs to wasm-bindgen-futures' experimental configuration;
`cargo tree --locked --features wasm -i tokio` finds no active package. Toon
continues to use asupersync for its optional async-stream feature.

insta advances 1.48.0 to 1.49.0 after reviewing its upstream release notes;
MSRV 1.66 remains compatible. No snapshots or fixture expectations were rewritten.
Cargo also deduplicated tempfile 3.27.0's getrandom edge from 0.4.3 to the existing
0.3.4. The unchanged published tempfile manifest permits `>=0.3.0, <0.5`; no
tempfile version, checksum or source changed.

The packaging check exposed yanked chacha20 0.10.1. Its official changelog
identifies an SSE4.1 intrinsic in the SSE2 RNG/legacy backend; update only that
transitive package to 0.10.2 (MSRV 1.85). The final gate includes this patch.

Remote formatting and all-target Clippy gates passed, as did all 277 default
tests and the WASM/conformance supplement (282 tests, with the conformance
harness's internal error-fixture skips disclosed in CHANGELOG). Independent
source/release review found no ship blockers. Detailed receipts and dependency
inventory are retained in the release-wave status/evidence directory.
