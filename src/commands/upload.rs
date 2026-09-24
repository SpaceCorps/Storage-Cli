//! Handles the `upload` command: packaging, MIME detection, SAS token generation,
//! Azure BlockBlob upload, and SAS URL output.

use std::fs;
use std::path::{Path, PathBuf};

use crate::account;
use crate::cli::UploadArgs;
use crate::client::{self, Client};
use crate::error::{Error, Result};
use crate::zip;
use crate::{obj, output};

struct CleanupGuard(Option<PathBuf>);
impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn run(args: UploadArgs) -> Result<()> {
    // Determine profile and path from arguments
    let (profile_name, path_str) = match &args.second {
        Some(second) => (args.first.as_str(), second.as_str()),
        None => {
            if let Some(acc) = &args.account {
                (acc.as_str(), args.first.as_str())
            } else {
                // If only one positional is passed without -a, try resolving it as profile first,
                // or if it's an existing path on disk, prompt with NoAccount error
                let path = Path::new(&args.first);
                if path.exists() {
                    let err = match account::resolve(None) {
                        Err(e) => e,
                        Ok(_) => unreachable!(),
                    };
                    return Err(err);
                } else {
                    (args.first.as_str(), "")
                }
            }
        }
    };

    let target_path = Path::new(path_str);
    if !target_path.exists() {
        return Err(Error::invalid(format!("Path not found: {path_str}")));
    }

    let resolved = account::resolve(Some(profile_name))?;

    let container = args.container.as_deref().unwrap_or_else(|| resolved.container_name());

    let endpoint = args.endpoint.as_deref().or_else(|| resolved.endpoint());

    // Validate and compute expiry date string (YYYY-MM-DD)
    let expiry_str = compute_expiry_string(args.expires.as_deref(), args.expiry_hours)?;

    let is_dir = target_path.is_dir();
    let should_zip = is_dir || args.zipped;

    let mut cleanup = CleanupGuard(None);

    let (upload_file_path, blob_name) = if should_zip {
        if is_dir {
            let (zip_path, name) = zip::zip_directory(target_path)?;
            cleanup.0 = Some(zip_path.clone());
            (zip_path, name)
        } else {
            let (zip_path, name) = zip::zip_single_file(target_path)?;
            cleanup.0 = Some(zip_path.clone());
            (zip_path, name)
        }
    } else {
        let name = target_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::invalid(format!("Invalid file path: {}", target_path.display())))?
            .to_string();
        (target_path.to_path_buf(), name)
    };

    let file_bytes = fs::read(&upload_file_path)?;
    let content_type = client::mime_type_for_path(&upload_file_path);

    let client = Client::new(resolved.account_name(), container, endpoint, resolved.secret.as_deref());

    // 1. Generate or fetch upload SAS token (write permissions: "acw")
    let upload_sas_token = match &args.sas_token {
        Some(tok) => tok.clone(),
        None => client.generate_sas_token("acw", &expiry_str)?,
    };

    // 2. Upload BlockBlob via PUT REST API
    client.upload_blob(&blob_name, &file_bytes, content_type, Some(&upload_sas_token))?;

    // 3. Generate read-only SAS token (read permission: "r")
    let read_sas_token = match &args.sas_token {
        Some(tok) => tok.clone(),
        None => client.generate_sas_token("r", &expiry_str)?,
    };

    // 4. Construct read-only SAS URL
    let read_url = client.build_read_sas_url(&blob_name, &read_sas_token);

    // 5. Output structured result
    output::write(&obj! {
        "status" => "uploaded",
        "account" => resolved.account_name(),
        "container" => container,
        "blob" => blob_name,
        "size_bytes" => file_bytes.len(),
        "content_type" => content_type,
        "expires" => expiry_str,
        "url" => read_url,
    });

    Ok(())
}

fn compute_expiry_string(expires_arg: Option<&str>, expiry_hours: Option<u64>) -> Result<String> {
    if let Some(exp) = expires_arg {
        if !is_valid_date(exp) {
            return Err(Error::invalid(format!("Invalid date format: {exp}. Use YYYY-MM-DD.")));
        }
        return Ok(exp.to_string());
    }

    let now_secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);

    let target_secs = if let Some(hours) = expiry_hours {
        now_secs + (hours as i64) * 3600
    } else {
        // Default: 30 days
        now_secs + 30 * 86400
    };

    Ok(format_utc_date(target_secs))
}

fn is_valid_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if i == 4 || i == 7 {
            continue;
        }
        if !b.is_ascii_digit() {
            return false;
        }
    }
    true
}

fn format_utc_date(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_validation() {
        assert!(is_valid_date("2026-06-01"));
        assert!(is_valid_date("2030-12-31"));
        assert!(!is_valid_date("2026/06/01"));
        assert!(!is_valid_date("invalid"));
        assert!(!is_valid_date("2026-6-1"));
    }

    #[test]
    fn test_format_utc_date() {
        assert_eq!(format_utc_date(0), "1970-01-01");
        assert_eq!(format_utc_date(1_790_000_000), "2026-09-21");
    }
}
