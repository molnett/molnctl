use super::CommandBase;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::FuzzySelect;
use tabled::Table;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true,
    visible_alias = "envs"
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
            Some(Commands::Delete(delete)) => delete.execute(base),
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
    /// Delete an environment
    Delete(Delete),
}

#[derive(Debug, Parser)]
pub struct Create {
    #[arg(help = "Name of the environment to create")]
    name: String,

    #[arg(long, help = "Copy from an existing environment", num_args(0..=1), require_equals(true), value_name = "ENV_NAME",)]
    copy_from: Option<String>,
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
            .create_environment(token, &self.name, &org_name, self.copy_from.as_deref())?;

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

#[derive(Debug, Parser)]
pub struct Delete {
    #[arg(help = "Name of the environment")]
    name: String,
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

        if !self.confirm_deletion(&org_name)? {
            println!("Delete cancelled");
            return Ok(());
        }
    

        base.api_client()
            .delete_environment(token, &org_name, &self.name)?;

        println!("Environment {} deleted", self.name);
        Ok(())
    }

    fn confirm_deletion(&self, org_name: &str) -> Result<bool> {
        if self.no_confirm == Some(true) {
            return Ok(true);
        }

        let prompt = format!(
            "Org: {}, Environment: {}.\nAre you sure you want to delete this environment and everything in it?",
            org_name, self.name
        );

        let options = ["no", "yes"];
        let selected = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .items(&options)
            .default(0)
            .interact()?;

        Ok(options[selected] == "yes")
    }
}
