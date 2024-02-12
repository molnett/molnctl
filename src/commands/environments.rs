use anyhow::Result;
use clap::{Parser, Subcommand};
use super::CommandBase;

#[derive(Parser)]
#[derive(Debug)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Environments {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Environments {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        match &self.command {
            Some(Commands::Create(create)) => create.execute(base),
            None => Ok(())
        }
    }
}

#[derive(Subcommand)]
#[derive(Debug)]
pub enum Commands {
    /// Create an environment
    #[command(arg_required_else_help = true)]
    Create(Create),
}

#[derive(Parser)]
#[derive(Debug)]
pub struct Create {
    #[arg(help = "Name of the environment to create")]
    name: String,
}

impl Create {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        Ok(())
    }
}
