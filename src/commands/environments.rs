use super::CommandBase;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use tabled::Table;

#[derive(Debug, Parser)]
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
            Some(Commands::List(list)) => list.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create an environment
    #[command(arg_required_else_help = true)]
    Create(Create),
    /// List environments
    #[command()]
    List(List),
}

#[derive(Debug, Parser)]
pub struct Create {
    #[arg(help = "Name of the environment to create")]
    name: String,
}

impl Create {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base
            .api_client()
            .create_environment(token, &self.name, &org_name)?;

        let table = Table::new([response]).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct List {}

impl List {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client().get_environments(token, &org_name)?;

        println!("{:?}", response);

        Ok(())
    }
}
