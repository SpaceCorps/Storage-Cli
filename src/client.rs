//! Azure Blob Storage REST API client via `ureq`.
//!
//! Handles BlockBlob uploads (`PUT /<container>/<blob>`), SAS token generation via
//! `az` CLI or explicit SAS tokens, and read-only SAS URL construction.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use ureq::Agent;

use crate::error::{Error, ErrorCode, Result};

const DEFAULT_TIMEOUT_SECS: u64 = 120;

#[derive(Clone)]
pub struct Client {
    agent: Agent,
    endpoint_base: String,
    account_name: String,
    container_name: String,
    secret: Option<String>,
}

impl Client {
    pub fn new(
        account_name: &str,
        container_name: &str,
        custom_endpoint: Option<&str>,
        secret: Option<&str>,
    ) -> Client {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .http_status_as_error(false)
            .user_agent(concat!("storage-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();

        // Check env override first (used in mock tests), then custom endpoint, then Azure public endpoint
        let endpoint_base =
            if let Some(env_ep) = std::env::var("STORAGE_ENDPOINT").ok().filter(|s| !s.trim().is_empty()) {
                ensure_trailing_slash(env_ep)
            } else if let Some(ep) = custom_endpoint.filter(|s| !s.trim().is_empty()) {
                ensure_trailing_slash(ep.to_string())
            } else {
                format!("https://{account_name}.blob.core.windows.net/")
            };

        Client {
            agent,
            endpoint_base,
            account_name: account_name.to_string(),
            container_name: container_name.to_string(),
            secret: secret.filter(|s| !s.trim().is_empty()).map(str::to_string),
        }
    }

    #[allow(dead_code)]
    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    #[allow(dead_code)]
    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    #[allow(dead_code)]
    pub fn endpoint_base(&self) -> &str {
        &self.endpoint_base
    }

    /// Generates a SAS token for the specified permissions ("acw" for upload, "r" for read).
    pub fn generate_sas_token(&self, permissions: &str, expires_str: &str) -> Result<String> {
        // 1. Check if STORAGE_SAS_TOKEN env var is set (useful for scripting, CI, and test mocks)
        if let Some(env_token) = std::env::var("STORAGE_SAS_TOKEN").ok().filter(|s| !s.trim().is_empty()) {
            return Ok(env_token.trim_start_matches('?').to_string());
        }

        // 2. If stored secret already looks like a query SAS token (e.g. starts with "sv=" or "?")
        if let Some(s) = &self.secret
            && (s.contains("sv=") || s.contains("sig="))
        {
            return Ok(s.trim_start_matches('?').to_string());
        }

        // 3. Generate SAS token via Azure CLI ('az storage container generate-sas')
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd.exe");
            c.arg("/c").arg("az");
            c
        } else {
            Command::new("az")
        };

        cmd.args([
            "storage",
            "container",
            "generate-sas",
            "--account-name",
            &self.account_name,
            "--name",
            &self.container_name,
            "--permissions",
            permissions,
            "--expiry",
            expires_str,
            "--https-only",
            "--output",
            "tsv",
        ]);

        // If secret is an account key, supply --account-key
        if let Some(key) = &self.secret
            && !key.contains("sv=")
            && !key.contains("sig=")
        {
            cmd.args(["--account-key", key]);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::new(ErrorCode::AuthRequired, "Azure CLI ('az') was not found on PATH.")
                    .detail("A SAS token could not be generated without the Azure CLI.")
                    .fix("Install Azure CLI ('az login') or set STORAGE_SAS_TOKEN environment variable.")
            } else {
                Error::other(format!("Could not run Azure CLI: {e}"))
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(Error::new(ErrorCode::AuthRequired, "Failed to generate SAS token via Azure CLI.")
                .detail(stderr)
                .fix("Ensure you are logged in ('az login') and have permissions on the storage account."));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().trim_start_matches('?').to_string();
        if token.is_empty() {
            return Err(Error::new(
                ErrorCode::AuthRequired,
                "Azure CLI returned an empty SAS token. Ensure you are logged in ('az login').",
            ));
        }

        Ok(token)
    }

    /// Uploads file content as a BlockBlob via PUT REST API.
    pub fn upload_blob(
        &self,
        blob_name: &str,
        content: &[u8],
        content_type: &str,
        upload_sas_token: Option<&str>,
    ) -> Result<()> {
        let sas_query = match upload_sas_token.filter(|s| !s.trim().is_empty()) {
            Some(tok) => format!("?{}", tok.trim_start_matches('?')),
            None => String::new(),
        };

        let url = format!("{}{}/{}{}", self.endpoint_base, self.container_name, blob_name, sas_query);

        let response = self
            .agent
            .put(&url)
            .header("x-ms-blob-type", "BlockBlob")
            .header("Content-Type", content_type)
            .header("Content-Length", &content.len().to_string())
            .send(content)
            .map_err(transport_error)?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let body = String::from_utf8_lossy(
                &response.into_body().with_config().limit(64 * 1024).read_to_vec().unwrap_or_default(),
            )
            .trim()
            .to_string();
            return Err(status_error(status, &body));
        }

        Ok(())
    }

    /// Builds a read-only SAS URL for the uploaded blob.
    pub fn build_read_sas_url(&self, blob_name: &str, read_sas_token: &str) -> String {
        let sas = read_sas_token.trim_start_matches('?');
        format!(
            "{}{}/{}{}",
            self.endpoint_base,
            self.container_name,
            blob_name,
            if sas.is_empty() { String::new() } else { format!("?{sas}") }
        )
    }
}

fn ensure_trailing_slash(mut s: String) -> String {
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn transport_error(e: ureq::Error) -> Error {
    match e {
        ureq::Error::Timeout(_) => {
            Error::new(ErrorCode::Network, "The upload request timed out.").fix("Retry once, then stop.")
        }
        other => Error::new(ErrorCode::Network, "Could not reach Azure Blob Storage.")
            .detail(other.to_string())
            .fix("Check your network connection or endpoint configuration."),
    }
}

pub fn status_error(status: u16, body: &str) -> Error {
    let mut detail = format!("HTTP {status}");
    if !body.is_empty() {
        detail.push_str(": ");
        detail.push_str(body);
    }

    let e = match status {
        401 | 403 => Error::new(ErrorCode::AuthRequired, "Azure Blob Storage authentication failed.")
            .fix("Verify your SAS token, account key, or Azure CLI login ('az login')."),
        404 => Error::new(ErrorCode::NotFound, "The storage container or resource was not found.")
            .fix("Check that the container exists in the Azure Storage account."),
        429 => Error::new(ErrorCode::RateLimited, "Rate limited by Azure Storage.").fix("Back off before retrying."),
        400 | 422 => Error::new(ErrorCode::InvalidInput, "Azure Storage refused the upload request."),
        s if s >= 500 => Error::new(ErrorCode::Network, "Azure Storage returned a server error.")
            .fix("Retry; if it persists, check Azure status."),
        _ => Error::new(ErrorCode::Error, "Upload to Azure Storage failed."),
    };
    e.detail(detail)
}

pub fn mime_type_for_path(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "txt" | "log" | "md" => "text/plain",
        "csv" => "text/csv",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "wasm" => "application/wasm",
        "yaml" | "yml" => "application/yaml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_detection() {
        assert_eq!(mime_type_for_path(Path::new("doc.pdf")), "application/pdf");
        assert_eq!(mime_type_for_path(Path::new("image.png")), "image/png");
        assert_eq!(mime_type_for_path(Path::new("archive.zip")), "application/zip");
        assert_eq!(mime_type_for_path(Path::new("unknown.xyz")), "application/octet-stream");
    }

    #[test]
    fn test_sas_url_construction() {
        let client = Client::new("testaccount", "testcontainer", None, None);
        let url = client.build_read_sas_url("file.txt", "sp=r&sig=abc");
        assert_eq!(url, "https://testaccount.blob.core.windows.net/testcontainer/file.txt?sp=r&sig=abc");
    }
}
