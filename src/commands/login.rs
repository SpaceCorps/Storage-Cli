//! Interactive or scripted login to configure an Azure Storage profile.

use crate::cli::LoginArgs;
use crate::commands::accounts;
use crate::error::Result;

pub fn run(args: LoginArgs) -> Result<()> {
    accounts::add(args.name, args.account_name, args.container, args.key, args.key_stdin, args.endpoint, args.force)
}
