# Archergate License Check

GitHub Action that verifies your binary includes Archergate license protection before you ship it. Catches unprotected releases in CI.

## Usage

```yaml
- name: Check license protection
  uses: lailaarcher/archergate-action@v1
  with:
    binary: target/release/my-app
    platform: linux
```

Add it after your build step, before your release step. If the binary doesn't contain Archergate license symbols, the build fails.

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `binary` | yes | | Path to compiled binary |
| `platform` | no | `linux` | Target platform (windows, macos, linux) |
| `fail-on-missing` | no | `true` | Fail the build if protection is missing |

## Outputs

| Output | Description |
|---|---|
| `protected` | `true` if Archergate symbols were found |
| `symbols-found` | List of detected symbols |

## Full example

```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - run: cargo build --release

      - name: Verify license protection
        uses: lailaarcher/archergate-action@v1
        with:
          binary: target/release/my-app

      - name: Release
        uses: softprops/action-gh-release@v2
        with:
          files: target/release/my-app
```

## What it checks

Scans the compiled binary for Archergate FFI symbols (`ag_license_new`, `ag_license_validate`, etc.) using `nm` and `strings`. If the symbols aren't linked in, the binary ships without protection.

This doesn't replace integration testing. It's a safety net that catches the case where someone accidentally removes the license check or misconfigures the build.
