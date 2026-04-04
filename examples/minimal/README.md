# Archergate Minimal Example

End-to-end proof that the SDK and validation server work together.

## What this does

1. Starts the Archergate license server in Docker
2. Creates a test API key and license
3. Runs a minimal Rust client that validates the license against the server
4. Prints PASS or FAIL

## Requirements

- Docker and docker-compose
- curl
- jq
- Rust 1.76+ (cargo)

## Run it

```bash
cd examples/minimal
./run.sh
```

Expected output:

```
Archergate Minimal Example
==========================

Validating license key: TEST-XXXX-YYYY-ZZZZ-WWWW
SUCCESS: License is valid!
  Machine fingerprint verified
  Offline grace period: 30 days
```

## What's inside

- **Cargo.toml** — minimal Rust project with archergate-license + tokio
- **src/main.rs** — single Rust binary that creates a client and validates a key
- **docker-compose.yml** — starts the license server with SQLite
- **run.sh** — orchestrates the entire flow (start server → create key → validate → stop server)

## This also serves as

- **E2E integration test** — proves SDK + server work together
- **Show HN demo** — "clone this repo, run `cd examples/minimal && ./run.sh`, see it work in 60 seconds"
- **Documentation** — shows exactly how to use the SDK in production code
