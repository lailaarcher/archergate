# Archergate Licensed App Template

Scaffold a new Rust application with Archergate license protection built in.

```
cargo generate lailaarcher/archergate-template
```

Prompts for your app name, app ID, and server URL. Generates a project with licensing already wired up. Build and ship.

## After scaffolding

1. Set your API key: `export ARCHERGATE_API_KEY=your-key`
2. Build: `cargo build --release`
3. Run: `LICENSE_KEY=XXXX-XXXX ./target/release/your-app`

## What's included

- `archergate-license` dependency in Cargo.toml
- License validation in main.rs startup
- Trial mode fallback when no key is provided
- Environment variable config (no hardcoded keys)
