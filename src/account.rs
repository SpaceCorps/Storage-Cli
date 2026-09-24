//! Account and profile resolution.
//!
//! Every command that mutates or interacts with Azure Storage targets an explicit account/profile,
//! avoiding implicit defaults and accidental uploads to the wrong container or account.

use crate::config::{self, Config, StorageProfile};
use crate::error::{Error, ErrorCode, Result};
use crate::secrets;

#[derive(Debug)]
pub struct Resolved {
    pub name: String,
    pub profile: StorageProfile,
    pub secret: Option<String>,
}

impl Resolved {
    pub fn account_name(&self) -> &str {
        &self.profile.account_name
    }

    pub fn container_name(&self) -> &str {
        &self.profile.container_name
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.profile.endpoint.as_deref()
    }
}

pub fn resolve(requested: Option<&str>) -> Result<Resolved> {
    let config = config::load()?;

    let Some(requested) = requested.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(Error::new(
            ErrorCode::NoAccount,
            "No storage profile specified. Pass --account <name> or <profile>.",
        )
        .detail(describe(&config))
        .fix("storage accounts list"));
    };

    let Some((name, profile)) = config.find(requested) else {
        return Err(Error::new(ErrorCode::NoAccount, format!("No storage profile named '{requested}'."))
            .detail(describe(&config))
            .fix("storage accounts list"));
    };

    let secret = secrets::store()?.get(&secrets::account_key(name))?;

    Ok(Resolved { name: name.clone(), profile: profile.clone(), secret })
}

fn describe(config: &Config) -> String {
    let profiles = config.sorted();
    if profiles.is_empty() {
        return "No storage profiles are configured yet. Run 'storage login <name>' or 'storage accounts add <name>'."
            .into();
    }
    let listed: Vec<String> = profiles
        .into_iter()
        .map(|(k, v)| format!("{k} (account: {}, container: {})", v.account_name, v.container_name))
        .collect();
    format!("Configured profiles: {}", listed.join(", "))
}
