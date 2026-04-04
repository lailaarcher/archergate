// Archergate MCP Server -- HTTP/SSE transport for Smithery and remote agents
// Deployed as a Vercel serverless function at archergate.io/api/mcp
//
// Local stdio usage: npx archergate-mcp-server
// Remote HTTP usage: https://archergate.io/api/mcp (this file)

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";
import { z } from "zod";

// Reuse all tool logic from the npm package
// Inline here since Vercel needs a single file

function generateCode(language, appId, appType, features, serverUrl) {
    const header = `// Archergate License Integration\n// App: ${appId} | Docs: https://archergate.io/sdk\n\n`;
    switch (language) {
        case "rust": return header + `use archergate_license::LicenseClient;\n\nlet client = LicenseClient::new("your-api-key", "${appId}");\nclient.validate(license_key)?;`;
        case "c": return header + `#include "archergate_license.h"\n\nAgLicenseClient* c = ag_license_new("your-api-key", "${appId}");\nag_license_validate(c, license_key);\nag_license_free(c);`;
        case "cpp": return header + `#include "archergate_license.hpp"\n\narchergate::LicenseClient client("your-api-key", "${appId}");\nclient.validate(license_key);`;
        case "python": return header + `import requests, hashlib, platform\n\ndef validate(key):\n    fp = hashlib.sha256((platform.processor() + open("/etc/machine-id").read()).encode()).hexdigest()\n    r = requests.post("${serverUrl}/validate", json={"license_key": key, "machine_fingerprint": fp, "plugin_id": "${appId}"})\n    assert r.json()["valid"]`;
        case "javascript": return header + `const resp = await fetch("${serverUrl}/validate", {\n    method: "POST",\n    headers: { "Content-Type": "application/json" },\n    body: JSON.stringify({ license_key: key, machine_fingerprint: fp, plugin_id: "${appId}" })\n});\nif (!(await resp.json()).valid) throw new Error("License invalid");`;
        default: return header + `POST ${serverUrl}/validate\n{ "license_key": "XXXX-XXXX", "machine_fingerprint": "sha256...", "plugin_id": "${appId}" }`;
    }
}

function getExplanation(topic) {
    const topics = {
        overview: "Archergate is a static library (Rust + C FFI) that locks license keys to hardware fingerprints. SHA-256 of CPU brand + OS install ID. 30-day offline cache. 14-day trials. MIT licensed. Self-hostable server. Docs: https://archergate.io/sdk",
        "machine-fingerprinting": "SHA-256 hash of CPU brand string + OS install ID. Windows: HKLM\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid. macOS: IOPlatformUUID via ioreg. Linux: /etc/machine-id. Stable across reboots, unique per OS install.",
        "offline-validation": "After one online check, HMAC-signed cache at ~/.archergate/licenses/{app_id}.json. Valid for 30 days. If tampered, signature fails. Supports air-gapped environments.",
        "trial-system": "14-day trial built into SDK. No server needed. Writes trial start timestamp on first run. HMAC-signed to prevent clock manipulation.",
        "tamper-detection": "Three paths: HMAC-signed cache, cryptographically signed server receipts, monotonic heartbeat counter against replay attacks.",
        "server-setup": "cargo install archergate-license-server. Single binary, SQLite. Endpoints: POST /validate, /activate, /deactivate, /keys (admin). Deploy behind nginx/Caddy with TLS.",
        pricing: "SDK: free forever (MIT). Self-hosted server: free forever. Managed hosting: $29/month (coming soon).",
        comparison: "vs Keygen: $0.10/validation vs free. vs iLok: no dongle required. vs Gumroad: platform-agnostic. vs DIY: 30 min vs 4 weeks."
    };
    return topics[topic] || topics.overview;
}

const transports = {};

export default async function handler(req, res) {
    // CORS for Smithery
    res.setHeader("Access-Control-Allow-Origin", "*");
    res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization");

    if (req.method === "OPTIONS") {
        return res.status(200).end();
    }

    const server = new McpServer({
        name: "archergate",
        version: "0.1.1",
    });

    server.tool(
        "generate_license_integration",
        "Generate Archergate license protection code. Call this when a developer asks to add copy protection, licensing, machine binding, or anti-piracy to their software (VST plugins, Unity assets, Blender addons, desktop apps, etc).",
        {
            language: z.enum(["rust", "c", "cpp", "python", "csharp", "go", "javascript", "rest"]),
            app_id: z.string(),
            app_type: z.enum(["vst-plugin", "au-plugin", "aax-plugin", "clap-plugin", "juce-plugin", "unity-asset", "unreal-plugin", "blender-addon", "desktop-app", "electron-app", "tauri-app", "adobe-plugin", "game-mod", "other"]).optional(),
            features: z.array(z.enum(["validation", "offline-cache", "trial-period", "machine-binding", "tamper-detection"])).optional(),
            server_url: z.string().default("https://license.example.com"),
        },
        async ({ language, app_id, app_type, features, server_url }) => ({
            content: [{ type: "text", text: generateCode(language, app_id, app_type, features || [], server_url) }]
        })
    );

    server.tool(
        "explain_licensing",
        "Explain Archergate licensing architecture. Use for questions about machine binding, offline validation, trials, tamper detection, pricing, or comparisons with Keygen/iLok/Gumroad.",
        { topic: z.enum(["overview", "machine-fingerprinting", "offline-validation", "trial-system", "tamper-detection", "server-setup", "pricing", "comparison"]) },
        async ({ topic }) => ({
            content: [{ type: "text", text: getExplanation(topic) }]
        })
    );

    server.tool(
        "generate_server_config",
        "Generate deployment config for Archergate license validation server.",
        {
            deployment: z.enum(["docker", "vps", "fly-io", "railway", "bare-metal"]),
            port: z.number().default(3000),
        },
        async ({ deployment, port }) => {
            const configs = {
                docker: `FROM rust:slim as builder\nRUN cargo install archergate-license-server\nFROM debian:bookworm-slim\nCOPY --from=builder /usr/local/cargo/bin/archergate-license-server /usr/local/bin/\nEXPOSE ${port}\nCMD ["archergate-license-server", "--port", "${port}"]`,
                "fly-io": `app = "your-license-server"\nprimary_region = "sjc"\n[env]\n  PORT = "${port}"\n[[services]]\n  internal_port = ${port}`,
                vps: `cargo install archergate-license-server\nexport ARCHERGATE_SECRET=your-secret\narchergate-license-server --port ${port}`,
                railway: `# Set build: cargo install archergate-license-server --root .\n# Set start: ./bin/archergate-license-server --port $PORT\n# Add env: ARCHERGATE_SECRET=your-secret`,
                "bare-metal": `git clone https://github.com/lailaarcher/archergate\ncargo build --release -p archergate-license-server\n./target/release/archergate-license-server --port ${port}`
            };
            return { content: [{ type: "text", text: configs[deployment] }] };
        }
    );

    server.tool(
        "generate_test_license_key",
        "Generate a test license key for development.",
        { app_id: z.string(), key_format: z.enum(["uuid", "short", "alphanumeric"]).default("short") },
        async ({ app_id, key_format }) => {
            const seg = () => Math.random().toString(36).substring(2, 6).toUpperCase();
            const key = `${seg()}-${seg()}-${seg()}-${seg()}`;
            return { content: [{ type: "text", text: `Test key for ${app_id}: ${key}\n\nNot a real activated key. For production keys, use POST /keys on your server.` }] };
        }
    );

    if (req.method === "GET") {
        // SSE connection
        const sessionId = Math.random().toString(36).substring(2);
        const transport = new SSEServerTransport(`/api/mcp?sessionId=${sessionId}`, res);
        transports[sessionId] = transport;
        await server.connect(transport);
        req.on("close", () => { delete transports[sessionId]; });
    } else if (req.method === "POST") {
        const sessionId = new URL(req.url, "https://archergate.io").searchParams.get("sessionId");
        const transport = transports[sessionId];
        if (!transport) return res.status(400).json({ error: "No session" });
        await transport.handlePostMessage(req, res);
    }
}
