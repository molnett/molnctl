use super::CommandBase;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use difference::{Changeset, Difference};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use tabled::Table;

use crate::{
    api::types::Service,
    config::{
        application::{Build, HttpService},
        scan::{scan_directory_for_type, ApplicationType},
    },
};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Services {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Services {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        match &self.command {
            Some(Commands::Deploy(depl)) => depl.execute(base),
            Some(Commands::Initialize(init)) => init.execute(base),
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Delete(delete)) => delete.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Deploy a service
    #[command(arg_required_else_help = true)]
    Deploy(Deploy),
    /// Generate Dockerfile and Molnett manifest
    Initialize(Initialize),
    /// List services
    List(List),
    /// Delete a service
    Delete(Delete),
}

#[derive(Debug, Parser)]
pub struct Deploy {
    #[arg(help = "Path to molnett manifest")]
    manifest: String,
    #[arg(long, help = "Skip confirmation", default_missing_value("true"), default_value("false"), num_args(0..=1), require_equals(true))]
    no_confirm: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct Manifest {
    environment: String,
    service: Service,
}

impl Deploy {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let manifest = self.read_manifest()?;

        let env_exists = base
            .api_client()
            .get_environments(token, &org_name)?
            .contains(&manifest.environment);
        if !env_exists {
            return Err(anyhow!(
                "Environment {} does not exist",
                manifest.environment
            ));
        }

        let response = base.api_client().get_service(
            token,
            &org_name,
            &manifest.environment,
            &manifest.service.name,
        );

        let existing_svc = match response? {
            Some(svc) => svc,
            None => return self.create_new_service(base, token, &org_name),
        };

        if let Some(false) = self.no_confirm {
            if existing_svc == manifest.service {
                println!("no changes detected");
                return Ok(());
            }
            let existing_svc_yaml = serde_yaml::to_string(&existing_svc)?;
            let new_svc_yaml = serde_yaml::to_string(&manifest.service)?;
            self.render_diff(existing_svc_yaml, new_svc_yaml)?;
            let selection = self.user_confirmation();
            if selection == 0 {
                println!("Cancelling...");
                return Ok(());
            }
        }

        let result = base.api_client().deploy_service(
            token,
            &org_name,
            &manifest.environment,
            manifest.service,
        )?;
        println!("Service {} deployed", result.name);
        Ok(())
    }

    fn create_new_service(&self, base: &CommandBase, token: &str, org_name: &str) -> Result<()> {
        let manifest = self.read_manifest()?;
        if let Some(false) = self.no_confirm {
            let new_svc_yaml = serde_yaml::to_string(&manifest.service)?;
            self.render_diff("".to_string(), new_svc_yaml)?;

            let selection = self.user_confirmation();
            if selection == 0 {
                println!("Cancelling...");
                return Ok(());
            }
        }

        let result = base.api_client().deploy_service(
            token,
            org_name,
            &manifest.environment,
            manifest.service,
        )?;
        println!("Service {} deployed", result.name);
        Ok(())
    }

    fn read_manifest(&self) -> Result<Manifest> {
        let file_path = self.manifest.clone();
        let mut file_content = String::new();
        File::open(file_path)?.read_to_string(&mut file_content)?;
        let manifest = serde_yaml::from_str(&file_content)?;
        Ok(manifest)
    }

    fn user_confirmation(&self) -> usize {
        FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Do you want to apply the above changes?")
            .items(&["no", "yes"])
            .default(0)
            .interact()
            .unwrap()
    }

    fn render_diff(&self, a: String, b: String) -> Result<()> {
        let Changeset { diffs, .. } = Changeset::new(&a, &b, "\n");
        let mut t = match term::stdout() {
            Some(stdout) => stdout,
            None => {
                return Err(anyhow!(
                    "Could not render diff. Consider using --no-confirm"
                ))
            }
        };
        for i in 0..diffs.len() {
            match diffs[i] {
                Difference::Same(ref x) => {
                    t.reset().unwrap();
                    writeln!(t, " {}", x)?;
                }
                Difference::Add(ref x) => {
                    t.fg(term::color::GREEN).unwrap();
                    writeln!(t, "+{}", x)?;
                }
                Difference::Rem(ref x) => {
                    t.fg(term::color::RED).unwrap();
                    writeln!(t, "-{}", x)?;
                }
            }
        }
        t.reset().unwrap();
        t.flush().unwrap();
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[clap(aliases = ["init"])]
pub struct Initialize {
    #[clap(short, long)]
    app_name: Option<String>,
}

impl Initialize {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        let app_config = base.app_config()?;
        if app_config.name().is_some() {
            return Err(anyhow!("App already initialized"));
        }

        let _token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let init_plan = InitPlan::builder()
            .app_name(self.app_name.as_deref())
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

        println!("Creating service...");
        println!("{:#?}", app_config);

        Ok(())
    }
}

#[derive(Debug)]
struct InitPlan {
    app_name: String,

    docker_file_path: String,
    application_type: ApplicationType,
}

struct InitPlanBuilder {
    app_name: String,

    docker_file_path: String,

    application_type: ApplicationType,
}

impl InitPlanBuilder {
    pub fn new() -> InitPlanBuilder {
        InitPlanBuilder {
            app_name: "".to_string(),
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
            self
        } else {
            self.app_name = app_name.unwrap().to_string();
            self
        }
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
        Ok(())
    }

    pub fn build(self) -> Result<InitPlan> {
        self.verify()?;
        Ok(InitPlan {
            app_name: self.app_name,
            docker_file_path: self.docker_file_path,
            application_type: self.application_type,
        })
    }
}

impl InitPlan {
    pub fn builder() -> InitPlanBuilder {
        InitPlanBuilder::new()
    }
}

#[derive(Parser, Debug)]
pub struct List {
    #[arg(long, help = "Environment to list the services of")]
    env: String,
}

impl List {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let response = base
            .api_client()
            .get_services(token, &org_name, &self.env)?;

        let table = Table::new(response.services).to_string();
        println!("{}", table);

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct Delete {
    #[arg(help = "Name of the service")]
    name: String,
    #[arg(long, help = "Environment the service is in")]
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
            let prompt = format!("Org: {}, Environment: {}, Service: {}. Are you sure you want to delete this service?", org_name, self.env, self.name);
            FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&["no", "yes"])
                .default(0)
                .interact()
                .unwrap();
        }

        base.api_client()
            .delete_service(token, &org_name, &self.env, &self.name)?;

        println!("Service {} deleted", self.name);
        Ok(())
    }
}
