pub mod accounts;
pub mod login;
pub mod upload;

use crate::cli::Command;
use crate::error::Result;
use crate::readme;

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Upload(args) => upload::run(args),
        Command::Login(args) => login::run(args),
        Command::Accounts(cmd) => accounts::run(cmd),
        Command::AgentReadme => {
            readme::print();
            Ok(())
        }
    }
}
