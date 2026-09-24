//! Storage profiles and non-secret account metadata in a readable YAML file.
//! The API key or secret token itself never lands here - that goes to [`crate::secrets`].
//!
//! The file layout and structure match NielsBosma's original `config.yaml` with `storage:` profiles,
//! ensuring existing profiles keep working seamlessly.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

fn default_provider() -> String {
    "azure".to_string()
}

fn default_version() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StorageProfile {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(rename = "account_name")]
    pub account_name: String,
    #[serde(rename = "container_name")]
    pub container_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "added_at")]
    pub added_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub storage: IndexMap<String, StorageProfile>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub accounts: IndexMap<String, StorageProfile>,
}

impl Default for Config {
    fn default() -> Self {
        Config { version: 1, storage: IndexMap::new(), accounts: IndexMap::new() }
    }
}

impl Config {
    /// Case-insensitive search in both `storage` and `accounts` maps.
    pub fn find(&self, name: &str) -> Option<(&String, &StorageProfile)> {
        self.storage
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .or_else(|| self.accounts.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)))
    }

    /// List all profiles sorted by name.
    pub fn sorted(&self) -> Vec<(&String, &StorageProfile)> {
        let mut v: Vec<(&String, &StorageProfile)> = self.storage.iter().collect();
        for (k, p) in &self.accounts {
            if !self.storage.contains_key(k) {
                v.push((k, p));
            }
        }
        v.sort_by_key(|(k, _)| k.to_lowercase());
        v
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("STORAGE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }

    #[cfg(windows)]
    let dir = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default().join("storage-cli");

    #[cfg(not(windows))]
    let dir = {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        if cfg!(target_os = "macos") {
            home.join("Library").join("Application Support").join("storage-cli")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"))
                .join("storage-cli")
        }
    };

    dir
}

pub fn config_path() -> PathBuf {
    let standard = config_dir().join("config.yaml");
    if standard.exists() {
        return standard;
    }

    // Check local fallback locations if the primary doesn't exist yet
    if Path::new("config.yaml").exists() {
        return PathBuf::from("config.yaml");
    }
    if Path::new("src/config.yaml").exists() {
        return PathBuf::from("src/config.yaml");
    }

    standard
}

pub fn ensure_dir() -> Result<()> {
    let dir = config_dir();
    if dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(&dir)?;
    restrict_to_owner(&dir);
    Ok(())
}

pub fn load() -> Result<Config> {
    let path = config_path();
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e.into()),
    };
    if text.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_norway::from_str::<Option<Config>>(&text).map(Option::unwrap_or_default).map_err(|e| {
        Error::other(format!("{} is not valid.", path.display()))
            .detail(e.to_string())
            .fix(format!("Fix or delete {}, then run: storage accounts add <name>", path.display()))
    })
}

pub fn save(config: &Config) -> Result<()> {
    ensure_dir()?;
    let yaml = serde_norway::to_string(config).map_err(|e| Error::other(e.to_string()))?;
    atomic_write(&config_dir().join("config.yaml"), yaml.as_bytes())
}

/// Write to a temp file in the same directory, then rename over - never a partial file.
pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, contents)?;
    restrict_to_owner(&tmp);
    fs::rename(&tmp, path)?;
    Ok(())
}

/// 0600 (0700 for directories) on Unix. On Windows the DPAPI blob is already user-scoped.
pub fn restrict_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = path;
}

pub struct Lock(#[allow(dead_code)] File);

pub fn lock() -> Result<Lock> {
    ensure_dir()?;
    let path = config_dir().join(".lock");
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(&path)?;

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut delay = Duration::from_millis(25);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Lock(file)),
            Err(fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(400));
            }
            Err(fs::TryLockError::WouldBlock) => {
                return Err(Error::other(format!(
                    "Timed out waiting for the lock at {}. Another storage process may be stuck.",
                    path.display()
                )));
            }
            Err(fs::TryLockError::Error(e)) => return Err(e.into()),
        }
    }
}

pub fn now_utc() -> String {
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    format_utc(secs)
}

fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_790_000_000), "2026-09-21T14:13:20Z");
    }

    #[test]
    fn reads_the_dotnet_layout() {
        let yaml = "storage:\n  ivy-tendril:\n    provider: azure\n    account_name: stivytelemetry\n    container_name: ivy-tendril\n";
        let c: Config = serde_norway::from_str(yaml).unwrap();
        let (name, profile) = c.find("ivy-tendril").unwrap();
        assert_eq!(name, "ivy-tendril");
        assert_eq!(profile.provider, "azure");
        assert_eq!(profile.account_name, "stivytelemetry");
        assert_eq!(profile.container_name, "ivy-tendril");
    }
}
