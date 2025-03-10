use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::FuzzySelect;
use tabled::Table;

use crate::commands::CommandBase;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true,
    visible_alias = "proj"
)]
pub struct Projects {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Projects {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        match self.command {
            Some(Commands::Create(create)) => create.execute(base),
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Delete(delete)) => delete.execute(base),
            Some(Commands::Switch(switch)) => switch.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a project
    Create(Create),
    /// List projects   
    List(List),
    /// Delete a project
    Delete(Delete),
    /// Switch to a project
    Switch(Switch),
}

#[derive(Debug, Parser)]
pub struct Create {
    #[arg(help = "Name of the project to create")]
    name: String,
}

impl Create {
    pub fn execute(self, mut base: CommandBase) -> Result<()> {
        let tenant_name = base.get_tenant()?;

        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base
            .api_client()
            .create_project(token, &tenant_name, &self.name)?;
        println!("Project created: {}", response.name);

        base.user_config_mut()
            .write_default_project(self.name.clone())?;

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct List {}

impl List {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let tenant_name = base.get_tenant()?;

        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client().get_projects(token, &tenant_name)?;

        let table = Table::new(response.projects).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct Delete {
    #[arg(help = "Name of the project to delete")]
    name: String,

    #[arg(long, help = "Skip confirmation", default_missing_value("true"), default_value("false"), num_args(0..=1), require_equals(true))]
    no_confirm: Option<bool>,
}

impl Delete {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let tenant_name = base.get_tenant()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        if !self.confirm_deletion(&tenant_name, &self.name)? {
            println!("Delete cancelled");
            return Ok(());
        }

        base.api_client()
            .delete_project(token, &tenant_name, &self.name)?;

        println!("Project deleted: {}", self.name);
        Ok(())
    }

    fn confirm_deletion(&self, tenant_name: &str, project_name: &str) -> Result<bool> {
        if self.no_confirm == Some(true) {
            return Ok(true);
        }

        let prompt = format!(
            "Tenant: {}, Project: {}, Environment: {}.\nAre you sure you want to delete this environment and everything in it?",
            tenant_name, project_name, self.name
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

#[derive(Debug, Parser)]
pub struct Switch {
    #[arg(help = "Name of the project to switch to")]
    name: Option<String>,
}

impl Switch {
    pub fn execute(self, mut base: CommandBase) -> Result<()> {
        let response = base.api_client().get_projects(
            base.user_config().get_token().unwrap(),
            base.get_tenant()?.as_str(),
        )?;
        let projects = response.projects;

        let project_name = match self.name {
            Some(name) => {
                if !projects.iter().any(|p| p.name == name) {
                    return Err(anyhow!("Project {} not found", name));
                }
                name
            }
            None => {
                let selection =
                    FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Please select your project: ")
                        .items(&projects.iter().map(|p| p.name.as_str()).collect::<Vec<_>>())
                        .interact()
                        .unwrap();
                projects[selection].name.clone()
            }
        };

        base.user_config_mut()
            .write_default_project(project_name.clone())?;
        println!("Switched to project: {}", project_name);
        Ok(())
    }
}
