# Archergate for Tauri

Machine-bound software licensing for Tauri v2 desktop applications.

Users get a 14-day trial on first launch. When they purchase a key (from
Gumroad, Stripe, your own store, etc.) they enter it in your app. The key
is validated against the Archergate server and locked to their hardware. No
dongles, no third-party accounts.

## Install

### Rust side

Add the plugin crate to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-archergate = "0.1"
```

Register the plugin in `src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_archergate::init("your-api-key", "com.you.app"))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Replace `your-api-key` with the API key from your Archergate dashboard and
`com.you.app` with your application identifier.

### Frontend side

Install the JS bindings:

```
npm install tauri-plugin-archergate
```

Add the plugin to your Tauri config (`tauri.conf.json`):

```json
{
  "plugins": {
    "archergate": {}
  }
}
```

## Usage

### Check license status on startup

```typescript
import { getLicenseStatus, checkTrial } from "tauri-plugin-archergate";

async function boot() {
  const status = await getLicenseStatus();

  if (status.licensed) {
    console.log("Licensed.");
    return;
  }

  const trial = await checkTrial();
  if (trial.active) {
    console.log(`Trial: ${trial.days_remaining} days left.`);
    return;
  }

  // No license and trial expired -- show activation screen.
  showLicenseDialog();
}
```

### Activate a license key

```typescript
import { activateLicense } from "tauri-plugin-archergate";

async function onActivate(key: string) {
  const result = await activateLicense(key);

  if (result.success) {
    console.log(result.message); // "License activated and locked to this machine."
  } else {
    console.error(result.message);
  }
}
```

### Validate without activating

```typescript
import { validateLicense } from "tauri-plugin-archergate";

const result = await validateLicense("XXXX-XXXX-XXXX-XXXX");
console.log(result.valid, result.message);
```

## Full example: license key dialog (React)

```tsx
import { useState } from "react";
import {
  activateLicense,
  getLicenseStatus,
  checkTrial,
} from "tauri-plugin-archergate";

function LicenseGate({ children }: { children: React.ReactNode }) {
  const [licensed, setLicensed] = useState(false);
  const [trialDays, setTrialDays] = useState<number | null>(null);
  const [key, setKey] = useState("");
  const [error, setError] = useState("");

  // Check status once on mount.
  useState(() => {
    (async () => {
      const status = await getLicenseStatus();
      if (status.licensed) {
        setLicensed(true);
        return;
      }
      const trial = await checkTrial();
      if (trial.active) {
        setTrialDays(trial.days_remaining);
        setLicensed(true);
      }
    })();
  });

  if (licensed) {
    return (
      <>
        {trialDays !== null && (
          <div className="trial-banner">
            Trial: {trialDays} days remaining
          </div>
        )}
        {children}
      </>
    );
  }

  async function handleActivate() {
    setError("");
    const result = await activateLicense(key);
    if (result.success) {
      setLicensed(true);
      setTrialDays(null);
    } else {
      setError(result.message);
    }
  }

  return (
    <div className="license-dialog">
      <h2>Enter License Key</h2>
      <p>
        Your trial has expired. Enter a license key to continue using the app.
      </p>
      <input
        type="text"
        value={key}
        onChange={(e) => setKey(e.target.value)}
        placeholder="XXXX-XXXX-XXXX-XXXX"
      />
      <button onClick={handleActivate}>Activate</button>
      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default LicenseGate;
```

Wrap your app root:

```tsx
import LicenseGate from "./LicenseGate";

function App() {
  return (
    <LicenseGate>
      <MainApp />
    </LicenseGate>
  );
}
```

## How it works

1. On first launch, a 14-day trial starts automatically (no signup).
2. The user buys a key from your store.
3. They paste it into your license dialog.
4. The key is validated against the Archergate server and bound to their
   machine's hardware fingerprint.
5. The app works offline for 30 days between re-validation checks.
6. If the key is copied to another machine, validation fails.

## API reference

### Rust

```rust
// Initialize the plugin with your API key and app identifier.
tauri_plugin_archergate::init(api_key: &str, app_id: &str) -> TauriPlugin<R>
```

### TypeScript

| Function | Arguments | Returns |
|---|---|---|
| `validateLicense` | `key: string` | `Promise<ValidationResult>` |
| `checkTrial` | none | `Promise<TrialStatus>` |
| `getLicenseStatus` | none | `Promise<LicenseStatus>` |
| `activateLicense` | `key: string` | `Promise<ActivationResult>` |

### Types

```typescript
interface ValidationResult {
  valid: boolean;
  message: string;
  expires_at: number | null;
}

interface TrialStatus {
  active: boolean;
  days_remaining: number;
}

interface LicenseStatus {
  licensed: boolean;
  trial_active: boolean;
  trial_days_remaining: number;
  license_key: string | null;
}

interface ActivationResult {
  success: boolean;
  message: string;
  machine_locked: boolean;
}
```

## License

MIT
