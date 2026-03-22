# Changelog

All notable changes to [toon_rust](https://github.com/Dicklesworthstone/toon_rust) are documented here.

Entries are organized by tagged release, with capabilities grouped thematically rather than by diff order. Commit links point to `https://github.com/Dicklesworthstone/toon_rust/commit/<hash>`. Releases with GitHub Releases are marked **(released)**; tag-only milestones are marked **(tag only)**.

---

## [Unreleased](https://github.com/Dicklesworthstone/toon_rust/compare/v0.2.1...HEAD)

Since v0.2.1 (2026-02-22). HEAD is [`153bd4f`](https://github.com/Dicklesworthstone/toon_rust/commit/153bd4fec34d3811d0e318ae0078a156f6974207).

### Licensing

- Update license to MIT with OpenAI/Anthropic Rider ([`7b2865c`](https://github.com/Dicklesworthstone/toon_rust/commit/7b2865cdbbc7be01bff64d171b45e212be091d12), [`fe9d0ba`](https://github.com/Dicklesworthstone/toon_rust/commit/fe9d0bac73106c8dbd9c58381733ffb5b81c1d1c))
- Update README license references to match ([`bc3f9da`](https://github.com/Dicklesworthstone/toon_rust/commit/bc3f9daaa3546f8722ca4163d7500886c58a69d8), [`d48c5b5`](https://github.com/Dicklesworthstone/toon_rust/commit/d48c5b55caeedc99368a1461535b7e0c143afe06))

### Housekeeping

- Remove stale `perf.data` profiling output ([`c5fa3b0`](https://github.com/Dicklesworthstone/toon_rust/commit/c5fa3b082cd624f46ca907339c5885eecbd24b87))
- Merge remote-tracking branch `origin/master` ([`49371d1`](https://github.com/Dicklesworthstone/toon_rust/commit/49371d1ee0d4b3b0ba399e6a6a3d7f9966562df7))
- Add cass (Cross-Agent Session Search) tool reference to AGENTS.md ([`6c72cee`](https://github.com/Dicklesworthstone/toon_rust/commit/6c72cee02640e5e214a63130cf02a7ab3372c8bd))

---

## [v0.2.1](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.2.1) -- 2026-02-22 **(released, latest)**

Tag: [`988bff0`](https://github.com/Dicklesworthstone/toon_rust/commit/988bff0d4955bdcd5913d7ab9e6e8b6718a91ba3) |
[Compare v0.2.0...v0.2.1](https://github.com/Dicklesworthstone/toon_rust/compare/v0.2.0...v0.2.1) |
[GitHub Release](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.2.1)

Release assets: cross-platform binaries (`toon-{darwin,linux,windows}-{amd64,arm64,musl-amd64}.tar.xz`) + SHA256 checksums.

### Security and Compatibility

- **Fix `cargo audit` vulnerability** (RUSTSEC-2026-0009): bump `time` 0.3.45 to 0.3.47 to eliminate stack-exhaustion DoS, update 39 other transitive deps ([`1b37caf`](https://github.com/Dicklesworthstone/toon_rust/commit/1b37caf69dc2a18adebb65ba6df2505c7dbcb06b))
- Bump MSRV from 1.85 to 1.88 to accommodate updated dependencies ([`1b37caf`](https://github.com/Dicklesworthstone/toon_rust/commit/1b37caf69dc2a18adebb65ba6df2505c7dbcb06b))

### Dependencies

- Upgrade 10 direct dependencies including clap 4.5.54 to 4.5.60, asupersync 0.2.0 to 0.2.5, criterion 0.8.1 to 0.8.2, proptest 1.9.0 to 1.10.0 ([`ba54b9f`](https://github.com/Dicklesworthstone/toon_rust/commit/ba54b9f810b3fd49d8abc0b8c7e08905a6b7ff8e))

### CI

- Replace deprecated `macos-13` runner with `macos-15-intel` for x86_64-apple-darwin builds; the deprecation had caused v0.2.0 to ship with zero binary assets ([`945ef71`](https://github.com/Dicklesworthstone/toon_rust/commit/945ef71f64851b90857f37b0857828dd8c1405bb))

### Licensing

- Update license to MIT with OpenAI/Anthropic Rider ([`fe9d0ba`](https://github.com/Dicklesworthstone/toon_rust/commit/fe9d0bac73106c8dbd9c58381733ffb5b81c1d1c))
- Update README license references ([`d48c5b5`](https://github.com/Dicklesworthstone/toon_rust/commit/d48c5b55caeedc99368a1461535b7e0c143afe06))

---

## [v0.2.0](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.2.0) -- 2026-02-15 **(released)**

Tag: [`c1c6dc6`](https://github.com/Dicklesworthstone/toon_rust/commit/c1c6dc6d08d5cbe30a90a9fc37e754d4af7aab7a) |
[Compare v0.1.2...v0.2.0](https://github.com/Dicklesworthstone/toon_rust/compare/v0.1.2...v0.2.0) |
[GitHub Release](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.2.0)

No release assets were published for this version (the deprecated `macos-13` runner blocked the release job -- fixed in v0.2.1).

### Breaking: Binary Renamed `tru` to `toon`

- **Binary renamed from `tru` to `toon`** to match the format name. All CLI examples, install scripts, release workflow asset names, and documentation updated throughout ([`336bbe0`](https://github.com/Dicklesworthstone/toon_rust/commit/336bbe0a622f5ca8a1a9b5145cf197349a14f67e)).
- Crate name on crates.io remains `tru` because `toon-rust` was already taken (Cargo normalizes `_` to `-`). The library is imported as `use toon::...` ([`c1c6dc6`](https://github.com/Dicklesworthstone/toon_rust/commit/c1c6dc6d08d5cbe30a90a9fc37e754d4af7aab7a)).

### crates.io Publishing

- Initial publication to crates.io as `tru` ([`9347178`](https://github.com/Dicklesworthstone/toon_rust/commit/9347178829986d89faa41836733c7126792b6b60))

### Distribution

- Add `repository_dispatch` triggers so homebrew-tap and scoop-bucket auto-update on release (skips pre-releases) ([`032ff3c`](https://github.com/Dicklesworthstone/toon_rust/commit/032ff3cb8e5f8d3489c134edf99959543304210d))

### Dependencies

- Bump asupersync dependency to 0.2.0 ([`48ede01`](https://github.com/Dicklesworthstone/toon_rust/commit/48ede0130e46de55770bfcdb647b39a984eddf3d))

### Code Quality

- Normalize method chain formatting in integration tests ([`35ec231`](https://github.com/Dicklesworthstone/toon_rust/commit/35ec231f0e1dccd70eb715a71f4e9f084a7b6c39))

---

## [v0.1.2](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.1.2) -- 2026-02-05 **(released)**

Tag: [`c3c6f86`](https://github.com/Dicklesworthstone/toon_rust/commit/c3c6f86ff04faeb1c03c6a6f2f9265b4c8df4b8a) |
[Compare v0.1.1...v0.1.2](https://github.com/Dicklesworthstone/toon_rust/compare/v0.1.1...v0.1.2) |
[GitHub Release](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.1.2)

Largest release by diff volume (+3,834 / -223 lines across 28 files). Release assets: `tru-linux-amd64.tar.xz` + SHA256 (only Linux amd64 built successfully).

### Async Streaming Pipeline

Full async encode/decode pipeline gated behind the optional `async-stream` feature flag, using the `asupersync` runtime:

- **Async streaming decode** with `AsyncDecodeStream` that yields `JsonStreamEvent` items from TOON lines, with cancellation support via capability context, cooperative scheduling with yield points, and backpressure handling for slow consumers ([`7fec77b`](https://github.com/Dicklesworthstone/toon_rust/commit/7fec77bf431b02d57fa751db10d3bccae882a405), [`ba1fcb8`](https://github.com/Dicklesworthstone/toon_rust/commit/ba1fcb864442849e3bee7245c73b4ff0a28e0822), [`9dbffe0`](https://github.com/Dicklesworthstone/toon_rust/commit/9dbffe0b23c1d3a9d884cb8137f7e204d937412f))
- **Async streaming encode** module with Tokio `AsyncWrite` support for event-by-event encoding, enabling memory-efficient output of large TOON documents ([`bd80f95`](https://github.com/Dicklesworthstone/toon_rust/commit/bd80f954eaf73d9e07f4e72a09104b9ac5af7923))
- Dual `try_decode_stream()` implementation: true async when feature enabled, sync wrapper otherwise

### WebAssembly Support

- Add `wasm` feature flag with `wasm-bindgen`, `js-sys`, `console_error_panic_hook` ([`9432060`](https://github.com/Dicklesworthstone/toon_rust/commit/94320608746e77d8af7cfd3ae7b29318029cbcbf), [`f29bb5b`](https://github.com/Dicklesworthstone/toon_rust/commit/f29bb5bbf5821ecccb1cde2c483e35dcedf479c8))
- JavaScript-friendly bindings: `encode`, `decode`, `encode_with_options`, `decode_with_options`, `decode_pretty`, `version()`
- Verified build for `wasm32-unknown-unknown` target
- Panic hook for better browser/edge error messages

### Error Type System

- Replace basic `Message` variant with rich, structured error types ([`8af2bff`](https://github.com/Dicklesworthstone/toon_rust/commit/8af2bff887af164f82d68c5d0ff6cf6a5faeb798)):
  - `Parse { line, message }` -- parse errors with line numbers
  - `Validation { line, message }` -- strict-mode violations
  - `EventStream { message }` -- event processing errors
  - `PathExpansion { path, message }` -- path resolution errors
  - `Io { operation, path, source }` -- I/O with context
  - `Json { message }` -- JSON serialization errors
- Constructor methods: `parse()`, `unterminated_string()`, `missing_colon_after_key()`, `io_read()`, `io_write()`, `validation()`, etc.
- Backward-compatible `Message` variant preserved

### CLI Improvements

- Restructured CLI command handling with better error messages and argument parsing ([`5f9bb36`](https://github.com/Dicklesworthstone/toon_rust/commit/5f9bb361da31237bc4f504846d8db1abd8dc8ce5))
- Comprehensive CLI integration test suite: encoding, decoding, round-trip, error cases (597 lines) ([`5f9bb36`](https://github.com/Dicklesworthstone/toon_rust/commit/5f9bb361da31237bc4f504846d8db1abd8dc8ce5))

### Bug Fixes

- Use saturating arithmetic to prevent integer overflow in indentation calculations for deeply nested structures ([`1787f05`](https://github.com/Dicklesworthstone/toon_rust/commit/1787f05fd3030bb2673653ae84f0d89708d1c700))
- Revert incorrect `tru` to `toon` binary rename, then fix asset names properly to align with install.sh/homebrew/scoop conventions ([`dbdd080`](https://github.com/Dicklesworthstone/toon_rust/commit/dbdd0809657b7943c505b584d945bd7c3dc530d4), [`82f35f8`](https://github.com/Dicklesworthstone/toon_rust/commit/82f35f8afbb3d17893b0afe84f35df199e33aaf3), [`c9a651e`](https://github.com/Dicklesworthstone/toon_rust/commit/c9a651e899759795e172ce89ca81b632cf794293))
- Fix `AsyncDecodeStream` `poll_next` to properly loop until events are exhausted instead of early returning on `None` ([`f29bb5b`](https://github.com/Dicklesworthstone/toon_rust/commit/f29bb5bbf5821ecccb1cde2c483e35dcedf479c8))

### Benchmarks and Testing

- Criterion benchmark suite: encode/decode at small/medium/large sizes, nested structures (100+ levels), tabular array detection, key folding overhead, compression ratio analysis vs raw JSON ([`f29bb5b`](https://github.com/Dicklesworthstone/toon_rust/commit/f29bb5bbf5821ecccb1cde2c483e35dcedf479c8))
  - Results: 48-52% size reduction on tabular data
- 43 edge-case tests: Unicode (emoji, RTL, zero-width, combining, surrogates), deep nesting (100+ levels), long strings/keys (10K+ chars), numeric extremes (MAX/MIN, subnormals, NaN, infinity), delimiter edge cases, key folding conflicts, property-based tests with proptest ([`f29bb5b`](https://github.com/Dicklesworthstone/toon_rust/commit/f29bb5bbf5821ecccb1cde2c483e35dcedf479c8))

### CI and Release Infrastructure

- Comprehensive release workflow overhaul ([`788589d`](https://github.com/Dicklesworthstone/toon_rust/commit/788589d7b11a8bf2c88e364befe63d87eb890eba)):
  - New `plan` job with semver validation and clear error messages
  - Cross-platform builds: linux-gnu, linux-musl, darwin-x86_64, darwin-aarch64, windows-msvc
  - Separate `publish-crates` job for crates.io (pre-releases skipped)
  - SHA256 checksums for all artifacts + consolidated `checksums.txt`
  - Artifact attestations enabled
  - Timeout limits on all jobs
- Add ACFS notification workflows for installer changes ([`145987e`](https://github.com/Dicklesworthstone/toon_rust/commit/145987e288ed5d73e55c610a658a3c59804a188d), [`25ced07`](https://github.com/Dicklesworthstone/toon_rust/commit/25ced076fd802cf70ed5dc0356693831812b9357))
- Fix release workflow issues: build tag on dispatch, switch macOS runner, allow provenance attestations, correct version parsing ([`0540a23`](https://github.com/Dicklesworthstone/toon_rust/commit/0540a23b06f404943048ed3180bba48f976807e0), [`12bd0fb`](https://github.com/Dicklesworthstone/toon_rust/commit/12bd0fba71bfb3f9d74e7cb0f30a2303ba244593), [`a18400a`](https://github.com/Dicklesworthstone/toon_rust/commit/a18400a50131f1f5e60f578d030c5d1d27de8e55))

### Dependencies

- Relax chrono pin for workspace compatibility ([`3573f2a`](https://github.com/Dicklesworthstone/toon_rust/commit/3573f2a2aae9383705dc571cb396924d3f1fb611))
- Streamline dependencies and update Cargo manifest ([`2efeec0`](https://github.com/Dicklesworthstone/toon_rust/commit/2efeec049897fd71e60b4dbaa78b96f24851bf5c))

### Docs

- Fix rustdoc link syntax for encode function ([`3907c12`](https://github.com/Dicklesworthstone/toon_rust/commit/3907c12e9db5ce274bff2da8b9f4fe355a431713))

---

## [v0.1.1](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.1.1) -- 2026-01-24 **(released)**

Tag: [`7d763be`](https://github.com/Dicklesworthstone/toon_rust/commit/7d763be7a9655c0306991c4cd40cb2aee14db27a) |
[Compare v0.1.0...v0.1.1](https://github.com/Dicklesworthstone/toon_rust/compare/v0.1.0...v0.1.1) |
[GitHub Release](https://github.com/Dicklesworthstone/toon_rust/releases/tag/v0.1.1)

Release assets: `tru-{darwin,linux,windows}-{amd64,arm64}.tar.xz` + SHA256 checksums (5 platforms).

### Breaking: CLI Binary Renamed to `tru`

- **Binary renamed from `toon_rust` to `tru`** to avoid conflict with coreutils `tr` while keeping the name short and memorable ([`57bf618`](https://github.com/Dicklesworthstone/toon_rust/commit/57bf618f3a1dbe410eccacaf6bb5fdebf161a784), [`7d763be`](https://github.com/Dicklesworthstone/toon_rust/commit/7d763be7a9655c0306991c4cd40cb2aee14db27a))
- Release assets aligned with `tru-<platform>-<arch>` naming convention

### Streaming Encode Events

- Implement `encode_stream_events` for streaming JSON event generation -- walks a `JsonValue` tree and emits `JsonStreamEvent` items, complementing the existing `decode_stream_sync` ([`587198e`](https://github.com/Dicklesworthstone/toon_rust/commit/587198e5177e8ec257b00a4d2ff704a65e24764a))
  - Normalizes input, applies optional replacers, marks keys requiring quoting via `was_quoted` field
  - 5 unit tests: primitives, objects, arrays, quoted keys, roundtrip

### Decoder and Parser Improvements

- Expand CLI capabilities and improve decoding ([`2878f24`](https://github.com/Dicklesworthstone/toon_rust/commit/2878f24ce8dfe5092e074765264e1a6ec4258702))
- Improve decoder and parser robustness ([`053fe12`](https://github.com/Dicklesworthstone/toon_rust/commit/053fe123db35d79d145a8753bdd0d9ea48cdd5a9))
- Continued implementation refinements across encode/decode ([`a40ea58`](https://github.com/Dicklesworthstone/toon_rust/commit/a40ea58bd9e8064fbbba83d97c5eca962c6b70e4), [`dce3617`](https://github.com/Dicklesworthstone/toon_rust/commit/dce361748e1944b10f10190c2bd7b31ed1ee5df4), [`f79f45e`](https://github.com/Dicklesworthstone/toon_rust/commit/f79f45ea07e3a57a2ed5f32a7ca36a61409556cc))

### Testing

- Extend decode test coverage and document module structure ([`9b75058`](https://github.com/Dicklesworthstone/toon_rust/commit/9b75058c81309099a17ab6dbd8adaca8d6ed9a9a))

### Documentation

- Add integration guide for `toon_rust` library usage ([`5e122d5`](https://github.com/Dicklesworthstone/toon_rust/commit/5e122d51f0c78c227d73bae4cc322b0bb197038f))
- Add MIT License ([`cecf69f`](https://github.com/Dicklesworthstone/toon_rust/commit/cecf69f9a90e98d54b825c6244c7eaf0484746f1))

---

## [v0.1.0](https://github.com/Dicklesworthstone/toon_rust/tree/v0.1.0) -- 2026-01-21 **(tag only)**

Tag: [`49a081d`](https://github.com/Dicklesworthstone/toon_rust/commit/49a081d0941ff45f620ffebddad01f49657bf953) |
No GitHub Release was created for this tag.

Initial working release. Core implementation, full spec conformance, CLI, performance optimizations, CI/CD, and comprehensive tests landed in a two-day sprint (Jan 19-21).

### Encode Pipeline

Spec-compliant TOON encoder in `src/encode/` ([`dbb79bc`](https://github.com/Dicklesworthstone/toon_rust/commit/dbb79bc9043601f5ed593c22b555b25ff4bc0ff4)):

- **Tabular array detection**: automatically recognizes homogeneous object arrays with identical primitive-valued keys and emits compact header+row format
- **Key folding**: collapses single-key nested object chains into dotted paths (`config.database.host: localhost`), with safe mode to prevent sibling conflicts
- **Primitive encoding**: strings (with smart quoting/escaping), numbers, booleans, null
- **Custom replacer support**: transform values during encoding via user-supplied functions
- **Normalization**: converts `serde_json::Value` to internal `JsonValue` representation, handling BigInt, Date, Set/Map edge cases

### Decode Pipeline

Event-based streaming TOON decoder in `src/decode/` ([`dbb79bc`](https://github.com/Dicklesworthstone/toon_rust/commit/dbb79bc9043601f5ed593c22b555b25ff4bc0ff4)):

- **Scanner**: character-level tokenization with lookahead, indentation detection
- **Parser**: token-to-event conversion with indentation-stack-based nesting
- **Event builder**: reconstructs JSON tree from `ObjectStart`/`ObjectEnd`, `ArrayStart`/`ArrayEnd`, `Key`, `Value` events
- **Path expansion**: converts dotted keys back to nested objects (`a.b.c: 1` becomes `{"a":{"b":{"c":1}}}`)
- **Validation**: strict mode enforces tab prohibition, blank line rules, declared array lengths
- **Streaming architecture**: constant-memory processing of arbitrarily large TOON files

### CLI

Command-line interface mirroring the reference `toon` tool ([`0799701`](https://github.com/Dicklesworthstone/toon_rust/commit/079970183a769688da8611eaa0596b2b9eaf337d)):

- Auto-detection: `.json` input triggers encode, `.toon` triggers decode
- Flags: `-e`/`-d`, `-o`, `--delimiter`, `--indent`, `--no-strict`, `--key-folding`, `--flatten-depth`, `--expand-paths`, `--stats`
- Stdin/stdout streaming support
- clap-based argument parsing with full `--help`

### Performance

- Pre-allocated buffers and reduced allocations ([`811a2d9`](https://github.com/Dicklesworthstone/toon_rust/commit/811a2d97921e164bc0117af0c3ff3be1ef1ac43a)):
  - Indented line builders to avoid `format!` allocations in hot encoder paths
  - Single pre-allocated buffer with size estimation in `json_stringify`
  - Pre-estimated Vec capacity in `parse_delimited_values`
  - Skip string allocation for blank scanner lines
- Benchmark results vs Node.js reference: **4-27x faster encode**, **9x faster decode**, **60x faster startup**, **8x less memory**

### Testing

- Comprehensive encode/decode test suite covering primitives, objects, arrays, tabular formatting, key folding, strict/non-strict modes, edge cases ([`0132c5b`](https://github.com/Dicklesworthstone/toon_rust/commit/0132c5bbef63d5898a0f2807ca5fd3be208dc5bf))
- Full spec conformance documented in FEATURE_PARITY.md

### Build and CI

- GitHub Actions CI/CD: lint, test, build on push to main/master ([`d005cdc`](https://github.com/Dicklesworthstone/toon_rust/commit/d005cdcc660df20f39d3dab0667f2c55a224b5ce), [`652886a`](https://github.com/Dicklesworthstone/toon_rust/commit/652886a0a5e9042a20678e44b127cc8ff99a5616))
- Upgrade vergen-gix to 9.1 to fix security audit finding ([`16f2ac5`](https://github.com/Dicklesworthstone/toon_rust/commit/16f2ac504f700b571446d3dc7132a47431bef6b3))
- Add crates.io publish job in release workflow ([`bfcaa18`](https://github.com/Dicklesworthstone/toon_rust/commit/bfcaa180676841c5d3de2c36a9919bd9866a6985))
- Fix release workflow: correct publish condition, use free-tier runners, shell check for registry token ([`81d747c`](https://github.com/Dicklesworthstone/toon_rust/commit/81d747c147c9b658ed65cb7bd3462998f3e681e7), [`8512685`](https://github.com/Dicklesworthstone/toon_rust/commit/8512685c9bcd1d6ef88c4d664bb57ff4f77c8179), [`49a081d`](https://github.com/Dicklesworthstone/toon_rust/commit/49a081d0941ff45f620ffebddad01f49657bf953))

---

## Pre-history

Initial commit: [`b78cc71`](https://github.com/Dicklesworthstone/toon_rust/commit/b78cc7107ca6f9efc0241ddf0df8f9c9e1c30edf) (2026-01-19) -- project setup for toon_rust port.

---

## Release Summary

| Version | Date | GitHub Release | Key Theme |
|---------|------|----------------|-----------|
| [v0.2.1](#v021----2026-02-22-released-latest) | 2026-02-22 | Yes (latest) | Security fix (RUSTSEC-2026-0009), MSRV bump to 1.88, dep upgrades |
| [v0.2.0](#v020----2026-02-15-released) | 2026-02-15 | Yes | Binary renamed `tru` to `toon`, crates.io publishing as `tru`, package manager dispatch |
| [v0.1.2](#v012----2026-02-05-released) | 2026-02-05 | Yes | Async streaming pipeline, WASM support, structured error types, benchmarks, edge-case tests |
| [v0.1.1](#v011----2026-01-24-released) | 2026-01-24 | Yes | Binary renamed to `tru`, streaming encode events, decoder/parser improvements |
| [v0.1.0](#v010----2026-01-21-tag-only) | 2026-01-21 | No | Initial implementation: encode/decode/CLI with full spec conformance, 4-27x perf vs Node.js |

## Binary Name History

The CLI binary name has changed across releases:

| Version | Binary Name | Crate Name (crates.io) | Library Import |
|---------|-------------|------------------------|----------------|
| v0.1.0 | `toon_rust` | -- | `use toon::...` |
| v0.1.1 | `tru` | -- | `use toon::...` |
| v0.1.2 | `tru` | -- | `use toon::...` |
| v0.2.0 | `toon` | `tru` | `use toon::...` |
| v0.2.1 | `toon` | `tru` | `use toon::...` |

The binary is now **`toon`**. The crate is published to crates.io as [`tru`](https://crates.io/crates/tru) (because `toon-rust` was already taken), but the library is always imported as `use toon::...`.
