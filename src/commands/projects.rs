use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::config::user::UserConfig;

// This just serves as an example for now
// of how to create a command that is hidden from the help menu
// based on the user's permissions.

pub fn is_hidden() -> bool {
    !UserConfig::get_permissions().contains(&"tenant:manage".to_string())
}

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true,
    hide = is_hidden(),
)]
pub struct Projects {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Projects {
    pub fn execute(self) -> Result<()> {
        match self.command {
            None => Ok(()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
}
