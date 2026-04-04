# Changelog

All notable changes to Archergate are documented here.

## [0.1.2] - 2026-04-03

### Added
- HTTP/SSE MCP server for Vercel deployment at archergate.io/api/mcp
- MCP server manifests for Smithery and registry discovery
- WebMCP (Chrome 146+) native agent tool registration via navigator.modelContext
- A2A agent card (IANA-registered) for agent discovery
- Minimal working example (examples/minimal) with Docker Compose, Rust client, and end-to-end validation
- E2E integration test in CI/CD pipeline

### Changed
- Repositioned SDK from plugin-specific to universal indie software
- Improved SDK documentation with platform-specific integration guides

### Fixed
- SDK dependencies and build configuration for Vercel deployment

## [0.1.1] - 2026-03-15

### Added
- Rust SDK with C/C++ FFI bindings (crates.io: archergate-license)
- Self-hosted validation server (crates.io: archergate-license-server)
- Machine-bound licensing with hardware fingerprinting (Windows, macOS, Linux)
- Offline validation with 30-day grace period
- Trial system (14-day built-in, no server required)
- REST API validation endpoint
- SEO content pages (how-to guides, comparisons)
- Marketing landing pages (/developers, /producers, /studios)

### Includes
- MIT license for SDK and server
- Docker support for server deployment
- Multiple download formats (static library, dynamic library, Rust crate)

## [0.1.0] - 2026-02-01

### Added
- Initial Archergate release
- Core SDK with machine-bound licensing
- Basic validation server
- Documentation and examples
