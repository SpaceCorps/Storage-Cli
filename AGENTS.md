# AGENTS.md

Notes for whoever extends or drives this tool.

`storage` is a native Rust CLI for Azure Blob Storage, built to be operated by humans and LLM agents. It replaced Niels Bosma's .NET global tool `Storage.Console` and retains full backward compatibility for `config.yaml` while adding native speed, OS keystore integration, deterministic multi-account safety, and agentic JSON protocols.

For the manual the *agent* reads, run `storage agent-readme` (or `storage agent-readme --json`). This file is for the human or agent modifying the tool's source code.

## Commands

```bash
cargo build --release              # target/release/storage
cargo test                         # unit tests + tests/cli.rs against an in-process mock server
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo install --path . --locked    # install on local PATH
```

## Testing Environment Variables

Use a throwaway config directory when testing so you never touch real credentials:

```bash
export STORAGE_CONFIG_DIR=$(mktemp -d) STORAGE_SECRET_STORE=plaintext STORAGE_ALLOW_PLAINTEXT_STORE=1
```

| Variable | Effect |
| --- | --- |
| `STORAGE_CONFIG_DIR` | Overrides the config directory containing `config.yaml` and keystores |
| `STORAGE_SECRET_STORE` | Forces a backend: `keychain`, `libsecret`, `dpapi`, `plaintext` |
| `STORAGE_ALLOW_PLAINTEXT_STORE=1` | Permits plaintext file fallback when no OS keystore is available |
| `STORAGE_ENDPOINT` | Overrides the Azure Blob endpoint URL (used for mock testing and Azurite) |
| `STORAGE_SAS_TOKEN` | Explicit SAS token bypass (avoids executing `az` CLI during test runs) |

## Layout

```
src/
  main.rs          arg parsing, --json pre-scan, clap error formatting
  cli.rs           command hierarchy (upload, login, accounts, agent-readme)
  commands/
    mod.rs         command dispatcher
    upload.rs      upload logic, directory packaging, SAS token generation
    accounts.rs    accounts add|list|test|remove
    login.rs       interactive / scripted profile login
  client.rs        blocking HTTP (ureq + rustls), Azure BlockBlob PUT, SAS URL builder
  error.rs         ErrorCode enum and Error {message, detail, remediation}
  output.rs        YAML by default, JSON with --json, write_error, obj! macro
  account.rs       profile / account resolution
  config.rs        config.yaml loader, atomic writes, 0600 file permissions, lock
  secrets.rs       Keychain (security), libsecret (secret-tool), DPAPI, plaintext fallback
  zip.rs           zip compression for directories and files (excluding build artifacts)
  readme.rs        agent-readme text and rules
tests/
  cli.rs           in-process TCP mock HTTP server test suite (offline, deterministic)
```

## Architectural Tenets

1. **Zero Runtime Dependencies**: Native standalone static binary executing in 1–3 ms.
2. **Blocking HTTP over Tokio**: `ureq` (rustls + gzip) avoids async runtime bloat for single-shot CLI tasks.
3. **OS Keystore Integration**: Secrets never touch plaintext files unless explicitly opted in via `STORAGE_ALLOW_PLAINTEXT_STORE=1`.
4. **Agentic Output Protocol**: YAML default for clean terminal inspection, `--json` for jq and agent tool loops, structured error envelopes on stderr with stable exit codes.
5. **Self-Documenting**: Embedded `agent-readme` command exposes full usage, rules, and exit code mappings without internet roundtrips.
