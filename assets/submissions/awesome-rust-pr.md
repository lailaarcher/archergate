## PR: Add archergate-license to awesome-rust

**Target repo:** github.com/rust-unofficial/awesome-rust

**Section:** Applications > Security / Libraries > Authentication

**Entry to add:**

```markdown
* [archergate-license](https://github.com/lailaarcher/archergate) - Machine-bound software licensing SDK. Locks license keys to hardware fingerprints. Offline validation, trial periods, tamper detection. Includes self-hosted validation server (Axum + SQLite). [![crates.io](https://img.shields.io/crates/v/archergate-license.svg)](https://crates.io/crates/archergate-license)
```

**PR title:** Add archergate-license (machine-bound software licensing SDK)

**PR body:**

Archergate is a licensing library for developers who ship compiled Rust binaries and need copy protection. It binds license keys to machine hardware (SHA-256 of CPU brand + OS install ID), caches validation offline for 30 days, and includes a self-hosted validation server.

- crates.io: https://crates.io/crates/archergate-license
- MIT licensed
- CI passing on Windows/macOS/Linux
- No unsafe code in the public API (FFI layer uses #[unsafe(no_mangle)] for C interop)
