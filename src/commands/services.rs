use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use difference::{Difference, Changeset};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use super::CommandBase;
use tabled::Table;

use crate::{config::{
    application::{Build, HttpService},
    scan::{scan_directory_for_type, ApplicationType},
}, api::types::Service};

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
            None => Ok(())
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
    List(List)
}

#[derive(Debug, Parser)]
pub struct Deploy {
    #[arg(help = "Path to molnett manifest")]
    manifest: String,
    #[arg(long, help = "Skip confirmation", default_missing_value("true"), default_value("false"), num_args(0..=1), require_equals(true))]
    no_confirm: Option<bool>,
    #[arg(long, help = "Organization to deploy to")]
    org: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    environment: String,
    service: Service
}

impl Deploy {
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

        let manifest = self.read_manifest()?;

        let env_exists = base.api_client().get_environments(token, &org_name)?.contains(&manifest.environment);
        if !env_exists {
            return Err(anyhow!("Environment {} does not exist", manifest.environment))
        }

        let response = base.api_client().get_service(
            token,
            &org_name,
            &manifest.environment,
            &manifest.service.name
        );

        let existing_svc = match response {
            Ok(svc) => svc,
            Err(e) => match e.status() {
                Some(reqwest::StatusCode::NOT_FOUND) => {
                    return self.create_new_service(base, token, &org_name)
                },
                Some(reqwest::StatusCode::UNAUTHORIZED) => {
                    return Err(anyhow!("Unauthorized, please login first"))
                },
                _ => {
                    return Err(anyhow!("Could not check whether service exists or not"))
                }
            }
        };

        if let Some(false) = self.no_confirm {
            if existing_svc == manifest.service {
                println!("no changes detected");
                return Ok(())
            }
            let existing_svc_yaml = serde_yaml::to_string(&existing_svc)?;
            let new_svc_yaml = serde_yaml::to_string(&manifest.service)?;
            self.render_diff(existing_svc_yaml, new_svc_yaml)?;
            let selection = self.user_confirmation();
            if selection == 0 {
                println!("Cancelling...");
                return Ok(())
            }
        }

        let result = base.api_client().deploy_service(token, &org_name, &manifest.environment, manifest.service)?;
        println!("Service {} deployed", result.name);
        Ok(())
    }

    fn create_new_service(&self, base: &CommandBase, token: &str, org_name: &str) -> Result<()> {
        let manifest = self.read_manifest()?;
        if let Some(false) = self.no_confirm {
            let new_svc_yaml = serde_yaml::to_string(&manifest.service)?;
            self.render_diff("".to_string(), new_svc_yaml)?;
        }

        let selection = self.user_confirmation();
        if selection == 0 {
            println!("Cancelling...");
            return Ok(())
        }

        let result = base.api_client().deploy_service(token, org_name, &manifest.environment, manifest.service)?;
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
        let mut t = term::stdout().unwrap();
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

    #[clap(short, long)]
    organization_id: Option<String>,

    #[clap(short, long)]
    cpus: Option<u8>,

    #[clap(short, long)]
    memory_mb: Option<usize>,
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

        println!("Creating service...");
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
    base: &'a CommandBase<'a>,

    app_name: String,
    organization_id: String,

    cpus: u8,
    memory_mb: usize,

    docker_file_path: String,

    application_type: ApplicationType,
}

impl InitPlanBuilder<'_> {
    pub fn new<'a>(base: &'a CommandBase<'a>) -> InitPlanBuilder<'a> {
        InitPlanBuilder {
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
            .get_organizations(self.base.user_config().get_token().unwrap())
            .unwrap();

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
    pub fn builder<'a>(base: &'a CommandBase<'a>) -> InitPlanBuilder<'a> {
        InitPlanBuilder::new(base)
    }
}

#[derive(Parser)]
#[derive(Debug)]
pub struct List {
    #[arg(long, help = "Organization to list the services of")]
    org: Option<String>,
    #[arg(long, help = "Environment to list the services of")]
    env: String,
}

impl List {
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

        let response = base.api_client().get_services(
            token,
            &org_name,
            &self.env
        )?;

        let table = Table::new(response.services).to_string();
        println!("{}", table);

        Ok(())
    }
}
