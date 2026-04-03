# Publishing Archergate License SDK

Complete guide to publishing the SDK to crates.io and GitHub Releases.

## Prerequisites

- Rust installed (`rustup`) — Edition 2021 or later
- crates.io account with API token (https://crates.io/me)
- GitHub repository set up for archergate-license
- Build machines for cross-platform compilation (Windows, macOS, Linux)

## Phase 1: Prepare & Test (Completed ✓)

### ✓ Version & Metadata
- `crates/archergate-license/Cargo.toml`: version 0.1.0, Rust 2021 edition
- `crates/archergate-license-server/Cargo.toml`: version 0.1.0
- All crate-type declarations in place (lib, cdylib, staticlib)
- Repository/homepage/documentation URLs configured

### ✓ Dry-Run Testing
```bash
cd crates/archergate-license
cargo publish --dry-run  # ✓ PASSED

cd crates/archergate-license-server
cargo publish --dry-run  # ✓ PASSED
```

## Phase 2: Authenticate & Publish to crates.io

### Step 1: Authenticate with crates.io

```bash
cargo login
```

You'll need your crates.io API token from https://crates.io/me

### Step 2: Publish the client library

```bash
cd crates/archergate-license
cargo publish
```

This publishes to: https://crates.io/crates/archergate-license
Docs auto-generate at: https://docs.rs/archergate-license

### Step 3: Publish the server library

```bash
cd crates/archergate-license-server
cargo publish
```

This publishes to: https://crates.io/crates/archergate-license-server

## Phase 3: Build Pre-Compiled Binaries

Run on **each respective platform** (or CI/CD pipeline):

### Windows (MSVC)
```bash
scripts/build-release-binaries.sh windows
```
Creates: `archergate-license-v0.1.0-windows-x64.tar.gz`

### macOS (Intel + Apple Silicon)
```bash
scripts/build-release-binaries.sh macos
```
Creates: `archergate-license-v0.1.0-macos-universal.tar.gz`

### Linux (x86_64)
```bash
scripts/build-release-binaries.sh linux
```
Creates: `archergate-license-v0.1.0-linux-x64.tar.gz`

All archives include:
- Compiled binaries (.dll, .lib on Windows; .a on Unix)
- C header file (`archergate_license.h`)
- README with quick-start guide

The build script also generates `SHA256SUMS` for integrity verification.

## Phase 4: Create GitHub Release

### Option A: Using GitHub CLI
```bash
gh release create v0.1.0 \
    release-artifacts/*.tar.gz \
    release-artifacts/SHA256SUMS \
    --title "Archergate License v0.1.0" \
    --notes-file release-notes.md
```

### Option B: Manual via GitHub Web UI
1. Go to https://github.com/YOUR-ORG/archergate-license/releases/new
2. Tag version: `v0.1.0`
3. Release title: `Archergate License v0.1.0`
4. Release notes (see template below)
5. Upload `.tar.gz` files and `SHA256SUMS`
6. Publish release

### Release Notes Template

```markdown
## Archergate License SDK v0.1.0

Free, production-ready copy protection for indie VST/AU/AAX plugin developers.

### Features

- **Machine-bound licenses**: SHA-256 hardware fingerprint prevents license sharing
- **30-day offline grace period**: Works perfectly for touring producers without internet
- **14-day trial licensing**: No server calls, no signup required
- **Anti-tamper defense**: HMAC-signed cache, validation receipts, heartbeat counter
- **Cross-platform**: C, C++, Rust APIs. Works with JUCE.
- **Self-hosted or cloud**: Full control. Choose Archergate's server or run your own.
- **No lock-in**: MIT licensed. Open source. Zero vendor dependency.

### Downloads

**Pre-compiled binaries** (ready to link into your plugin):
- Windows (MSVC): [archergate-license-v0.1.0-windows-x64.tar.gz](https://github.com/YOUR-ORG/archergate-license/releases/download/v0.1.0/archergate-license-v0.1.0-windows-x64.tar.gz)
- macOS (Universal): [archergate-license-v0.1.0-macos-universal.tar.gz](https://github.com/YOUR-ORG/archergate-license/releases/download/v0.1.0/archergate-license-v0.1.0-macos-universal.tar.gz)
- Linux (x86_64): [archergate-license-v0.1.0-linux-x64.tar.gz](https://github.com/YOUR-ORG/archergate-license/releases/download/v0.1.0/archergate-license-v0.1.0-linux-x64.tar.gz)

**Or via Rust package manager:**
```
cargo add archergate-license
```

### Quick Start

**Rust:**
```rust
use archergate_license::LicenseClient;

let client = LicenseClient::new("your-api-key", "com.you.synth");
client.validate("LICENSE-KEY")?;
```

**C / JUCE:**
```c
#include "archergate_license.h"

AgLicenseClient* client = ag_license_new("your-api-key", "com.you.synth");
ag_license_validate(client, licenseKey);
ag_license_free(client);
```

### What's Included

Extract the archive and you'll get:
- `.lib` / `.a` — static library (link into your plugin)
- `.dll` — dynamic library (Windows only)
- `archergate_license.h` — C header with C++ wrapper
- `README.md` — Integration guide

### Links

- **GitHub**: https://github.com/YOUR-ORG/archergate-license
- **crates.io**: https://crates.io/crates/archergate-license
- **Docs**: https://docs.rs/archergate-license
- **Email**: hello@archergate.com

### Verification

Verify download integrity:
```bash
sha256sum -c SHA256SUMS
```

---

Built for indie developers. No exclusivity. No lock-in.
```

## Phase 5: Update Landing Page & Website

1. **Deploy landing page** as `/license/index.html`:
   - File: `assets/license-landing-page.html`
   - Update GitHub release URLs with actual release tag
   - Verify email form action endpoint is configured

2. **Update email template** with correct download links:
   - File: `assets/email-template.txt`
   - Replace GitHub release URLs with actual release page

3. **Email signature**:
   - Update `hello@archergate.com` with your actual support email
   - Consider adding "Contact" link to landing page

## Phase 6: Email Campaign

### Manual Option
- Copy `assets/email-template.txt`
- Send to indie plugin developer mailing list
- Include GitHub release link + crates.io link

### Automated Option (Recommended)
1. Set up form on `/license` (already in landing page HTML)
2. Configure backend to:
   - Capture name, email, plugin name, plugin ID
   - Send automated welcome email with SDK download links
   - Store signups for follow-up campaigns
3. Services: AWS SES, Mailgun, SendGrid, or custom solution

## Verification Checklist

- [ ] `cargo publish --dry-run` passes for both crates
- [ ] `cargo login` configured with valid API token
- [ ] Windows binaries built: `.dll` + `.lib` present
- [ ] macOS binaries built (if on macOS)
- [ ] Linux binaries built (if on Linux)
- [ ] `SHA256SUMS` generated for all archives
- [ ] GitHub release created with correct tag (v0.1.0)
- [ ] All `.tar.gz` files uploaded to release
- [ ] Release notes include download links
- [ ] Landing page deployed with correct URLs
- [ ] Email template updated with actual links
- [ ] Test email signup → verify automatic email received

## Archive Contents Verification

Extract any archive and verify:
```bash
tar -tzf archergate-license-v0.1.0-windows-x64.tar.gz
```

Expected files:
```
windows-x64/
├── archergate_license.dll      (2-3 MB)
├── archergate_license.lib      (35-40 MB static lib)
├── archergate_license.h        (5-6 KB header)
└── README.md                   (reference guide)
```

## Future Releases

For v0.2.0+:
1. Update `Cargo.toml` versions in both crates
2. Update `CHANGELOG.md` with changes
3. Test: `cargo publish --dry-run`
4. Publish: `cargo publish` (both crates)
5. Build binaries on each platform
6. Create GitHub release with new version tag
7. Update landing page + email template links
8. (Optional) Send announcement email to signup list

---

**Note**: Replace `YOUR-ORG` and email addresses with actual values before publishing.
