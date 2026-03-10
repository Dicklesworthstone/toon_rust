# Dependency Upgrade Log

**Date:** 2026-02-19  |  **Project:** toon_rust  |  **Language:** Rust

## Summary
- **Updated:** 11 (10 crates + 1 toolchain)  |  **Skipped:** 0  |  **Failed:** 0

## Toolchain

### Rust nightly: 1.95.0-nightly (a423f68a0 2026-02-13) -> 1.95.0-nightly (7f99507f5 2026-02-19)
- **Breaking:** None
- **Note:** Required uninstall/reinstall due to corrupted cross-compilation target state
- **Tests:** Passed

## Dependencies

### anyhow: 1.0.100 -> 1.0.102
- **Breaking:** None (patch)
- **Tests:** Passed

### asupersync: 0.2.0 -> 0.2.5
- **Breaking:** None (patch within 0.x)
- **Tests:** Passed

### clap: 4.5.54 -> 4.5.60
- **Breaking:** None (patch)
- **Tests:** Passed

### clap_complete: 4.5.65 -> 4.5.66
- **Breaking:** None (patch)
- **Tests:** Passed

### criterion: 0.8.1 -> 0.8.2
- **Breaking:** None (patch)
- **Tests:** Passed

### insta: 1.46.1 -> 1.46.3
- **Breaking:** None (patch)
- **Tests:** Passed

### predicates: 3.1.3 -> 3.1.4
- **Breaking:** None (patch)
- **Tests:** Passed

### proptest: 1.9.0 -> 1.10.0
- **Breaking:** None observed (minor version bump)
- **Tests:** Passed

### rand: 0.9.2 -> 0.10.0
- **Breaking:** Major API changes (trait renames, method renames, OsRng -> SysRng)
- **Impact:** None -- rand is an unused dev-dependency in this project (zero imports found)
- **Tests:** Passed

### tempfile: 3.24.0 -> 3.25.0
- **Breaking:** None (minor)
- **Tests:** Passed

## Validation

- `cargo check --all-targets`: Passed
- `cargo test`: 103/103 passed
- `cargo clippy --all-targets -- -D warnings`: Clean (0 warnings)
- `cargo fmt --check`: Clean
- `cargo outdated --root-deps-only`: "All dependencies are up to date"
