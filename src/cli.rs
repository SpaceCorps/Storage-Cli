//! Clap command definitions for Storage CLI.

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "storage",
    version,
    about = "CLI for Azure Blob Storage - upload files and folders with time-limited read-only SAS URLs",
    after_help = "An LLM agent should start with: storage agent-readme",
    propagate_version = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Print raw JSON instead of YAML, for scripting
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Upload a file or folder to Azure Blob Storage and get a read-only SAS URL
    Upload(UploadArgs),

    /// Log in or configure an Azure Storage profile with credentials
    Login(LoginArgs),

    /// Manage configured storage profiles and stored credentials
    #[command(subcommand)]
    Accounts(AccountsCommand),

    /// Print the operating manual for an LLM agent driving this CLI
    AgentReadme,
}

// ---------------------------------------------------------------------------------------------
// upload

#[derive(Args, Clone, Debug)]
pub struct UploadArgs {
    /// Storage profile name or path to upload
    #[arg(value_name = "PROFILE_OR_PATH")]
    pub first: String,

    /// Path to upload when profile name is passed as the first argument
    #[arg(value_name = "PATH")]
    pub second: Option<String>,

    /// Storage profile / account name (alternative to positional profile)
    #[arg(short = 'a', long = "account", value_name = "ACCOUNT")]
    pub account: Option<String>,

    /// Override the storage container name
    #[arg(short = 'c', long = "container", value_name = "CONTAINER")]
    pub container: Option<String>,

    /// Force zip compression even for single files
    #[arg(long)]
    pub zipped: bool,

    /// Expiry date for the SAS URL (YYYY-MM-DD or RFC3339, defaults to 30 days from now)
    #[arg(long, value_name = "DATE")]
    pub expires: Option<String>,

    /// Expiry duration in hours from now (e.g. 24, 720)
    #[arg(long, value_name = "HOURS", conflicts_with = "expires")]
    pub expiry_hours: Option<u64>,

    /// Custom storage endpoint URL (e.g. for Azurite or mock servers)
    #[arg(long, value_name = "URL")]
    pub endpoint: Option<String>,

    /// Explicit SAS token to use for upload/read operations
    #[arg(long, value_name = "TOKEN")]
    pub sas_token: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// login & accounts

#[derive(Args, Clone, Debug)]
pub struct LoginArgs {
    /// Profile name to configure (default: "default")
    #[arg(value_name = "NAME", default_value = "default")]
    pub name: String,

    /// Azure Storage account name (defaults to profile name)
    #[arg(long, value_name = "ACCOUNT_NAME")]
    pub account_name: Option<String>,

    /// Default container name for this profile
    #[arg(long, value_name = "CONTAINER")]
    pub container: Option<String>,

    /// Azure Storage account key or SAS token (prompted without echo if omitted)
    #[arg(long, value_name = "KEY", conflicts_with = "key_stdin")]
    pub key: Option<String>,

    /// Read key or SAS token from stdin
    #[arg(long)]
    pub key_stdin: bool,

    /// Custom endpoint URL
    #[arg(long, value_name = "URL")]
    pub endpoint: Option<String>,

    /// Replace an existing profile with the same name
    #[arg(long)]
    pub force: bool,

    /// Skip checking container accessibility before saving
    #[arg(long)]
    pub no_verify: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum AccountsCommand {
    /// Add or update a storage profile and store its credentials in the OS keystore
    Add {
        /// Short profile name used to identify this storage account
        name: String,

        /// Azure Storage account name (defaults to profile name)
        #[arg(long, value_name = "ACCOUNT_NAME")]
        account_name: Option<String>,

        /// Default container name
        #[arg(long, value_name = "CONTAINER")]
        container: Option<String>,

        /// Azure Storage account key or SAS token (prompted without echo if omitted)
        #[arg(long, value_name = "KEY", conflicts_with = "key_stdin")]
        key: Option<String>,

        /// Read key or SAS token from stdin
        #[arg(long)]
        key_stdin: bool,

        /// Custom endpoint URL
        #[arg(long, value_name = "URL")]
        endpoint: Option<String>,

        /// Replace an existing profile with the same name
        #[arg(long)]
        force: bool,

        /// Skip verifying credentials before saving
        #[arg(long)]
        no_verify: bool,
    },

    /// List configured storage profiles
    List {
        /// Test reachability of each profile
        #[arg(long)]
        check: bool,
    },

    /// Test credentials and connectivity for a storage profile
    Test {
        /// Profile name
        name: String,
    },

    /// Remove a storage profile and delete its stored credentials
    Remove {
        /// Profile name
        name: String,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
