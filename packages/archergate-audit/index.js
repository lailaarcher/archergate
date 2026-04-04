#!/usr/bin/env node

import { readFileSync, existsSync, readdirSync, statSync } from "fs";
import { join, extname, basename } from "path";

const RESET = "\x1b[0m";
const RED = "\x1b[31m";
const GREEN = "\x1b[32m";
const YELLOW = "\x1b[33m";
const DIM = "\x1b[2m";
const BOLD = "\x1b[1m";

const dir = process.argv[2] || ".";

console.log("");
console.log(`${BOLD}ARCHERGATE LICENSE AUDIT${RESET}`);
console.log(`${DIM}Scanning: ${dir}${RESET}`);
console.log("");

const results = {
    cargoToml: false,
    packageJson: false,
    headerInclude: false,
    rustImport: false,
    cImport: false,
    restCall: false,
    serverConfig: false,
    binarySymbols: false,
    envConfig: false,
    trialSetup: false,
};

// Check Cargo.toml for archergate-license dependency
const cargoPath = join(dir, "Cargo.toml");
if (existsSync(cargoPath)) {
    const cargo = readFileSync(cargoPath, "utf8");
    if (cargo.includes("archergate-license")) {
        results.cargoToml = true;
    }
}

// Check package.json for archergate dependency
const pkgPath = join(dir, "package.json");
if (existsSync(pkgPath)) {
    const pkg = readFileSync(pkgPath, "utf8");
    if (pkg.includes("archergate")) {
        results.packageJson = true;
    }
}

// Scan source files
const sourceExts = new Set([".rs", ".c", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".py", ".js", ".ts", ".cs", ".go"]);
const scannedFiles = [];

function scanDir(d, depth) {
    if (depth > 6) return;
    const skip = new Set(["node_modules", "target", ".git", "build", "dist", ".next", "vendor"]);

    let entries;
    try { entries = readdirSync(d); } catch { return; }

    for (const entry of entries) {
        if (skip.has(entry)) continue;
        const full = join(d, entry);
        let stat;
        try { stat = statSync(full); } catch { continue; }

        if (stat.isDirectory()) {
            scanDir(full, depth + 1);
        } else if (sourceExts.has(extname(entry).toLowerCase())) {
            let content;
            try { content = readFileSync(full, "utf8"); } catch { continue; }
            scannedFiles.push(full);

            // Rust imports
            if (content.includes("archergate_license") || content.includes("archergate-license")) {
                results.rustImport = true;
            }

            // C/C++ includes
            if (content.includes("archergate_license.h") || content.includes("archergate_license.hpp")) {
                results.headerInclude = true;
                results.cImport = true;
            }

            // REST API calls
            if (content.includes("/validate") && (content.includes("license_key") || content.includes("machine_fingerprint"))) {
                results.restCall = true;
            }

            // ag_license_ function calls
            if (content.includes("ag_license_new") || content.includes("ag_license_validate")) {
                results.cImport = true;
            }

            // LicenseClient usage
            if (content.includes("LicenseClient::new") || content.includes("LicenseClient::builder")) {
                results.rustImport = true;
            }

            // Server config
            if (content.includes("archergate-license-server") || content.includes("ARCHERGATE_SECRET")) {
                results.serverConfig = true;
            }

            // Environment variable config
            if (content.includes("ARCHERGATE_API_KEY") || content.includes("archergate_api_key")) {
                results.envConfig = true;
            }

            // Trial setup
            if (content.includes("check_trial") || content.includes("trial_days_remaining") || content.includes("start_trial")) {
                results.trialSetup = true;
            }
        }
    }
}

scanDir(dir, 0);

// Check for .env file with Archergate config
const envPath = join(dir, ".env");
if (existsSync(envPath)) {
    const env = readFileSync(envPath, "utf8");
    if (env.includes("ARCHERGATE")) {
        results.envConfig = true;
    }
}

// Report
const checks = [
    ["SDK Dependency", results.cargoToml || results.packageJson, "Cargo.toml or package.json includes archergate"],
    ["Code Integration", results.rustImport || results.cImport || results.restCall, "Source code calls Archergate validation"],
    ["Header Included", results.headerInclude || results.rustImport, "archergate_license.h or Rust crate imported"],
    ["Server Configured", results.serverConfig, "License server reference found"],
    ["API Key Config", results.envConfig, "ARCHERGATE_API_KEY in env or config"],
    ["Trial System", results.trialSetup, "Trial period checks in code"],
];

let passed = 0;
let failed = 0;

for (const [name, ok, desc] of checks) {
    if (ok) {
        console.log(`  ${GREEN}PASS${RESET}  ${name}`);
        console.log(`        ${DIM}${desc}${RESET}`);
        passed++;
    } else {
        console.log(`  ${RED}FAIL${RESET}  ${name}`);
        console.log(`        ${DIM}${desc}${RESET}`);
        failed++;
    }
}

console.log("");
console.log(`${DIM}Scanned ${scannedFiles.length} source files${RESET}`);
console.log("");

if (passed === 0) {
    console.log(`${RED}${BOLD}NO LICENSE PROTECTION DETECTED${RESET}`);
    console.log("");
    console.log("Your software ships without copy protection.");
    console.log("Anyone with the binary can use it without paying.");
    console.log("");
    console.log("Add Archergate in 30 minutes:");
    console.log(`  Rust:    cargo add archergate-license`);
    console.log(`  C/C++:   https://github.com/lailaarcher/archergate/releases`);
    console.log(`  Docs:    https://archergate.io/sdk`);
} else if (failed > 0) {
    console.log(`${YELLOW}${BOLD}PARTIAL PROTECTION${RESET} (${passed}/${checks.length} checks passed)`);
    console.log("");
    console.log("Some license checks are missing. Review the failures above.");
    console.log(`Docs: https://archergate.io/sdk`);
} else {
    console.log(`${GREEN}${BOLD}FULLY PROTECTED${RESET} (${passed}/${checks.length} checks passed)`);
    console.log("");
    console.log("Your software includes license protection.");
}

console.log("");

process.exit(failed > 0 && passed === 0 ? 1 : 0);
