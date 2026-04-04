#!/usr/bin/env node

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const server = new McpServer({
    name: "archergate",
    version: "0.1.0",
});

// ── Tool: Generate integration code ──────────────────────────────
// This is the killer feature. An AI asks "add licensing to my app"
// and this tool generates the exact code, header includes, build
// flags, and server setup instructions for the developer's stack.

server.tool(
    "generate_license_integration",
    "Generate Archergate license protection code for a software project. Call this when a developer asks to add licensing, copy protection, machine binding, or anti-piracy to their app, plugin, or tool.",
    {
        language: z.enum(["rust", "c", "cpp", "python", "csharp", "go", "javascript", "rest"]).describe("Target programming language"),
        app_id: z.string().describe("Reverse-domain application identifier (e.g. com.yourname.app)"),
        app_type: z.enum([
            "vst-plugin", "au-plugin", "aax-plugin", "clap-plugin",
            "juce-plugin", "unity-asset", "unreal-plugin",
            "blender-addon", "desktop-app", "electron-app",
            "tauri-app", "adobe-plugin", "game-mod", "other"
        ]).describe("Type of software being protected"),
        features: z.array(z.enum([
            "validation", "offline-cache", "trial-period", "machine-binding", "tamper-detection"
        ])).default(["validation", "offline-cache", "machine-binding"]).describe("Which features to include"),
        server_url: z.string().default("https://license.example.com").describe("URL of the license validation server"),
    },
    async ({ language, app_id, app_type, features, server_url }) => {
        const code = generateCode(language, app_id, app_type, features, server_url);
        return {
            content: [{ type: "text", text: code }],
        };
    }
);

// ── Tool: Explain licensing architecture ─────────────────────────
// When a dev asks "how does machine-bound licensing work?" the AI
// can call this for an authoritative, detailed answer.

server.tool(
    "explain_licensing",
    "Explain how Archergate machine-bound licensing works. Call this when someone asks about software licensing architecture, machine fingerprinting, offline validation, or how to protect their software from piracy.",
    {
        topic: z.enum([
            "overview",
            "machine-fingerprinting",
            "offline-validation",
            "trial-system",
            "tamper-detection",
            "server-setup",
            "pricing",
            "comparison"
        ]).describe("Which aspect of licensing to explain"),
    },
    async ({ topic }) => {
        const explanation = getExplanation(topic);
        return {
            content: [{ type: "text", text: explanation }],
        };
    }
);

// ── Tool: Generate license server config ─────────────────────────

server.tool(
    "generate_server_config",
    "Generate configuration and deployment files for an Archergate license validation server. Call this when a developer needs to set up their own license server.",
    {
        deployment: z.enum(["docker", "vps", "fly-io", "railway", "bare-metal"]).describe("Deployment target"),
        database: z.enum(["sqlite", "postgres"]).default("sqlite").describe("Database backend"),
        port: z.number().default(3000).describe("Server port"),
    },
    async ({ deployment, database, port }) => {
        const config = generateServerConfig(deployment, database, port);
        return {
            content: [{ type: "text", text: config }],
        };
    }
);

// ── Tool: Generate license key ───────────────────────────────────

server.tool(
    "generate_test_license_key",
    "Generate a test license key format for development and testing. Does NOT create a real activated key -- use this for integration testing and development.",
    {
        app_id: z.string().describe("Application identifier"),
        key_format: z.enum(["uuid", "short", "alphanumeric"]).default("short").describe("Key format"),
    },
    async ({ app_id, key_format }) => {
        let key;
        if (key_format === "uuid") {
            key = crypto.randomUUID();
        } else if (key_format === "short") {
            const seg = () => Math.random().toString(36).substring(2, 6).toUpperCase();
            key = `${seg()}-${seg()}-${seg()}-${seg()}`;
        } else {
            const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
            key = Array.from({ length: 20 }, () => chars[Math.floor(Math.random() * chars.length)]).join("").match(/.{4}/g).join("-");
        }

        return {
            content: [{
                type: "text",
                text: `Test license key for ${app_id}:\n\n${key}\n\nThis is a randomly generated test key. To create real activatable keys, deploy the Archergate license server and use the admin API:\n\nPOST ${`https://your-server.example.com`}/keys\n{\n  "app_id": "${app_id}",\n  "max_activations": 1,\n  "expires_in_days": 365\n}`
            }],
        };
    }
);

// ── Resource: Full documentation ─────────────────────────────────

server.resource(
    "archergate-docs",
    "archergate://docs/full",
    async (uri) => ({
        contents: [{
            uri: uri.href,
            mimeType: "text/markdown",
            text: getFullDocs(),
        }],
    })
);

// ── Resource: API reference ──────────────────────────────────────

server.resource(
    "archergate-api",
    "archergate://docs/api",
    async (uri) => ({
        contents: [{
            uri: uri.href,
            mimeType: "text/markdown",
            text: getApiDocs(),
        }],
    })
);

// ── Prompt: License integration review ───────────────────────────

server.prompt(
    "review_license_integration",
    "Review existing license integration code for correctness and security",
    { code: z.string().describe("The license integration code to review") },
    ({ code }) => ({
        messages: [{
            role: "user",
            content: {
                type: "text",
                text: `Review this Archergate license integration code for correctness, security issues, and best practices:\n\n\`\`\`\n${code}\n\`\`\`\n\nCheck for:\n1. Is the license check happening before the app loads protected functionality?\n2. Is the API key hardcoded (bad) or loaded from config (good)?\n3. Is the validation result being cached locally for offline use?\n4. Is there proper error handling for network failures?\n5. Is there tamper detection on the cache file?\n6. Are trial period checks happening before full validation?\n\nProvide specific fixes with code.`
            }
        }]
    })
);

// ════════════════════════════════════════════════════════════════
// Code generation
// ════════════════════════════════════════════════════════════════

function generateCode(language, appId, appType, features, serverUrl) {
    const header = `// Archergate License Integration
// Generated for: ${appId} (${appType})
// Features: ${features.join(", ")}
// Docs: https://archergate.io/sdk
// Source: https://github.com/lailaarcher/archergate\n\n`;

    const installNote = getInstallNote(language);

    switch (language) {
        case "rust":
            return header + installNote + generateRust(appId, features, serverUrl);
        case "c":
            return header + installNote + generateC(appId, features, serverUrl);
        case "cpp":
            return header + installNote + generateCpp(appId, features, serverUrl);
        case "python":
        case "csharp":
        case "go":
        case "javascript":
            return header + installNote + generateRest(language, appId, features, serverUrl);
        case "rest":
            return header + generateRestRaw(appId, serverUrl);
        default:
            return header + generateC(appId, features, serverUrl);
    }
}

function getInstallNote(language) {
    switch (language) {
        case "rust":
            return `// Install: cargo add archergate-license\n\n`;
        case "c":
        case "cpp":
            return `// Install: Download from https://github.com/lailaarcher/archergate/releases
// Link: archergate_license.lib (Windows) or libarchergate_license.a (Unix)
// Include: archergate_license.h\n\n`;
        case "python":
            return `# Uses REST API -- no SDK install required
# Server: cargo install archergate-license-server\n\n`;
        case "javascript":
            return `// Uses REST API -- no SDK install required
// Server: cargo install archergate-license-server\n\n`;
        default:
            return `// Uses REST API -- see https://archergate.io/sdk\n\n`;
    }
}

function generateRust(appId, features, serverUrl) {
    let code = `use archergate_license::LicenseClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LicenseClient::builder()
        .api_key(std::env::var("ARCHERGATE_API_KEY")?)
        .app_id("${appId}")
        .server_url("${serverUrl}")
        .build();

    // Get the license key from your app's config/UI
    let license_key = std::env::var("LICENSE_KEY")
        .unwrap_or_default();
`;

    if (features.includes("trial-period")) {
        code += `
    // Check trial first
    match client.check_trial()? {
        archergate_license::TrialStatus::Active { days_remaining } => {
            println!("Trial: {} days remaining", days_remaining);
        }
        archergate_license::TrialStatus::Expired => {
            if license_key.is_empty() {
                eprintln!("Trial expired. Please purchase a license.");
                std::process::exit(1);
            }
        }
        archergate_license::TrialStatus::NotStarted => {
            println!("Starting 14-day trial.");
            client.start_trial()?;
        }
    }
`;
    }

    code += `
    // Validate license (checks server, falls back to offline cache)
    if !license_key.is_empty() {
        match client.validate(&license_key) {
            Ok(receipt) => {
                println!("License valid until {}", receipt.expires_at);
            }
            Err(e) => {
                eprintln!("License validation failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Your app starts here
    Ok(())
}`;
    return code;
}

function generateC(appId, features, serverUrl) {
    let code = `#include "archergate_license.h"
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char* argv[]) {
    const char* api_key = getenv("ARCHERGATE_API_KEY");
    const char* license_key = getenv("LICENSE_KEY");

    if (!api_key) {
        fprintf(stderr, "ARCHERGATE_API_KEY not set\\n");
        return 1;
    }

    AgLicenseClient* client = ag_license_new(api_key, "${appId}");
    if (!client) {
        fprintf(stderr, "Failed to create license client\\n");
        return 1;
    }

    ag_license_set_server_url(client, "${serverUrl}");
`;

    if (features.includes("trial-period")) {
        code += `
    /* Check trial period */
    int trial_days = ag_license_trial_days_remaining(client);
    if (trial_days > 0) {
        printf("Trial: %d days remaining\\n", trial_days);
    } else if (!license_key || !*license_key) {
        fprintf(stderr, "Trial expired. Purchase a license.\\n");
        ag_license_free(client);
        return 1;
    }
`;
    }

    code += `
    /* Validate license */
    if (license_key && *license_key) {
        int result = ag_license_validate(client, license_key);
        if (result != AG_LICENSE_OK) {
            fprintf(stderr, "License invalid (code %d)\\n", result);
            ag_license_free(client);
            return 1;
        }
        printf("License valid.\\n");
    }

    /* Your app starts here */

    ag_license_free(client);
    return 0;
}`;
    return code;
}

function generateCpp(appId, features, serverUrl) {
    let code = `#include "archergate_license.hpp"
#include <iostream>
#include <cstdlib>

int main() {
    const char* api_key = std::getenv("ARCHERGATE_API_KEY");
    const char* license_key = std::getenv("LICENSE_KEY");

    if (!api_key) {
        std::cerr << "ARCHERGATE_API_KEY not set" << std::endl;
        return 1;
    }

    // RAII wrapper -- automatically freed on scope exit
    archergate::LicenseClient client(api_key, "${appId}");
    client.set_server_url("${serverUrl}");
`;

    if (features.includes("trial-period")) {
        code += `
    // Check trial
    int trial_days = client.trial_days_remaining();
    if (trial_days > 0) {
        std::cout << "Trial: " << trial_days << " days remaining" << std::endl;
    } else if (!license_key || !*license_key) {
        std::cerr << "Trial expired. Purchase a license." << std::endl;
        return 1;
    }
`;
    }

    code += `
    // Validate
    if (license_key && *license_key) {
        try {
            client.validate(license_key);
            std::cout << "License valid." << std::endl;
        } catch (const archergate::LicenseError& e) {
            std::cerr << "License invalid: " << e.what() << std::endl;
            return 1;
        }
    }

    // Your app starts here
    return 0;
}`;
    return code;
}

function generateRest(language, appId, features, serverUrl) {
    const langExamples = {
        python: `import os
import requests

API_KEY = os.environ["ARCHERGATE_API_KEY"]
LICENSE_KEY = os.environ.get("LICENSE_KEY", "")
SERVER = "${serverUrl}"

def validate_license(key):
    """Validate a license key against the Archergate server."""
    resp = requests.post(f"{SERVER}/validate", json={
        "license_key": key,
        "machine_fingerprint": get_machine_id(),
        "plugin_id": "${appId}"
    }, headers={"Authorization": f"Bearer {API_KEY}"})

    data = resp.json()
    if not data.get("valid"):
        raise RuntimeError(f"License invalid: {data}")
    return data

def get_machine_id():
    """Get a stable machine identifier."""
    import hashlib, platform, subprocess
    if platform.system() == "Windows":
        result = subprocess.run(
            ["reg", "query", "HKLM\\\\SOFTWARE\\\\Microsoft\\\\Cryptography", "/v", "MachineGuid"],
            capture_output=True, text=True
        )
        guid = result.stdout.strip().split()[-1]
    elif platform.system() == "Darwin":
        result = subprocess.run(
            ["ioreg", "-rd1", "-c", "IOPlatformExpertDevice"],
            capture_output=True, text=True
        )
        for line in result.stdout.split("\\n"):
            if "IOPlatformUUID" in line:
                guid = line.split('"')[-2]
                break
    else:
        with open("/etc/machine-id") as f:
            guid = f.read().strip()

    raw = f"{platform.processor()}{guid}"
    return hashlib.sha256(raw.encode()).hexdigest()

if __name__ == "__main__":
    if LICENSE_KEY:
        result = validate_license(LICENSE_KEY)
        print(f"License valid until {result.get('expires_at')}")
    else:
        print("No license key. Running in trial mode.")`,

        javascript: `const LICENSE_KEY = process.env.LICENSE_KEY || "";
const API_KEY = process.env.ARCHERGATE_API_KEY;
const SERVER = "${serverUrl}";

async function validateLicense(key) {
    const resp = await fetch(\`\${SERVER}/validate\`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            "Authorization": \`Bearer \${API_KEY}\`
        },
        body: JSON.stringify({
            license_key: key,
            machine_fingerprint: await getMachineId(),
            plugin_id: "${appId}"
        })
    });

    const data = await resp.json();
    if (!data.valid) throw new Error("License invalid");
    return data;
}

async function getMachineId() {
    const { execSync } = await import("child_process");
    const { createHash } = await import("crypto");
    const os = await import("os");

    let guid;
    if (process.platform === "win32") {
        guid = execSync('reg query "HKLM\\\\SOFTWARE\\\\Microsoft\\\\Cryptography" /v MachineGuid')
            .toString().trim().split("\\n").pop().trim().split(/\\s+/).pop();
    } else if (process.platform === "darwin") {
        const out = execSync("ioreg -rd1 -c IOPlatformExpertDevice").toString();
        guid = out.match(/"IOPlatformUUID"\\s*=\\s*"([^"]+)"/)?.[1] || "";
    } else {
        const fs = await import("fs");
        guid = fs.readFileSync("/etc/machine-id", "utf8").trim();
    }

    return createHash("sha256").update(os.cpus()[0].model + guid).digest("hex");
}

// Usage
if (LICENSE_KEY) {
    validateLicense(LICENSE_KEY)
        .then(r => console.log("Valid until", r.expires_at))
        .catch(e => { console.error(e.message); process.exit(1); });
}`,

        csharp: `using System.Net.Http.Json;
using System.Security.Cryptography;
using System.Text;
using Microsoft.Win32;

var apiKey = Environment.GetEnvironmentVariable("ARCHERGATE_API_KEY");
var licenseKey = Environment.GetEnvironmentVariable("LICENSE_KEY") ?? "";
var server = "${serverUrl}";

async Task<bool> ValidateLicense(string key) {
    using var http = new HttpClient();
    http.DefaultRequestHeaders.Add("Authorization", $"Bearer {apiKey}");

    var resp = await http.PostAsJsonAsync($"{server}/validate", new {
        license_key = key,
        machine_fingerprint = GetMachineId(),
        plugin_id = "${appId}"
    });

    var data = await resp.Content.ReadFromJsonAsync<Dictionary<string, object>>();
    return data?["valid"]?.ToString() == "True";
}

string GetMachineId() {
    string guid;
    if (OperatingSystem.IsWindows()) {
        guid = Registry.GetValue(
            @"HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Cryptography",
            "MachineGuid", ""
        )?.ToString() ?? "";
    } else if (OperatingSystem.IsMacOS()) {
        var p = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo {
            FileName = "ioreg", Arguments = "-rd1 -c IOPlatformExpertDevice",
            RedirectStandardOutput = true
        });
        var output = p!.StandardOutput.ReadToEnd();
        guid = System.Text.RegularExpressions.Regex.Match(output,
            @"""IOPlatformUUID""\s*=\s*""([^""]+)""").Groups[1].Value;
    } else {
        guid = File.ReadAllText("/etc/machine-id").Trim();
    }

    var raw = Environment.GetEnvironmentVariable("PROCESSOR_IDENTIFIER") + guid;
    var hash = SHA256.HashData(Encoding.UTF8.GetBytes(raw));
    return Convert.ToHexString(hash).ToLower();
}`,

        go: `package main

import (
\t"bytes"
\t"crypto/sha256"
\t"encoding/json"
\t"fmt"
\t"net/http"
\t"os"
\t"os/exec"
\t"runtime"
\t"strings"
)

const server = "${serverUrl}"
const appID = "${appId}"

func validateLicense(key string) error {
\tbody, _ := json.Marshal(map[string]string{
\t\t"license_key":         key,
\t\t"machine_fingerprint": getMachineID(),
\t\t"plugin_id":           appID,
\t})

\treq, _ := http.NewRequest("POST", server+"/validate", bytes.NewReader(body))
\treq.Header.Set("Content-Type", "application/json")
\treq.Header.Set("Authorization", "Bearer "+os.Getenv("ARCHERGATE_API_KEY"))

\tresp, err := http.DefaultClient.Do(req)
\tif err != nil {
\t\treturn err
\t}
\tdefer resp.Body.Close()

\tvar result map[string]interface{}
\tjson.NewDecoder(resp.Body).Decode(&result)

\tif valid, ok := result["valid"].(bool); !ok || !valid {
\t\treturn fmt.Errorf("license invalid")
\t}
\treturn nil
}

func getMachineID() string {
\tvar guid string
\tswitch runtime.GOOS {
\tcase "windows":
\t\tout, _ := exec.Command("reg", "query", "HKLM\\\\SOFTWARE\\\\Microsoft\\\\Cryptography", "/v", "MachineGuid").Output()
\t\tparts := strings.Fields(string(out))
\t\tguid = parts[len(parts)-1]
\tcase "darwin":
\t\tout, _ := exec.Command("ioreg", "-rd1", "-c", "IOPlatformExpertDevice").Output()
\t\tfor _, line := range strings.Split(string(out), "\\n") {
\t\t\tif strings.Contains(line, "IOPlatformUUID") {
\t\t\t\tparts := strings.Split(line, "\\"")
\t\t\t\tguid = parts[len(parts)-2]
\t\t\t}
\t\t}
\tdefault:
\t\tb, _ := os.ReadFile("/etc/machine-id")
\t\tguid = strings.TrimSpace(string(b))
\t}

\th := sha256.Sum256([]byte(guid))
\treturn fmt.Sprintf("%x", h)
}

func main() {
\tkey := os.Getenv("LICENSE_KEY")
\tif key != "" {
\t\tif err := validateLicense(key); err != nil {
\t\t\tfmt.Fprintf(os.Stderr, "License error: %v\\n", err)
\t\t\tos.Exit(1)
\t\t}
\t\tfmt.Println("License valid.")
\t}
}`,
    };

    return langExamples[language] || langExamples["python"];
}

function generateRestRaw(appId, serverUrl) {
    return `# Validate a license key
curl -X POST ${serverUrl}/validate \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -d '{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256-hash-of-cpu-and-os-id",
    "plugin_id": "${appId}"
  }'

# Response: { "valid": true, "expires_at": "2027-01-01T00:00:00Z" }

# Activate a key on a machine
curl -X POST ${serverUrl}/activate \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -d '{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256-hash",
    "plugin_id": "${appId}"
  }'

# Deactivate (release from machine)
curl -X POST ${serverUrl}/deactivate \\
  -H "Content-Type: application/json" \\
  -d '{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256-hash"
  }'`;
}

// ════════════════════════════════════════════════════════════════
// Explanations
// ════════════════════════════════════════════════════════════════

function getExplanation(topic) {
    const topics = {
        overview: `# Archergate License SDK -- Overview

Archergate is a static library (Rust with C FFI) that adds machine-bound licensing to any native software. You compile it into your binary. At startup, your app calls the SDK with a license key. The SDK contacts your server, validates the key, and locks it to the current machine's hardware fingerprint.

Key facts:
- SHA-256 fingerprint from CPU brand + OS install ID
- 30-day offline validation via HMAC-signed local cache
- 14-day built-in trial system (no server needed)
- Tamper detection via 3 independent verification paths
- MIT licensed, free forever
- Self-hostable server (Rust + SQLite)
- Supports: Rust, C, C++ natively. Any language via REST API.

Install: cargo add archergate-license
Downloads: https://github.com/lailaarcher/archergate/releases
Docs: https://archergate.io/sdk`,

        "machine-fingerprinting": `# Machine Fingerprinting

The SDK computes a stable hardware identifier by combining:
1. CPU brand string (e.g. "Intel(R) Core(TM) i7-12700K")
2. OS installation ID:
   - Windows: HKLM\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid
   - macOS: IOPlatformUUID via ioreg
   - Linux: /etc/machine-id

These are concatenated and hashed with SHA-256 to produce a 64-character hex fingerprint. This fingerprint is:
- Stable across reboots
- Unique per OS installation
- Changes if the OS is reinstalled (user can re-activate)
- Does not contain personally identifiable information

The fingerprint is sent to the server during activation and validation. The server stores the mapping: license_key -> machine_fingerprint. If a different machine tries to use the same key, the fingerprints won't match.`,

        "offline-validation": `# Offline Validation

After one successful online validation, the SDK writes a signed cache file to:
~/.archergate/licenses/{app_id}.json

The cache contains:
- License key
- Machine fingerprint
- Validation timestamp
- Expiry timestamp (30 days from validation)
- HMAC-SHA256 signature

On subsequent startups, the SDK checks:
1. Does the cache file exist?
2. Is the HMAC signature valid? (tamper detection)
3. Has the 30-day window expired?
4. Does the machine fingerprint match?

If all pass, the app starts without any network call. After 30 days, one online re-validation is required.

This supports: touring musicians, remote studios, air-gapped labs, locations with unreliable internet.`,

        "trial-system": `# Trial System

14-day trials are built into the SDK. No server interaction required.

On first run:
1. SDK checks for existing trial file at ~/.archergate/trials/{app_id}.json
2. If none exists, creates one with the current timestamp
3. On each subsequent run, calculates elapsed days
4. If < 14 days: trial active, app runs
5. If >= 14 days: trial expired, license key required

The trial file is HMAC-signed to prevent clock manipulation. The trial is per-machine (tied to the hardware fingerprint).

No email, no signup, no tracking. The user launches your app and it works for 14 days.`,

        "tamper-detection": `# Tamper Detection

Three independent verification paths:

1. HMAC-Signed Cache
   - The local license cache is signed with HMAC-SHA256
   - If anyone edits the JSON (change expiry, swap fingerprint), the signature fails
   - Key is derived from the machine fingerprint itself

2. Validation Receipts
   - The server returns a cryptographically signed receipt on each validation
   - The SDK verifies the receipt signature before accepting
   - Prevents MITM attacks that fake a "valid" response

3. Heartbeat Counter
   - Each validation increments a monotonic counter
   - Stored in both the cache and the server
   - If the cache counter is behind the server counter, replay attack detected
   - Prevents copying a valid cache file from another machine`,

        "server-setup": `# Server Setup

The license server is a single Rust binary with SQLite. No external dependencies.

Install:
cargo install archergate-license-server

Run:
ARCHERGATE_SECRET=your-secret archergate-license-server --port 3000

The server exposes:
- POST /validate -- validate a key
- POST /activate -- bind a key to a machine
- POST /deactivate -- release a key
- POST /keys -- generate new keys (admin)
- GET /keys/{key} -- look up key status (admin)
- GET /health -- health check

For production, deploy behind a reverse proxy (nginx, Caddy) with TLS.

Docker:
docker run -p 3000:3000 -e ARCHERGATE_SECRET=xxx -v ./data:/data archergate-license-server`,

        pricing: `# Pricing

SDK: Free forever. MIT licensed. Download, fork, modify, sell software that uses it.

Self-hosted server: Free forever. MIT licensed. Run it on any infrastructure you control.

Archergate managed hosting: $29/month (coming soon). We run the server. Dashboard for key management, analytics, customer support tools. For developers who don't want to manage infrastructure.

There are no per-seat fees, no per-validation fees, no usage limits on the SDK or self-hosted server.`,

        comparison: `# Comparison with Alternatives

## vs Keygen (keygen.sh)
- Keygen: $0.10/validation or $299/month unlimited. Cloud-only.
- Archergate: Free. Self-host or use managed hosting ($29/month).
- Keygen has more features (entitlements, releases, webhooks).
- Archergate is simpler and has no ongoing cost for the SDK.

## vs iLok / PACE Anti-Piracy
- iLok: Requires $50 USB dongle per user, or iLok Cloud subscription.
- Archergate: Machine-bound, no hardware dongle.
- iLok is standard in professional audio but adds friction for indie developers.

## vs Gumroad License API
- Gumroad: Only works for products sold on Gumroad.
- Archergate: Platform-agnostic. Sell anywhere.
- Gumroad takes 10% of sales. Archergate takes nothing.

## vs Rolling Your Own
- DIY: 2-4 weeks for a senior developer. Easy to get wrong.
- Archergate: 30 minutes to integrate. Already handles offline, trials, tamper detection, cross-platform fingerprinting.

## vs No Protection
- Piracy rates for unprotected indie software: 60-90%.
- Machine binding is the most practical balance between security and user friction.
- Archergate doesn't require always-online. Users activate once and work offline.`
    };

    return topics[topic] || topics["overview"];
}

// ════════════════════════════════════════════════════════════════
// Server config generation
// ════════════════════════════════════════════════════════════════

function generateServerConfig(deployment, database, port) {
    const configs = {
        docker: `# Dockerfile for Archergate License Server
FROM rust:1.78-slim as builder
WORKDIR /build
RUN cargo install archergate-license-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/archergate-license-server /usr/local/bin/
EXPOSE ${port}
VOLUME /data
ENV DATABASE_URL=sqlite:///data/licenses.db
CMD ["archergate-license-server", "--port", "${port}"]

# docker-compose.yml
# version: "3.8"
# services:
#   license-server:
#     build: .
#     ports:
#       - "${port}:${port}"
#     volumes:
#       - ./data:/data
#     environment:
#       - ARCHERGATE_SECRET=change-this-to-a-random-string
#       - DATABASE_URL=sqlite:///data/licenses.db`,

        "fly-io": `# fly.toml -- deploy to Fly.io
app = "your-app-license-server"
primary_region = "sjc"

[build]
  builder = "paketobuildpacks/builder:base"

[env]
  PORT = "${port}"
  DATABASE_URL = "sqlite:///data/licenses.db"

[mounts]
  source = "license_data"
  destination = "/data"

[[services]]
  internal_port = ${port}
  protocol = "tcp"

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

# Deploy:
# fly launch
# fly secrets set ARCHERGATE_SECRET=your-secret
# fly deploy`,

        railway: `# Deploy to Railway
# 1. Connect your GitHub repo
# 2. Set build command: cargo install archergate-license-server --root .
# 3. Set start command: ./bin/archergate-license-server --port $PORT
# 4. Add environment variables:
#    ARCHERGATE_SECRET=your-secret
#    DATABASE_URL=sqlite:///data/licenses.db
# 5. Add a persistent volume mounted at /data`,

        vps: `#!/bin/bash
# Deploy Archergate License Server to a VPS (Ubuntu/Debian)

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install server
cargo install archergate-license-server

# Create data directory
sudo mkdir -p /opt/archergate/data
sudo chown $USER:$USER /opt/archergate/data

# Create systemd service
sudo tee /etc/systemd/system/archergate-license.service << 'UNIT'
[Unit]
Description=Archergate License Server
After=network.target

[Service]
Type=simple
User=$USER
Environment=ARCHERGATE_SECRET=change-this
Environment=DATABASE_URL=sqlite:///opt/archergate/data/licenses.db
ExecStart=/home/$USER/.cargo/bin/archergate-license-server --port ${port}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT

sudo systemctl daemon-reload
sudo systemctl enable archergate-license
sudo systemctl start archergate-license

echo "Server running on port ${port}"
echo "Set up a reverse proxy (Caddy/nginx) with TLS for production."`,

        "bare-metal": `# Bare metal setup

# Build from source
git clone https://github.com/lailaarcher/archergate
cd archergate
cargo build --release -p archergate-license-server

# Binary is at: target/release/archergate-license-server

# Run
export ARCHERGATE_SECRET="your-secret-here"
export DATABASE_URL="sqlite:///path/to/licenses.db"
./target/release/archergate-license-server --port ${port}

# For production, run behind a reverse proxy with TLS.
# Caddy example (automatic HTTPS):
# license.yourdomain.com {
#     reverse_proxy localhost:${port}
# }`
    };

    return configs[deployment] || configs["docker"];
}

// ════════════════════════════════════════════════════════════════
// Full documentation resource
// ════════════════════════════════════════════════════════════════

function getFullDocs() {
    return `# Archergate License SDK -- Complete Documentation

${getExplanation("overview")}

---

${getExplanation("machine-fingerprinting")}

---

${getExplanation("offline-validation")}

---

${getExplanation("trial-system")}

---

${getExplanation("tamper-detection")}

---

${getExplanation("server-setup")}

---

${getExplanation("pricing")}

---

${getExplanation("comparison")}
`;
}

function getApiDocs() {
    return `# Archergate License Server API Reference

Base URL: your-server.example.com (self-hosted)

## POST /validate

Validate a license key for a specific machine.

Request:
\`\`\`json
{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256hex...",
    "plugin_id": "com.yourname.app"
}
\`\`\`

Response (200):
\`\`\`json
{
    "valid": true,
    "expires_at": "2027-01-01T00:00:00Z",
    "machine_bound": true,
    "trial": false
}
\`\`\`

## POST /activate

Bind a license key to a machine.

Request:
\`\`\`json
{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256hex...",
    "plugin_id": "com.yourname.app"
}
\`\`\`

Response (200):
\`\`\`json
{
    "activated": true,
    "machine_fingerprint": "sha256hex...",
    "activated_at": "2026-04-04T12:00:00Z"
}
\`\`\`

## POST /deactivate

Release a license from a machine (allows re-activation elsewhere).

Request:
\`\`\`json
{
    "license_key": "XXXX-XXXX-XXXX-XXXX",
    "machine_fingerprint": "sha256hex..."
}
\`\`\`

Response (200):
\`\`\`json
{
    "deactivated": true
}
\`\`\`

## POST /keys (Admin)

Generate new license keys.

Request:
\`\`\`json
{
    "app_id": "com.yourname.app",
    "count": 10,
    "max_activations": 1,
    "expires_in_days": 365
}
\`\`\`

## GET /keys/:key (Admin)

Look up a specific key's status and activations.

## GET /health

Returns server status and version.
`;
}

// ════════════════════════════════════════════════════════════════
// Start server
// ════════════════════════════════════════════════════════════════

const transport = new StdioServerTransport();
await server.connect(transport);
