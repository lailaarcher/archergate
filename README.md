# Archergate License SDK

Machine-bound software licensing for indie developers.

Hardware-locked keys. 30-day offline validation. 14-day trials. Cryptographic tamper detection. Integrates in 30 minutes. MIT licensed. Self-hostable.

Built for VST plugins, Unity assets, Blender addons, Electron and Tauri desktop apps, Adobe extensions, game mods, and any native software that ships as a binary.

## The problem

You ship software. People share the files. Enterprise licensing costs enterprise money and locks you into someone else's platform. Rolling your own takes weeks. Most indie developers ship with no protection at all.

Archergate is a single library that validates a license key against the machine it was activated on. If the files get shared, they stop working on a different machine. Fair enforcement that respects legitimate users.

## Install

```
cargo add archergate-license
```

Pre-built binaries for Windows, macOS (Intel + Apple Silicon), and Linux: [Releases](https://github.com/lailaarcher/archergate/releases)

## Integration

### Rust

```rust
use archergate_license::LicenseClient;

let client = LicenseClient::new("your-api-key", "com.you.app");
client.validate("XXXX-XXXX-XXXX-XXXX")?;
```

### C

```c
#include "archergate_license.h"

AgLicenseClient* c = ag_license_new("your-api-key", "com.you.app");
int rc = ag_license_validate(c, license_key);
if (rc != AG_OK) {
    printf("License error: %s\n", ag_license_error_string(rc));
}
ag_license_free(c);
```

### C++ (RAII)

```cpp
#include "archergate_license.h"

archergate::License license("your-api-key", "com.you.app");
license.validate(license_key);
// Automatically freed when `license` goes out of scope.
```

### REST API (Python, C#, Go, anything)

```
POST /validate
Content-Type: application/json

{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256-of-cpu-and-os",
    "plugin_id": "com.you.app"
}
```

Returns `{ "valid": true }` or `{ "valid": false, "error": "expired" }`.

## What it does

**Machine binding.** SHA-256 fingerprint derived from CPU brand string and OS install identifier. Each license key is locked to the machine where it was first activated. Windows reads `MachineGuid` from the registry. macOS reads `IOPlatformUUID` via `ioreg`. Linux reads `/etc/machine-id`.

**Offline validation.** After one successful online check, the license is cached locally for 30 days. The cache file is HMAC-SHA256 signed so it cannot be edited or moved to another machine. Touring musicians, remote studios, air-gapped labs, unreliable hotel wifi. All handled.

**14-day trials.** Built into the SDK. No server call needed, no user signup, no email collection, no tracking. Start a trial, it writes a local file, it expires in 14 days. Simple.

**Tamper detection.** Three independent verification paths. HMAC-signed cache files. Validation receipts with timestamps and fingerprints. An atomic heartbeat counter that increments on every successful check. A cracker has to find and disable all three. Most will not bother.

**Self-hosted server.** A reference validation server ships with the SDK. Rust, Axum, SQLite. One binary, no external dependencies, runs on a $5/month VPS. Or use Archergate's hosted option when it launches.

## Self-hosted server

```
cargo install archergate-license-server
archergate-license-server serve --port 3100 --db ./licenses.db
```

Create an API key and generate a license:

```
archergate-license-server create-key --email you@example.com
archergate-license-server create-license --plugin com.you.app --max-machines 3 --api-key-id <key-id>
```

REST endpoints: `POST /validate`, `POST /activate`, `POST /licenses`, `GET /health`.

## Who this is for

**Audio plugin developers.** VST3, AU, AAX. Works with JUCE, iPlug2, or any framework that links C. Static library, no separate DLL to distribute.

**Unity asset sellers.** Call the REST API from C# in your asset's initialization. Machine binding stops casual redistribution of purchased assets.

**Blender addon developers.** Call the REST API from Python in your addon's `register()` function. Self-hosted server means no dependency on a third party.

**Desktop app developers.** Tauri, Electron, Qt, WPF, native Cocoa. If it runs on a desktop and you charge money for it, this works.

**Adobe plugin developers.** Premiere, After Effects, Photoshop. C++ integration through the FFI layer.

**Game mod and tool creators.** Protect paid mods, map editors, asset packs, trainer tools.

**Anyone shipping native software** who needs copy protection without vendor lock-in.

## Error codes

| Code | Constant | Meaning |
|------|----------|---------|
| -1 | `AG_OK` | License is valid |
| -2 | `AG_ERR_INVALID` | Key not found |
| -3 | `AG_ERR_EXPIRED` | License has expired |
| -4 | `AG_ERR_MACHINE_MISMATCH` | Wrong machine |
| -5 | `AG_ERR_NETWORK` | Server unreachable (falls back to cache) |
| -6 | `AG_ERR_TRIAL_EXPIRED` | 14-day trial is over |
| -7 | `AG_ERR_ACTIVATION_LIMIT` | Too many machines activated |

## How offline mode works

1. App calls `validate()` on startup.
2. SDK contacts the server. If valid, caches the response at `~/.archergate/licenses/{app_id}.json` with an HMAC-SHA256 signature in a `.sig` file.
3. Next startup: if the server is unreachable, the SDK reads the cache. If the signature is valid and the cache is less than 30 days old, validation succeeds.
4. If the cache is older than 30 days or the signature is wrong, validation fails.

No internet for a month? No problem. Cache tampered with? Caught.

## Repository structure

```
crates/
  archergate-license/         Rust SDK + C FFI + C++ wrapper
  archergate-license-server/  Self-hosted validation server
  archergate-core/            Creative memory engine (internal)
  archergate-tauri/           Tauri desktop app (internal)
assets/
  sdk-beta.html               Beta signup page
  outreach/                   Community post templates
api/
  sdk-beta.js                 Resend email integration (serverless)
scripts/
  build-release-binaries.sh   Cross-platform build script
```

## Tests

```
cargo test --workspace
```

52 tests across all crates. Unit tests, integration tests with mock HTTP servers, doc tests.

## Contributing

File issues on GitHub. Pull requests welcome. If you integrate Archergate into your software and want to share your experience, open a discussion.

## License

MIT. Use it, fork it, modify it, sell it, self-host it. No restrictions.

If Archergate disappears tomorrow, your licensing keeps working. That is the point.
