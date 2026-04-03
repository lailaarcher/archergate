# archergate-license

Machine-bound software licensing for indie developers.

Hardware-locked keys. 30-day offline validation. 14-day trials. Cryptographic tamper detection. Integrates in 30 minutes.

Built for desktop apps, creative tools, game assets, and plugins.

## Install

```
cargo add archergate-license
```

Or download pre-built binaries from [Releases](https://github.com/lailaarcher/archergate/releases).

## Three lines

```rust
use archergate_license::LicenseClient;

let client = LicenseClient::new("your-api-key", "com.you.app");
client.validate("XXXX-XXXX-XXXX-XXXX")?;
```

```c
#include "archergate_license.h"

AgLicenseClient* c = ag_license_new("your-api-key", "com.you.app");
ag_license_validate(c, license_key);
ag_license_free(c);
```

## What's inside

- **Machine binding** — SHA-256 fingerprint from CPU + OS install ID
- **Offline validation** — 30-day grace period with HMAC-signed cache
- **14-day trials** — No server, no signup, no tracking
- **Tamper detection** — Validation receipts, heartbeat counter, signed cache
- **Self-hosted server** — Axum + SQLite reference server included
- **Languages** — Rust, C, C++ (FFI), any language via REST API

## Self-hosted server

```
cargo install archergate-license-server
archergate-license-server serve --port 3100
```

## Built for

- VST / AU / AAX plugins
- Unity assets
- Blender addons
- Electron / Tauri desktop apps
- Adobe plugins
- Game mods
- Any native software

## License

MIT
