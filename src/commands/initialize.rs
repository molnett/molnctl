use std::path::Path;

use anyhow::{anyhow, Result};
use clap::Parser;
use dialoguer::{FuzzySelect, Input};

use crate::config::{
    application::{Build, HttpService},
    scan::{scan_directory_for_type, ApplicationType},
};

use super::CommandBase;

#[derive(Parser, Debug)]
#[command(author, version)]
#[clap(aliases = ["init"])]
pub struct Initialize {
    #[clap(short, long)]
    app_name: Option<String>,

    #[clap(short, long)]
    organization_id: Option<String>,

    #[clap(short, long)]
    cpus: Option<u8>,

    #[clap(short, long)]
    memory_mb: Option<usize>,
}

impl Initialize {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        base.user_config()?
            .token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let app_config = base.app_config()?;
        if app_config.name().is_some() {
            return Err(anyhow!("App already initialized"));
        }

        let init_plan = InitPlan::builder(base)
            .organization_id(self.organization_id.as_deref())
            .app_name(self.app_name.as_deref())
            .cpus(self.cpus)
            .memory_mb(self.memory_mb)
            .determine_docker_image_path()
            .determine_application_type()
            .build()?;

        let app_config = base.app_config_mut()?;

        app_config.set_name(init_plan.app_name)?;
        app_config.set_http_service(HttpService::new(8080, None))?;
        app_config.set_build_config(Build::new(Some(init_plan.docker_file_path), None))?;

        if let ApplicationType::Rust = init_plan.application_type {
            app_config.set_build_config(Build::new(Some("Dockerfile".to_string()), None))?;

            let dockerfile = Path::new("Dockerfile");
            if !dockerfile.exists() {
                std::fs::write(dockerfile, "")?;
                let template = Path::new("../config/templates/Dockerfile.rust");

                std::fs::copy(template, dockerfile)?;
            }
        }

        println!("Creating app...");
        println!("{:#?}", app_config);

        Ok(())
    }
}

#[derive(Debug)]
struct InitPlan {
    app_name: String,
    organization_id: String,

    cpus: u8,
    memory_mb: usize,

    docker_file_path: String,
    application_type: ApplicationType,
}

struct InitPlanBuilder<'a> {
    base: &'a CommandBase,

    app_name: String,
    organization_id: String,

    cpus: u8,
    memory_mb: usize,

    docker_file_path: String,

    application_type: ApplicationType,
}

impl<'a> InitPlanBuilder<'a> {
    pub fn new(base: &'a CommandBase) -> Self {
        Self {
            base,
            app_name: "".to_string(),
            organization_id: "".to_string(),
            cpus: 0,
            memory_mb: 0,
            docker_file_path: "".to_string(),
            application_type: ApplicationType::Unknown,
        }
    }

    pub fn app_name(mut self, app_name: Option<&str>) -> Self {
        if app_name.is_none() {
            let prompt = "Please enter a name for your app: ";
            let input: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .interact_text()
                .unwrap();

            self.app_name = input;
            return self;
        } else {
            self.app_name = app_name.unwrap().to_string();
            return self;
        }
    }

    pub fn organization_id(mut self, organization_id: Option<&str>) -> Self {
        if organization_id.is_some() {
            self.organization_id = organization_id.unwrap().to_string();
            return self;
        }

        let orgs = self
            .base
            .api_client()
            .unwrap()
            .get_organizations(self.base.user_config().unwrap().token().unwrap())
            .unwrap();

        let a = 0


        let org_ids = orgs
            .organizations
            .iter()
            .map(|o| o.name.as_str())
            .collect::<Vec<_>>();

        let prompt = "Please select your organization: ";
        let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .items(&org_ids[..])
            .interact()
            .unwrap();

        let input = orgs.organizations[selection].id.clone();

        self.organization_id = input;
        return self;
    }

    pub fn cpus(mut self, cpus: Option<u8>) -> Self {
        self.cpus = cpus.or_else(|| Some(1)).unwrap();
        self
    }

    pub fn memory_mb(mut self, memory_mb: Option<usize>) -> Self {
        self.memory_mb = memory_mb.or_else(|| Some(2048)).unwrap();
        self
    }

    pub fn determine_docker_image_path(mut self) -> Self {
        let docker_file_exists = Path::new("Dockerfile").exists();
        if docker_file_exists {
            self.docker_file_path = "Dockerfile".to_string();
            return self;
        }

        let prompt = "Could not find Dockerfile in current working directory. Please enter the relative path to your Dockerfile: ";
        let input: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .unwrap();

        self.docker_file_path = input;

        self
    }

    pub fn determine_application_type(mut self) -> Self {
        let application_type = scan_directory_for_type();

        self.application_type = application_type;
        self
    }

    fn verify(&self) -> Result<()> {
        let max_memory_limit = self.cpus as usize * 8192;
        let min_memory_limit = self.cpus as usize * 2048;

        if self.memory_mb > max_memory_limit {
            return Err(anyhow!(
                "Memory limit cannot be greater than 8192 MB per CPU",
            ));
        }
        if self.memory_mb < min_memory_limit {
            return Err(anyhow!(
                "Memory allocation cannot be less than 2048 MB per CPU",
            ));
        }

        Ok(())
    }

    pub fn build(self) -> Result<InitPlan> {
        self.verify()?;
        Ok(InitPlan {
            app_name: self.app_name,
            organization_id: self.organization_id,
            cpus: self.cpus,
            memory_mb: self.memory_mb,
            docker_file_path: self.docker_file_path,
            application_type: self.application_type,
        })
    }
}

impl InitPlan {
    pub fn builder(base: &CommandBase) -> InitPlanBuilder {
        InitPlanBuilder::new(base)
    }
}
