## Show HN Draft

**Title:** Show HN: Archergate -- machine-bound licensing SDK for indie devs (Rust, C, MIT)

**URL:** https://github.com/lailaarcher/archergate

**Text:**

We built a licensing library for developers who ship compiled software and need copy protection without paying per-seat fees or requiring hardware dongles.

The SDK is a static library (Rust with C FFI). You link it into your binary, add three lines of code, and it validates license keys at startup. Each key gets locked to the machine's hardware fingerprint (SHA-256 of CPU brand + OS install ID).

What it handles: machine binding, 30-day offline validation with HMAC-signed cache, 14-day trials, tamper detection (3 independent verification paths). Self-hosted validation server included (Axum + SQLite).

Built for audio plugins (VST3/AU/AAX), Unity/Unreal assets, Blender addons, Electron/Tauri desktop apps, and anything else that ships as a native binary.

The SDK and server are MIT licensed and free. We plan to charge $29/month for managed hosting later, but the self-hosted path stays free forever.

Also published an MCP server (npm: archergate-mcp-server) so AI coding assistants can generate license-protected code automatically.

Feedback welcome. Especially interested in hearing from anyone who has built their own licensing system and what we are missing.
