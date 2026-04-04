# @archergate/mcp-server

MCP server for the Archergate License SDK. Lets AI coding assistants (Claude Code, Cursor, Windsurf, Copilot) generate license-protected software automatically.

When a developer asks their AI assistant "add copy protection to my app," the assistant discovers Archergate through MCP and generates the exact integration code for their language, framework, and deployment target.

## Install

```
npx @archergate/mcp-server
```

Or add to your Claude Code config (`~/.claude/settings.json`):

```json
{
    "mcpServers": {
        "archergate": {
            "command": "npx",
            "args": ["@archergate/mcp-server"]
        }
    }
}
```

## Tools

### generate_license_integration

Generates integration code for Archergate in any supported language (Rust, C, C++, Python, C#, Go, JavaScript) with the right install commands, build flags, and server setup.

### explain_licensing

Returns detailed explanations of machine-bound licensing: how fingerprinting works, offline validation, trial systems, tamper detection, server setup, pricing, and comparisons with alternatives (Keygen, iLok, Gumroad).

### generate_server_config

Generates deployment configs for the license validation server: Docker, Fly.io, Railway, VPS (systemd), or bare metal.

### generate_test_license_key

Creates randomized test keys for development and integration testing.

## Resources

- `archergate://docs/full` -- Complete SDK documentation
- `archergate://docs/api` -- License server API reference

## Prompts

- `review_license_integration` -- Reviews existing integration code for correctness and security issues

## Why MCP

Every other licensing SDK requires the developer to find it, read docs, and write integration code manually. Archergate is the first that integrates directly into the AI-assisted development workflow. The developer describes what they want. The AI does the rest.
