//! Storage profiles and credential management (`storage accounts`).

use std::io::{BufRead, IsTerminal, Write};

use serde_json::Value;

use crate::account;
use crate::cli::AccountsCommand;
use crate::client::Client;
use crate::config::{self, StorageProfile};
use crate::error::{Error, Result};
use crate::secrets;
use crate::{obj, output};

pub fn run(cmd: AccountsCommand) -> Result<()> {
    match cmd {
        AccountsCommand::Add { name, account_name, container, key, key_stdin, endpoint, force, no_verify: _ } => {
            add(name, account_name, container, key, key_stdin, endpoint, force)
        }
        AccountsCommand::List { check } => list(check),
        AccountsCommand::Test { name } => test(&name),
        AccountsCommand::Remove { name, yes } => remove(&name, yes),
    }
}

pub(crate) fn add(
    name: String,
    account_name: Option<String>,
    container: Option<String>,
    key_flag: Option<String>,
    key_stdin: bool,
    endpoint: Option<String>,
    force: bool,
) -> Result<()> {
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() {
        return Err(Error::invalid("Profile name cannot be empty."));
    }

    let actual_account_name = account_name.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| name_trimmed.to_string());

    let actual_container = container.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| name_trimmed.to_string());

    // Acquire lock before inspecting and modifying config
    let _lock = config::lock()?;
    let mut cfg = config::load()?;

    if cfg.find(name_trimmed).is_some() && !force {
        return Err(Error::invalid(format!("Profile '{name_trimmed}' already exists."))
            .detail("An existing profile cannot be overwritten without --force.")
            .fix(format!("storage accounts add {name_trimmed} --force")));
    }

    // Resolve key from flag, stdin, or interactive prompt
    let secret_opt = if key_stdin {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else if let Some(k) = key_flag {
        let trimmed = k.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else if std::io::stdin().is_terminal() {
        eprint!("Enter Azure Storage account key or SAS token (optional, press Enter to use az CLI): ");
        std::io::stderr().flush()?;
        let pass = rpassword::read_password().unwrap_or_default();
        let trimmed = pass.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else {
        None
    };

    if let Some(secret) = &secret_opt {
        secrets::store()?.set(&secrets::account_key(name_trimmed), secret)?;
    }

    let profile = StorageProfile {
        provider: "azure".to_string(),
        account_name: actual_account_name.clone(),
        container_name: actual_container.clone(),
        endpoint: endpoint.clone(),
        added_at: Some(config::now_utc()),
    };

    cfg.storage.insert(name_trimmed.to_string(), profile);
    config::save(&cfg)?;

    output::write(&obj! {
        "status" => "ok",
        "action" => "added",
        "profile" => name_trimmed,
        "account_name" => actual_account_name,
        "container_name" => actual_container,
        "endpoint" => endpoint.unwrap_or_default(),
        "has_stored_secret" => secret_opt.is_some(),
    });

    Ok(())
}

fn list(check: bool) -> Result<()> {
    let cfg = config::load()?;
    let profiles = cfg.sorted();

    let mut list = Vec::new();
    for (name, p) in profiles {
        let secret = secrets::store()?.get(&secrets::account_key(name)).unwrap_or(None);
        let has_secret = secret.is_some();

        let status = if check {
            let client = Client::new(&p.account_name, &p.container_name, p.endpoint.as_deref(), secret.as_deref());
            // Try generating a temporary test token or verify config
            match client.generate_sas_token("r", "2026-12-31") {
                Ok(_) => "valid",
                Err(_) => "unreachable_or_no_az",
            }
        } else {
            "configured"
        };

        list.push(obj! {
            "profile" => name,
            "provider" => &p.provider,
            "account_name" => &p.account_name,
            "container_name" => &p.container_name,
            "endpoint" => p.endpoint.as_deref().unwrap_or(""),
            "has_stored_key" => has_secret,
            "status" => status,
        });
    }

    output::write(&Value::Array(list));
    Ok(())
}

fn test(name: &str) -> Result<()> {
    let resolved = account::resolve(Some(name))?;
    let client = Client::new(
        resolved.account_name(),
        resolved.container_name(),
        resolved.endpoint(),
        resolved.secret.as_deref(),
    );

    // Test token generation
    let _token = client.generate_sas_token("r", "2026-12-31")?;

    output::write(&obj! {
        "status" => "ok",
        "profile" => resolved.name,
        "account_name" => resolved.account_name(),
        "container_name" => resolved.container_name(),
        "result" => "valid",
    });

    Ok(())
}

fn remove(name: &str, yes: bool) -> Result<()> {
    let _lock = config::lock()?;
    let mut cfg = config::load()?;

    let (key_name, _) = cfg
        .find(name)
        .map(|(k, p)| (k.clone(), p.clone()))
        .ok_or_else(|| Error::not_found(format!("Profile '{name}' does not exist.")))?;

    if !yes && std::io::stdin().is_terminal() {
        eprint!("Are you sure you want to remove profile '{key_name}'? (y/N): ");
        std::io::stderr().flush()?;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            return Err(Error::other("Operation aborted by user."));
        }
    }

    cfg.storage.shift_remove(&key_name);
    cfg.accounts.shift_remove(&key_name);
    config::save(&cfg)?;

    let _ = secrets::store()?.delete(&secrets::account_key(&key_name));

    output::write(&obj! {
        "status" => "ok",
        "action" => "removed",
        "profile" => key_name,
    });

    Ok(())
}
