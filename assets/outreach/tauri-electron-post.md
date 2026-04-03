# Post to: Tauri Discord, r/rust, r/electronjs, HN (Show HN)

---

## Tauri Discord / r/rust version

**Title:** archergate-license — machine-bound software licensing SDK in Rust (open beta)

Built a licensing library for indie developers shipping desktop apps. If you're building with Tauri or distributing native Rust binaries, this drops in directly.

- Machine binding (SHA-256 from CPU + OS install ID)
- 30-day offline grace (HMAC-signed cache)
- 14-day trials (no server needed)
- Anti-tamper (validation receipts + heartbeat counter)
- Self-hosted server included (Axum + SQLite)
- C FFI + C++ RAII wrapper for native interop

```rust
use archergate_license::LicenseClient;

let client = LicenseClient::new("your-key", "com.you.app");
client.validate("XXXX-XXXX-XXXX-XXXX")?;
```

MIT licensed. On crates.io: `cargo add archergate-license`

GitHub: https://github.com/lailaarcher/archergate

Looking for feedback from anyone who's dealt with licensing for desktop apps. What's missing?

---

## Show HN version

**Title:** Show HN: Archergate – Open-source machine-bound licensing for indie software

**Body:**

I built a licensing SDK for indie developers who ship desktop software — plugins, creative tools, game assets, Electron/Tauri apps.

The problem: enterprise licensing solutions (iLok, PACE, Cryptlex) are expensive and lock you in. Rolling your own takes weeks. Most indie devs ship with no protection at all.

Archergate is a Rust SDK with C FFI that does machine-bound license validation. One online check, then 30 days offline. 14-day trials built in. HMAC-signed cache for tamper detection. MIT licensed. Self-hostable (Axum + SQLite server included).

Three lines to integrate:

```c
#include "archergate_license.h"
AgLicenseClient* c = ag_license_new("key", "com.you.app");
ag_license_validate(c, license_key);
```

Works from Rust, C, C++, or any language via REST API.

What I'm looking for: feedback from developers who've dealt with licensing/copy protection. What pain points did you hit? What would make this worth integrating?

- GitHub: https://github.com/lailaarcher/archergate
- crates.io: https://crates.io/crates/archergate-license
- Beta signup: https://archergate.io/sdk
