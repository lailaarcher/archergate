# archergate-license

License management for indie software developers.

Machine binding. Offline grace periods. 14-day trials. Anti-tamper.

Works from Rust, C, C++, or any language via REST API.

## What it does

- Validates license keys against the Archergate server (or your own)
- Binds licenses to machines via hardware fingerprint (CPU + OS install ID)
- Works offline for 30 days after last validation (touring producers stay unblocked)
- 14-day trials with zero server calls
- HMAC-signed cache files detect manual tampering
- Validation receipts make binary patching harder

## Rust integration

### Cargo.toml

```toml
[dependencies]
archergate-license = "0.1"
```

### Validate on startup

```rust
use archergate_license::{LicenseClient, LicenseError};

let client = LicenseClient::new("your-api-key", "com.yourname.synth");

match client.validate("XXXX-XXXX-XXXX-XXXX") {
    Ok(()) => {
        // License valid. Run normally.
    }
    Err(LicenseError::Invalid) => {
        // Bad key. Show registration dialog.
    }
    Err(LicenseError::Expired) => {
        // Time to renew.
    }
    Err(LicenseError::MachineMismatch) => {
        // Different machine. Show deactivation instructions.
    }
    Err(LicenseError::NetworkError(_)) => {
        // Offline and no cache. Degrade gracefully or warn.
    }
    Err(LicenseError::TrialExpired) => {
        // Trial over. Show purchase link.
    }
    Err(LicenseError::ActivationLimitReached) => {
        // Too many machines.
    }
}
```

### Trial mode

```rust
match client.start_trial() {
    Ok(trial) => println!("{} days left", trial.days_remaining),
    Err(LicenseError::TrialExpired) => { /* show purchase prompt */ }
    Err(e) => eprintln!("{e}"),
}
```

### Anti-tamper (defense in depth)

```rust
// Get a cryptographic receipt proving validation ran
let receipt = client.validate_with_receipt("LICENSE-KEY").unwrap();

// Check it later (e.g. in your audio callback, on a timer, wherever)
let fp = LicenseClient::machine_fingerprint();
if !receipt.verify("LICENSE-KEY", &fp, 86400) {
    // Receipt invalid or too old — someone patched the binary
}

// Or check the global heartbeat counter
if archergate_license::integrity::heartbeat_count() == 0 {
    // validate() never ran — bypass detected
}
```

### Complete example (20 lines)

```rust
use archergate_license::{LicenseClient, LicenseError};

fn check_license() -> bool {
    let client = LicenseClient::new("ag_key_abc123", "com.yourname.synth");
    let key = load_saved_key();

    if let Some(key) = key {
        return client.validate(&key).is_ok();
    }

    match client.start_trial() {
        Ok(t) if t.days_remaining > 0 => true,
        _ => false,
    }
}

fn load_saved_key() -> Option<String> {
    // Read from your plugin's settings file
    None
}
```

---

## C / C++ integration

This is what most indie software developers need.

### Build the library

```bash
cargo build --release -p archergate-license
```

Produces:
- **Windows**: `target/release/archergate_license.dll` + `archergate_license.dll.lib`
- **macOS**: `target/release/libarchergate_license.dylib` + `libarchergate_license.a`
- **Linux**: `target/release/libarchergate_license.so` + `libarchergate_license.a`

### Include the header

Copy `include/archergate_license.h` into your JUCE project.

### Link in CMakeLists.txt

```cmake
target_include_directories(YourPlugin PRIVATE path/to/archergate-license/include)
target_link_libraries(YourPlugin PRIVATE path/to/libarchergate_license.a)

# On Windows, also link:
# ws2_32 userenv bcrypt ntdll
```

### Use in your PluginProcessor

```cpp
#include "archergate_license.h"

class MyPluginProcessor : public juce::AudioProcessor {
public:
    MyPluginProcessor() {
        // C API
        auto* client = ag_license_new("your-api-key", "com.yourname.synth");
        int rc = ag_license_validate(client, getSavedLicenseKey());
        if (rc != AG_OK) {
            DBG("License error: " << ag_license_error_string(rc));
        }
        ag_license_free(client);
    }
};
```

### Or use the C++ wrapper (included in the same header)

```cpp
#include "archergate_license.h"

class MyPluginProcessor : public juce::AudioProcessor {
public:
    MyPluginProcessor() {
        try {
            archergate::License license("your-api-key", "com.yourname.synth");
            license.validate(getSavedLicenseKey());
        } catch (const archergate::LicenseException& e) {
            DBG("License error: " << e.what());
            // e.code has the AG_ERR_* constant
        }
    }
};
```

### Trial mode in C++

```cpp
archergate::License license("your-api-key", "com.yourname.synth");
try {
    uint32_t daysLeft = license.startTrial();
    DBG("Trial: " << daysLeft << " days remaining");
} catch (const archergate::LicenseException& e) {
    if (e.code == AG_ERR_TRIAL_EXPIRED) {
        showPurchaseDialog();
    }
}
```

### Error codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `AG_OK` | License valid |
| -1 | `AG_ERR_INVALID` | Key not recognized |
| -2 | `AG_ERR_EXPIRED` | License expired |
| -3 | `AG_ERR_MACHINE_MISMATCH` | Wrong machine |
| -4 | `AG_ERR_NETWORK` | No internet + no cache |
| -5 | `AG_ERR_TRIAL_EXPIRED` | Trial period over |
| -6 | `AG_ERR_ACTIVATION_LIMIT` | Too many machines |

---

## Self-hosted server

The server is a single Rust binary with SQLite. No Postgres, no Redis, no Docker required.

### Setup

```bash
cargo build --release -p archergate-license-server

# Create an API key
./archergate-license-server create-key --email you@example.com
# → ag_key_abc123... (save this)

# Create a license
./archergate-license-server create-license \
  --plugin com.yourname.synth \
  --email customer@example.com \
  --max-machines 3 \
  --api-key-id <the-id-from-above>

# Run the server
./archergate-license-server serve --port 3100
```

### Point the SDK at your server

```rust
let client = LicenseClient::new("ag_key_abc123", "com.yourname.synth")
    .with_api_url("https://license.yoursite.com");
```

Or in C:
```c
AgLicenseClient* client = ag_license_new_with_url(
    "ag_key_abc123",
    "com.yourname.synth",
    "https://license.yoursite.com"
);
```

### API endpoints

```
POST /validate
Body: { "license_key": "...", "machine_fingerprint": "...", "plugin_id": "..." }
→ { "valid": true, "expires_at": "2025-12-31T00:00:00Z" }
→ { "valid": false, "error": "expired" | "invalid" | "machine_mismatch" }

POST /activate
Body: { "license_key": "...", "machine_fingerprint": "...", "plugin_id": "...", "email": "..." }
→ { "token": "...", "offline_token": "..." }

POST /licenses (admin)
Body: { "plugin_id": "...", "email": "...", "max_machines": 3 }
→ { "license_key": "XXXX-XXXX-XXXX-XXXX", ... }

GET /health
→ { "status": "ok" }
```

---

## How offline mode works

1. Plugin calls `validate()` on startup.
2. If the server is reachable, the response is cached at `~/.archergate/licenses/{plugin_id}.json` with an HMAC signature.
3. If the server is unreachable and the cache is under 30 days old, `validate()` returns `Ok`.
4. After 30 days without server contact, `validate()` returns `NetworkError`.

The 30-day window means touring producers with spotty internet aren't locked out during a show.

## How anti-tamper works

The SDK has multiple independent defense layers:

1. **Signed cache files**: Each `.json` cache has a `.sig` companion. Editing the JSON (e.g. extending `expires_at`) invalidates the HMAC.
2. **Validation receipts**: `validate_with_receipt()` returns a cryptographic proof that the check actually ran. You can verify this at any point in your code.
3. **Heartbeat counter**: A global atomic counter increments every time validation runs. If it's 0, the function was never called.
4. **Machine fingerprint binding**: Licenses are tied to hardware. Copying the cache file to another machine doesn't work.

None of this stops a determined reverse engineer with a debugger. But it stops casual crackers who hexedit a single byte to flip a bool, and it makes automated cracking tools fail. That's the realistic bar for indie plugins.

## License

MIT
