use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use tabled::Table;

use crate::config::user::UserConfig;
use super::CommandBase;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Orgs {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Orgs {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        match self.command {
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Create(create)) => create.execute(base),
            Some(Commands::Switch(switch)) => switch.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List your orgs
    List(List),
    /// Create a new org
    Create(Create),
    /// Switch default org for all commands
    Switch(Switch),
}

#[derive(Parser, Debug)]
pub struct List {}

impl List {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base.get_token()?;
        let response = base.api_client().get_organizations(&token)?;

        let table = Table::new(response.organizations).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Create {
    #[clap(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    billing_email: Option<String>,
}

impl Create {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base.get_token()?;
        let plan = CreatePlan::builder()
            .name(self.name.as_deref())
            .billing_email(self.billing_email.as_deref())
            .build()?;

        let response = base.api_client().create_organization(
            &token,
            plan.name.as_str(),
            plan.billing_email.as_str(),
        )?;

        println!("{:#?}", response);

        Ok(())
    }
}

#[derive(Debug)]
struct CreatePlan {
    name: String,
    billing_email: String,
}

struct CreatePlanBuilder {
    name: String,
    billing_email: String,
}

impl CreatePlanBuilder {
    pub fn new() -> CreatePlanBuilder {
        CreatePlanBuilder {
            name: "".to_string(),
            billing_email: "".to_string(),
        }
    }

    pub fn name(mut self, name: Option<&str>) -> Self {
        if name.is_none() {
            let prompt = "Please enter a name for your organization: ";
            let input: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .interact_text()
                .unwrap();

            self.name = input;
        } else {
            self.name = name.unwrap().to_string();
        }

        self
    }

    pub fn billing_email(mut self, billing_email: Option<&str>) -> Self {
        if billing_email.is_some() {
            self.billing_email = billing_email.unwrap().to_string();
            return self;
        }

        let prompt = "Please enter the billing email for your organization: ";
        let input: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap();

        self.billing_email = input;

        self
    }

    fn verify(&self) -> Result<()> {
        Ok(())
    }

    pub fn build(self) -> Result<CreatePlan> {
        self.verify()?;
        Ok(CreatePlan {
            name: self.name,
            billing_email: self.billing_email,
        })
    }
}

impl CreatePlan {
    pub fn builder() -> CreatePlanBuilder {
        CreatePlanBuilder::new()
    }
}

#[derive(Parser, Debug)]
pub struct Switch {
    #[arg(help = "Name of the org to switch to")]
    org: Option<String>,
}

impl Switch {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base.get_token()?;
        let orgs = base
            .api_client()
            .get_organizations(&token)?;
        let org_names = orgs
            .organizations
            .iter()
            .map(|o| o.name.as_str())
            .collect::<Vec<_>>();

        let org_name = if self.org.is_some() {
            let arg_org = self.org.clone().unwrap();
            if org_names.contains(&arg_org.as_str()) {
                arg_org
            } else {
                return Err(anyhow!(
                    "organization {} does not exist or you do not have access to it",
                    arg_org
                ));
            }
        } else {
            let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Please select your organization: ")
                .items(&org_names[..])
                .interact()
                .unwrap();
            org_names[selection].to_string()
        };

        match UserConfig::set_default_org(org_name) {
            Ok(_) => Ok(()),
            Err(err) => {
                println!("Error while writing config: {}", err);
                Ok(())
            }
        }
    }
}
