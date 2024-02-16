use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use super::CommandBase;
use tabled::Table;

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
    #[arg(long, help = "Organization to create the environment in")]
    org: Option<String>,
}

impl Create {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = if self.org.is_some() {
            self.org.clone().unwrap()
        } else {
            base.user_config().get_default_org().unwrap().to_string()
        };
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client().create_environment(
            token,
            &self.name,
            &org_name
        )?;

        let table = Table::new([response]).to_string();
        println!("{}", table);

        Ok(())
    }
}
