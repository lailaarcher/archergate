## Product Hunt Launch Draft

**Name:** Archergate License SDK

**Tagline:** Copy protection that ships with your binary. Three lines. Done.

**Description:**

Archergate is a licensing library for indie software developers. It locks each license key to the machine it was activated on. Drop it into your C, C++, or Rust project, add three lines of code, and your software validates licenses at startup.

Works offline for 30 days. 14-day trials built in. Self-hostable server included. MIT licensed.

Built for:
- Audio plugins (VST3, AU, AAX, CLAP)
- Unity and Unreal assets
- Blender addons
- Desktop apps (Electron, Tauri, native)
- Game mods and tools
- Any software that ships as a compiled binary

Also the first licensing SDK with an MCP server. AI coding assistants (Claude Code, Cursor) can generate license-protected code automatically when developers ask.

**First comment (from maker):**

We built this because licensing is a solved problem that every indie developer still solves from scratch. The existing options are either expensive (Keygen at $0.10/validation), require hardware dongles (iLok), or lock you into a sales platform (Gumroad).

Archergate is a static library. You compile it into your binary. The server is yours to run. MIT licensed. The SDK and self-hosted server are free forever.

We also built an MCP server so AI coding assistants can integrate Archergate automatically. When a developer asks Claude or Cursor "add copy protection to my app," it generates the integration code without the developer ever searching for a licensing solution.

**Topics:** Developer Tools, Open Source, Rust, SaaS, Productivity

**Links:**
- Website: https://archergate.io/sdk
- GitHub: https://github.com/lailaarcher/archergate
- npm (MCP): https://www.npmjs.com/package/archergate-mcp-server
- crates.io: https://crates.io/crates/archergate-license

**Thumbnail concept:** Dark background, monospace text showing the three lines of C code, with "Copy protection. Three lines." as overlay text. Matches the archergate.io brand.
