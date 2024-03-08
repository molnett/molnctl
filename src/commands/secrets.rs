use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use super::CommandBase;
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
pub struct Secrets {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Secrets {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        match &self.command {
            Some(Commands::Create(create)) => create.execute(base),
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Delete(delete)) => delete.execute(base),
            None => Ok(())
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a secret
    #[command(arg_required_else_help = true)]
    Create(Create),
    /// List secrets
    List(List),
    /// Delete a secret
    Delete(Delete)
}

#[derive(Debug, Parser)]
pub struct Create {

}

impl Create {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct List {
    #[arg(long, help = "Environment to list the secrets of")]
    env: String,
}

impl List {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client().get_secrets(
            token,
            &org_name,
            &self.env
        )?;

        let table = Table::new(response.secrets).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct Delete {
    #[arg(help = "Name of the secret")]
    name: String,
    #[arg(long, help = "Environment the secret is in")]
    env: String,
    #[arg(long, help = "Skip confirmation", default_missing_value("true"), default_value("false"), num_args(0..=1), require_equals(true))]
    no_confirm: Option<bool>,
}

impl Delete {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        if let Some(false) = self.no_confirm {
            let prompt = format!("Org: {}, Environment: {}, Secret: {}. Are you sure you want to delete this secret?", org_name, self.env, self.name);
            FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&["no", "yes"])
                .default(0)
                .interact()
                .unwrap();
        }

        base.api_client().delete_secret(
            token,
            &org_name,
            &self.env,
            &self.name
        )?;

        println!("Secret {} deleted", self.name);
        Ok(())
    }
}
