# Post to: KVR Audio Developer Forum, Gearspace (DSP & Plug-in Development), r/AudioPlugins, JUCE Forum

---

**Title:** Free open-source license SDK for indie plugin developers — machine binding, offline validation, JUCE compatible

**Body:**

Built a licensing SDK specifically for indie audio plugin developers. It's MIT licensed, free forever, and self-hostable.

I know the options right now are limited: iLok (expensive, vendor lock-in), PACE (same), or roll your own (weeks of work). This is the third option — a production-ready SDK you can integrate in 30 minutes.

**What it does:**
- Machine-bound keys (SHA-256 fingerprint from CPU + OS)
- 30-day offline validation (HMAC-signed local cache — works during tours, flights, remote sessions)
- 14-day trial system (no server call needed)
- Anti-tamper detection (HMAC cache signatures, validation receipts, heartbeat counter)
- Self-hosted server (Rust + SQLite, single binary, runs on a $5 VPS)

**Integration (C++ / JUCE):**
```cpp
#include "archergate_license.h"

// In your PluginProcessor constructor:
archergate::License license("your-api-key", "com.you.synth");
license.validate(storedLicenseKey);
```

Static library links directly into your plugin binary. No separate DLL.

**Also works from:** Rust (native), C (FFI), any language via REST API.

**Links:**
- GitHub: https://github.com/lailaarcher/archergate
- crates.io: `cargo add archergate-license`
- Pre-built binaries: https://github.com/lailaarcher/archergate/releases
- Beta signup: https://archergate.io/sdk

Open beta. Looking for feedback from plugin devs who've dealt with copy protection. What would you change?
