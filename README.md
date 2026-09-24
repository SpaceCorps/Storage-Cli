# Storage CLI

[![Release](https://img.shields.io/github/v/release/SpaceCorps/Storage-Cli?color=blue&label=version)](https://github.com/SpaceCorps/Storage-Cli/releases/latest)
[![CI](https://github.com/SpaceCorps/Storage-Cli/actions/workflows/ci.yml/badge.svg)](https://github.com/SpaceCorps/Storage-Cli/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-online-success)](https://spacecorps.github.io/Storage-Cli/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A blazing fast, native Rust command-line tool and autonomous agent interface for Azure Blob Storage. Upload files and directories with automatic zip compression and obtain time-limited, read-only SAS URLs.

Replaces the legacy .NET global tool `Storage.Console` by Niels Bosma with native speed, zero runtime dependencies, and an agent-ready interface.

---

## Highlights

- ⚡ **Sub-3ms Startup**: Compiled as a native static binary with zero runtime dependencies. Executes in 1–3 ms.
- 📦 **Automated Directory Packaging**: Uploading a folder automatically packages it into a zip archive, omitting build artifacts (`.git`, `bin`, `obj`, `node_modules`).
- ⏱️ **Time-Limited SAS URLs**: Generates read-only Shared Access Signature (SAS) URLs with custom expiry dates or hours (defaults to 30 days).
- 🔐 **OS Keystore Integration**: Secrets never touch plaintext disk. Credentials live in native vaults (macOS Keychain, Linux Secret Service, Windows DPAPI).
- 🤖 **Agentic Protocol**: Deterministic multi-account safety (`-a`), raw `--json` flag, structured error envelopes on stderr, and built-in `<bin> agent-readme`.
- 🔄 **Full Compatibility**: Supports existing `config.yaml` storage profiles from `Storage.Console`.

---

## Installation

### Using Cargo

```bash
cargo install --git https://github.com/SpaceCorps/Storage-Cli --locked
```

### Pre-built Standalone Binaries

Download standalone binary archives directly from the [GitHub Releases](https://github.com/SpaceCorps/Storage-Cli/releases/latest) page:

| Platform | Architecture | Binary Package |
|:---|:---|:---|
| **macOS** | Apple Silicon (`aarch64`) | [`storage-v1.0.0-aarch64-apple-darwin.tar.gz`](https://github.com/SpaceCorps/Storage-Cli/releases/download/v1.0.0/storage-v1.0.0-aarch64-apple-darwin.tar.gz) |
| **macOS** | Intel (`x86_64`) | [`storage-v1.0.0-x86_64-apple-darwin.tar.gz`](https://github.com/SpaceCorps/Storage-Cli/releases/download/v1.0.0/storage-v1.0.0-x86_64-apple-darwin.tar.gz) |
| **Linux** | x86_64 (musl static) | [`storage-v1.0.0-x86_64-unknown-linux-musl.tar.gz`](https://github.com/SpaceCorps/Storage-Cli/releases/download/v1.0.0/storage-v1.0.0-x86_64-unknown-linux-musl.tar.gz) |
| **Windows**| x64 (MSVC) | [`storage-v1.0.0-x86_64-pc-windows-msvc.zip`](https://github.com/SpaceCorps/Storage-Cli/releases/download/v1.0.0/storage-v1.0.0-x86_64-pc-windows-msvc.zip) |

---

## Quickstart

### 1. Configure Storage Profile

You can configure storage accounts interactively or via `config.yaml`:

```bash
# Add a storage profile with account name and container
storage accounts add ivy-tendril --account-name stivytelemetry --container ivy-tendril
```

Or configure via `config.yaml`:

```yaml
storage:
  ivy-tendril:
    provider: azure
    account_name: stivytelemetry
    container_name: ivy-tendril
```

### 2. Upload Files & Folders

```bash
# Upload a single file (default 30-day read-only SAS URL)
storage upload ivy-tendril ./report.pdf

# Upload a folder (auto-zipped, excluding .git, bin, obj, node_modules)
storage upload ivy-tendril ./my-project

# Upload using standard --account flag
storage upload ./report.pdf -a ivy-tendril

# Force zip compression on a single file
storage upload ivy-tendril ./large-file.log --zipped

# Custom expiry date or hours
storage upload ivy-tendril ./data.csv --expires 2026-12-31
storage upload ivy-tendril ./data.csv --expiry-hours 48

# Machine-readable JSON output for scripting or agents
storage upload ivy-tendril ./report.pdf --json
```

---

## Command Reference

| Command | Description |
|:---|:---|
| `storage upload <profile> <path>` | Upload a file or folder and output time-limited read-only SAS URL |
| `storage login [name]` | Configure storage profile and store credentials in OS keystore |
| `storage accounts add <name>` | Add a storage profile with custom container and account details |
| `storage accounts list [--check]` | List configured profiles and verify connectivity |
| `storage accounts test <name>` | Test credentials and SAS token generation for a profile |
| `storage accounts remove <name>` | Remove profile and purge stored credentials |
| `storage agent-readme [--json]` | Self-documenting manual for autonomous agents |

---

## License & Credits

- Licensed under the [MIT License](LICENSE).
- Ported from the original C# `Storage.Console` prototype created by [Niels Bosma](https://github.com/nielsbosma/Storage.Console).
- Maintained by [SpaceCorps](https://github.com/SpaceCorps).
