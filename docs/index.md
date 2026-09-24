# Storage CLI - Native Rust CLI for Azure Blob Storage

> A blazing fast native command-line tool and agent interface for Azure Blob Storage. Upload files and directories with automatic zip packaging and receive time-limited, read-only SAS URLs.

- **Canonical Website**: [https://spacecorps.github.io/Storage-Cli/](https://spacecorps.github.io/Storage-Cli/)
- **Agent Discovery**: [https://spacecorps.github.io/Storage-Cli/llms.txt](https://spacecorps.github.io/Storage-Cli/llms.txt)
- **License**: [MIT](LICENSE)
- **Repository**: [https://github.com/SpaceCorps/Storage-Cli](https://github.com/SpaceCorps/Storage-Cli)

---

## Key Features

- **⚡ Native Cold-Start**: Standalone static binary executing in 1–3 ms. Zero runtime or interpreter overhead.
- **📦 Smart Directory Zipping**: Automatic zip packaging for directory uploads, excluding `.git`, `bin`, `obj`, `node_modules`, and `target`.
- **⏱️ Time-Limited Links**: Instant read-only SAS URL generation with configurable expiry (days or hours).
- **🔐 OS Keystore Integration**: Secrets never touch plaintext disk files. Storage account keys live in macOS Keychain, Windows DPAPI, or Linux Secret Service.
- **🤖 Agentic Protocol**: Structured `--json` flag, YAML-first terminal output, deterministic error envelopes on stderr, and built-in `<bin> agent-readme`.

---

## Installation

### Cargo
```bash
cargo install --git https://github.com/SpaceCorps/Storage-Cli --locked
```

### Pre-built Binaries
Precompiled standalone archives for macOS (ARM64 & x86_64), Linux (musl static), and Windows (x64) are available on [GitHub Releases](https://github.com/SpaceCorps/Storage-Cli/releases/latest).

---

## Quickstart

```bash
# 1. Configure profile
storage accounts add ivy-tendril --account-name stivytelemetry --container ivy-tendril

# 2. Upload file
storage upload ivy-tendril ./report.pdf

# 3. Upload directory (automatically zipped)
storage upload ivy-tendril ./dist/

# 4. Custom expiry date or hours
storage upload ivy-tendril ./data.csv --expires 2026-12-31
storage upload ivy-tendril ./data.csv --expiry-hours 48

# 5. Output raw JSON for agent tool loops
storage upload ivy-tendril ./report.pdf --json
```

---

## Documentation Links

- [About Storage CLI](about.html)
- [Authentication Guide](auth.md)
- [Pricing & Licensing](pricing.md)
- [Privacy Policy](privacy.html)
- [Contact & Support](contact.html)
- [LLMs Manifest](llms.txt)
- [Full Agent Reference Manual](llms-full.txt)
