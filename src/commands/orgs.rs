use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use tabled::Table;

use super::CommandBase;

#[derive(Parser)]
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
    pub fn execute(&self) -> Result<()> {
        let base = CommandBase::new();

        match &self.command {
            Some(Commands::List(list)) => list.execute(&base),
            Some(Commands::Create(create)) => create.execute(&base),
            None => Ok(()),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    List(List),
    Create(Create),
}

#[derive(Parser)]
pub struct List {}

impl List {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let token = base
            .user_config()?
            .token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base.api_client()?.get_organizations(token)?;

        let table = Table::new(response.organizations).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Parser)]
pub struct Create {
    #[clap(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    billing_email: Option<String>,
}

impl Create {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let token = base
            .user_config()?
            .token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let mut plan = CreatePlan::builder(base)
            .name(self.name.as_deref())
            .billing_email(self.billing_email.as_deref())
            .build()?;

        let response = base.api_client()?.create_organization(
            token,
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

struct CreatePlanBuilder<'a> {
    base: &'a CommandBase,

    name: String,
    billing_email: String,
}

impl<'a> CreatePlanBuilder<'a> {
    pub fn new(base: &'a CommandBase) -> Self {
        Self {
            base,
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
            return self;
        } else {
            self.name = name.unwrap().to_string();
            return self;
        }
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
        return self;
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
    pub fn builder(base: &CommandBase) -> CreatePlanBuilder {
        CreatePlanBuilder::new(base)
    }
}
