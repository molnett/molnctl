use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::FuzzySelect;
use tabled::Table;

use super::CommandBase;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Tenants {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Tenants {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        match self.command {
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Switch(switch)) => switch.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List your tenants
    List(List),
    /// Switch default tenant for all commands
    Switch(Switch),
}

#[derive(Parser, Debug)]
pub struct List {}

impl List {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client().get_tenants(token)?;

        let table = Table::new(response.tenants).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Switch {
    #[arg(help = "Name of the tenant to switch to")]
    tenant: Option<String>,
}

impl Switch {
    pub fn execute(self, mut base: CommandBase) -> Result<()> {
        let response = base
            .api_client()
            .get_tenants(base.user_config().get_token().unwrap())?;
        let tenant_names = response
            .tenants
            .iter()
            .map(|o| o.name.as_str())
            .collect::<Vec<_>>();

        let tenant_name = if self.tenant.is_some() {
            let arg_tenant = self.tenant.clone().unwrap();
            if tenant_names.contains(&arg_tenant.as_str()) {
                arg_tenant
            } else {
                return Err(anyhow!(
                    "tenant {} does not exist or you do not have access to it",
                    arg_tenant
                ));
            }
        } else {
            let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Please select your tenant: ")
                .items(&tenant_names[..])
                .interact()
                .unwrap();
            tenant_names[selection].to_string()
        };

        match base.user_config_mut().write_default_tenant(tenant_name) {
            Ok(_) => Ok(()),
            Err(err) => {
                println!("Error while writing config: {}", err);
                Ok(())
            }
        }
    }
}
