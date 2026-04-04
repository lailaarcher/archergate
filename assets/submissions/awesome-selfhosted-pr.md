## PR: Add archergate-license-server to awesome-selfhosted

**Target repo:** github.com/awesome-selfhosted/awesome-selfhosted

**Section:** Software Development - IDE & Tools (or a new Licensing subsection)

**Entry to add:**

```markdown
- [Archergate License Server](https://github.com/lailaarcher/archergate) - Self-hosted license validation server for software developers. Machine-bound keys, offline validation, trial management. ([Source Code](https://github.com/lailaarcher/archergate)) `MIT` `Rust`
```

**PR title:** Add Archergate License Server

**PR body:**

Self-hosted license validation server for indie software developers. Manages machine-bound license keys, handles offline validation with signed caches, and supports 14-day trials. Single Rust binary with SQLite. No external dependencies.

Pairs with the Archergate client SDK (Rust, C, C++) but the server REST API works with any language.

- MIT licensed
- Rust + SQLite (single binary)
- Docker image available
