//! The operating manual an agent reads before calling `storage`.
//! Markdown by default; `--json` outputs structured machine-readable rules and exit codes.

use crate::{obj, output};

pub fn print() {
    if output::json() {
        output::write(&obj! {
            "tool" => "storage",
            "apiVersion" => API_VERSION,
            "rules" => RULES,
            "exitCodes" => obj! {
                "0" => "ok",
                "1" => "error - unclassified failure, report and stop",
                "2" => "network - transient connection issue, retry once then stop",
                "3" => "auth_required - stop, credentials missing or rejected, surface remediation",
                "4" => "not_found - storage container or file not found, do not retry",
                "5" => "rate_limited - rate limit hit, back off before retrying",
                "6" => "invalid_input - bad arguments, fix the call",
                "7" => "no_account - run storage accounts list or storage login",
            },
        });
        return;
    }
    println!("{README}");
}

pub const API_VERSION: &str = "2024-11-04";

const RULES: &[&str] = &[
    "Specify the storage profile as the first argument or via --account / -a.",
    "Run 'storage accounts list' first if you do not know which profiles exist; ask the human if multiple exist.",
    "Uploading a directory automatically packages it into a zip archive, omitting .git, bin, obj, node_modules.",
    "Use --zipped to force zip compression on single files.",
    "Returned SAS URLs are read-only and time-limited (defaults to 30 days from now).",
    "On code auth_required, stop and surface the remediation string. Do not retry.",
    "Use --json when you need structured JSON output for tool loops or jq.",
];

const README: &str = r#"# storage - agent operating manual

A native Rust CLI for Azure Blob Storage. Upload files and directories with automatic zip
packaging and receive time-limited, read-only SAS URLs. Output defaults to clean YAML,
or raw JSON when `--json` is supplied. Errors are emitted to stderr as a structured envelope.

## Profile and Account Resolution

Every upload targets a named storage profile configured in `config.yaml` or added via `storage login`:

    storage upload ivy-tendril ./report.pdf
    storage upload ./report.pdf -a ivy-tendril
    storage accounts list

If you do not know which profile to use, run `storage accounts list` to inspect available profiles.

### Managing Profiles & Authentication

    storage login <name> [--account-name <name>] [--container <container>] [--key <key>]
    storage accounts add <name> [--account-name <name>] [--container <container>] [--key <key>]
    storage accounts list [--check]
    storage accounts test <name>
    storage accounts remove <name> --yes

Credentials (storage account keys or SAS tokens) are stored in the OS keystore (macOS Keychain,
Windows DPAPI, Linux secret-tool). If no key is stored in the keystore, `storage` automatically
uses the active Azure CLI (`az login`) session to generate SAS tokens.

## Uploading

    # Upload a single file (default 30 days SAS expiry)
    storage upload ivy-tendril ./report.pdf

    # Upload a directory (automatically creates zip archive, excluding .git/bin/obj/node_modules)
    storage upload ivy-tendril ./dist/

    # Force zip compression on a single file
    storage upload ivy-tendril ./server.log --zipped

    # Custom expiry date or hours
    storage upload ivy-tendril ./data.csv --expires 2026-12-31
    storage upload ivy-tendril ./data.csv --expiry-hours 48

    # Custom container override
    storage upload ivy-tendril ./data.csv --container public-assets

    # JSON output for machine parsing
    storage upload ivy-tendril ./report.pdf --json

## Output Format

On success, `storage` emits:

```yaml
status: uploaded
account: stivytelemetry
container: ivy-tendril
blob: report.pdf
size_bytes: 124508
content_type: application/pdf
expires: 2026-10-24
url: https://stivytelemetry.blob.core.windows.net/ivy-tendril/report.pdf?se=...
```

Or JSON with `--json`:

```json
{
  "status": "uploaded",
  "account": "stivytelemetry",
  "container": "ivy-tendril",
  "blob": "report.pdf",
  "size_bytes": 124508,
  "content_type": "application/pdf",
  "expires": "2026-10-24",
  "url": "https://stivytelemetry.blob.core.windows.net/ivy-tendril/report.pdf?se=..."
}
```

## Exit Codes

    0  ok
    1  error          unclassified - report it and stop
    2  network        transient error - retry once, then stop
    3  auth_required  stop; give the human the remediation string verbatim
    4  not_found      the container or file does not exist
    5  rate_limited   back off before trying again
    6  invalid_input  fix the call
    7  no_account     profile not found; run `storage accounts list`

## Environment Variables

- `STORAGE_CONFIG_DIR`: custom directory for `config.yaml`
- `STORAGE_SECRET_STORE`: force `keychain`, `libsecret`, `dpapi`, or `plaintext`
- `STORAGE_ALLOW_PLAINTEXT_STORE=1`: allow fallback to unencrypted local file if no OS keystore exists
- `STORAGE_ENDPOINT`: override Azure Blob endpoint (e.g. for Azurite or mock servers)
- `STORAGE_SAS_TOKEN`: explicit SAS token for uploads/reads without calling Azure CLI
"#;
