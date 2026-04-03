# Post to: Unity Discord (#asset-store-development), r/Unity3D, Unity Forums (Asset Store section)

---

**Title:** Open-source license SDK for Unity asset sellers — machine binding, offline validation, anti-tamper

**Body:**

I've been building a licensing SDK that lets you add machine-bound copy protection to your Unity assets. It's MIT licensed, free, and self-hostable.

The short version: your asset validates against the buyer's hardware. If someone shares the files, it won't run on a different machine. 30-day offline grace period so you're not punishing legitimate users with always-online requirements.

**What it does:**
- Machine binding (SHA-256 fingerprint from CPU + OS)
- 30-day offline validation (HMAC-signed local cache)
- 14-day trial system (no server needed)
- Tamper detection (3 independent verification paths)
- Self-hosted server included (Rust + SQLite, runs on a $5 VPS)

**Integration options:**
- REST API (works from C# / any language)
- Native C/C++ FFI (for native plugins)
- Rust SDK (if you're building native)

**How to get it:**
- crates.io: `cargo add archergate-license`
- GitHub: https://github.com/lailaarcher/archergate
- Pre-built binaries: https://github.com/lailaarcher/archergate/releases
- Or sign up at https://archergate.io/sdk and we'll email you download links + docs

It's in open beta. Looking for feedback from asset sellers who've dealt with piracy. What would make this actually useful for your workflow?

---

*Note: Adjust tone per platform. Reddit = casual. Unity Forums = more technical. Discord = shortest version with link.*
