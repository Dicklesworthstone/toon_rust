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


## v0.3.0 dependency modernization, feature split, and MSRV restoration (2026-11-20)

### Removed dependencies
- **`vergen-gix` 10.0.x (build-dependency):** Removed completely. `build.rs` already emitted git metadata using pure `std::process::Command` + env fallback, and nothing in the crate read `VERGEN_BUILD_*` / `VERGEN_CARGO_*` / `VERGEN_RUSTC_*`. Removing `vergen-gix` restored true Rust 1.88.0 MSRV compatibility (fixing the 1.95+/1.96+ transitive build-dep regression), eliminated ~108 transitive `gix` crates, and resolved 9 unsound RustSec warnings (`RUSTSEC-2026-0306` in `faster-hex`, `RUSTSEC-2026-0574`..`RUSTSEC-2026-0582` in `heapless`).
- **`chrono` 0.4.45:** Unused anywhere in the crate; removing it resolved `RUSTSEC-2026-0543` (unmaintained).
- **`anyhow`, `clap_complete`, `tracing`, `tracing-subscriber`:** Unused anywhere in the crate; removed.
- **`thiserror`:** Replaced derive macro on `ToonError` with direct `std::fmt::Display` and `std::error::Error` implementations (unit-tested for exact message and `source()` parity).
- **`serde`:** Moved from `[dependencies]` to `[dev-dependencies]` (only used by test fixture deserializers).

### Feature split
- Split crate features into `lib` (zero third-party runtime dependencies over `JsonValue`), `serde` (optional `serde_json` interop), and `cli` (optional `clap` + `toon` binary), with `default = ["lib", "serde", "cli"]` preserving full CLI and default library compatibility.
- Added pure-stdlib ECMA-262 shortest round-trip float formatter with exact round-half-to-even tie-breaking for the zero-dependency `lib` build, verified bit-for-bit against `serde_json` across 16,384 deterministic bit patterns and 512 constructed exact binary64 decimal midpoints.

### Direct & transitive updates (all verified MSRV <= 1.88.0)
- `clap`: `4.6.7` -> `4.6.9` (manifest floor `4.6.0` -> `4.6.9`)
- `serde_json`: locked `1.0.154` (manifest floor aligned `1.0.150` -> `1.0.154`)
- `asupersync`: exact pin preserved at `=0.6.2`
- Coupled WASM family: `wasm-bindgen` `0.2.129` -> `0.2.130`, `js-sys` / `web-sys` `0.3.106` -> `0.3.107`, `wasm-bindgen-futures` `0.4.79` -> `0.4.80`, `console_error_panic_hook` `0.1.7`
- Dev dependencies: `serde` `1.0.229`, `tempfile` `3.27.0`, `walkdir` `2.5.0` -> `2.5.1`, `insta` `1.49.0`, `proptest` `1.12.0`, `rand` `0.10.3` -> `0.10.4`
- Security & soundness transitive refreshes: `ctutils` `0.4.2` -> `0.4.8` (resolves `RUSTSEC-2026-0385`), `rustix` `1.1.4` -> `1.1.6` (resolves `RUSTSEC-2026-0486`), `tokio` `1.53.2` -> `1.54.1` (resolves `RUSTSEC-2026-0386`), plus Rust 1.88-compatible transitive lock refreshes (holding `bincode-next` at `3.1.1` for Rust 1.88 compatibility).

### Verification
- `cargo audit -D warnings`: 0 vulnerabilities, 0 warnings across 259 locked crates.
- Unpacked `.crate` package built and tested from outside the repository on exact `rustc 1.88.0` across `--no-default-features`, `--features lib`, `--features serde`, and default features.
- Full test and Clippy (`-D warnings`) matrix passed across `--no-default-features`, `--features lib`, `--features serde`, default, `--features conformance`, `--features wasm`, and `--features async-stream`.
