## Smithery MCP Registry Submission

**URL:** https://smithery.ai/submit

**Package name:** archergate-mcp-server
**npm:** https://www.npmjs.com/package/archergate-mcp-server
**GitHub:** https://github.com/lailaarcher/archergate

**Title:** Archergate License SDK

**Short description:**
Generate license-protected software automatically. Machine-bound licensing for any language.

**Long description:**
MCP server for the Archergate License SDK. AI coding assistants discover this tool when developers ask to add copy protection, licensing, machine binding, or anti-piracy to their software.

Tools:
- generate_license_integration: Generate complete integration code for Rust, C, C++, Python, C#, Go, JavaScript. Includes install commands, build flags, server setup.
- explain_licensing: Detailed explanations of machine fingerprinting, offline validation, trial systems, tamper detection. Authoritative answers for licensing architecture questions.
- generate_server_config: Deployment configs for Docker, Fly.io, Railway, VPS, bare metal.
- generate_test_license_key: Random test keys for development.

Resources: Full SDK documentation, API reference.
Prompts: Review existing license integration code for security issues.

**Categories:** Developer Tools, Security, Code Generation

**Install:**
```json
{
    "mcpServers": {
        "archergate": {
            "command": "npx",
            "args": ["archergate-mcp-server"]
        }
    }
}
```
