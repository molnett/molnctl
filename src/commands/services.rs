use super::CommandBase;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
use difference::{Changeset, Difference};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tabled::Table;
use tungstenite::connect;
use tungstenite::http::Uri;
use tungstenite::ClientRequestBuilder;

use crate::api::types::{DisplayHashMap, DisplayOption, Service};
use crate::api::APIClient;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true,
    visible_alias = "svcs"
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
            Some(Commands::ImageName(image_name)) => image_name.execute(base),
            Some(Commands::List(list)) => list.execute(base),
            Some(Commands::Delete(delete)) => delete.execute(base),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Deploy a service
    Deploy(Deploy),
    /// Generate Dockerfile and Molnett manifest
    Initialize(Initialize),
    /// Get the image name that should used to push to the Molnett registry
    ImageName(ImageName),
    /// List services
    List(List),
    /// Delete a service
    Delete(Delete),
}

#[derive(Debug, Parser)]
pub struct Deploy {
    #[arg(help = "Path to molnett manifest", default_value("./molnett.yaml"))]
    manifest: String,
    #[arg(long, help = "Skip confirmation", default_missing_value("true"), default_value("false"), num_args(0..=1), require_equals(true))]
    no_confirm: Option<bool>,
}

#[derive(Deserialize, Debug, Serialize)]
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

        let manifest = read_manifest(&self.manifest)?;

        let env_exists = base
            .api_client()
            .get_environments(token, &org_name)?
            .environments
            .iter()
            .any(|env| env.name == manifest.environment);
        if !env_exists {
            return Err(anyhow!(
                "Environment {} does not exist",
                manifest.environment
            ));
        }

        if let Some(false) = self.no_confirm {
            let response = base.api_client().get_service(
                token,
                &org_name,
                &manifest.environment,
                &manifest.service.name,
            );

            let existing_svc_yaml = match response? {
                Some(svc) => {
                    if svc == manifest.service {
                        println!("no changes detected");
                        return Ok(());
                    }
                    serde_yaml::to_string(&svc)?
                }
                None => "".to_string(),
            };
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
        println!("Service deployed.");
        Ok(())
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
#[command(
    author,
    version,
    about,
    long_about,
    arg_required_else_help = true,
    visible_alias = "init"
)]
pub struct Initialize {
    #[arg(
        short,
        long,
        help = "Path to molnett manifest",
        default_value("./molnett.yaml")
    )]
    manifest: String,
}

impl Initialize {
    pub fn execute(&self, base: &mut CommandBase) -> Result<()> {
        let file_path = Path::new(&self.manifest);
        if file_path.exists() {
            let prompt = format!(
                "The file {} exists, do you want to overwrite it?",
                self.manifest
            );
            let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .items(&["no", "yes"])
                .default(0)
                .interact()
                .unwrap();
            if selection == 0 {
                println!("Cancelling...");
                return Ok(());
            }
        }

        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let manifest = ManifestBuilder::new(token.to_string(), base.api_client(), base.get_org()?)
            .get_env_name()?
            .get_service_name()?
            .get_port()?
            .get_image()?
            .build();

        write_manifest(&self.manifest, &manifest)?;

        println!("Wrote manifest to {}...", &self.manifest);
        Ok(())
    }
}

struct ManifestBuilder {
    token: String,
    api_client: APIClient,
    org_name: String,
    manifest: Manifest,
}

impl ManifestBuilder {
    pub fn new(token: String, api_client: APIClient, org_name: String) -> ManifestBuilder {
        ManifestBuilder {
            token,
            api_client,
            org_name,
            manifest: Manifest {
                environment: "".to_string(),
                service: Service {
                    name: "".to_string(),
                    image: "".to_string(),
                    container_port: 0,
                    env: DisplayOption(Some(DisplayHashMap(IndexMap::new()))),
                    secrets: DisplayOption(Some(DisplayHashMap(IndexMap::new()))),
                },
            },
        }
    }

    pub fn get_service_name(mut self) -> Result<Self> {
        self.manifest.service.name = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Please enter a name for your service: ")
            .interact_text()?;

        Ok(self)
    }

    pub fn get_env_name(mut self) -> Result<Self> {
        let envs = self
            .api_client
            .get_environments(&self.token, &self.org_name)?;

        let selection = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Please select the environment to deploy the service in: ")
            .items(
                &envs
                    .environments
                    .iter()
                    .map(|env| env.name.clone())
                    .collect::<Vec<String>>(),
            )
            .interact()?;
        self.manifest.environment = envs
            .environments
            .iter()
            .filter(|env| env.name == selection.to_string())
            .map(|env| env.name.clone())
            .collect::<Vec<String>>()[0]
            .clone();

        Ok(self)
    }

    pub fn get_port(mut self) -> Result<Self> {
        self.manifest.service.container_port =
            Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Please enter the port your container is listening on: ")
                .interact_text()?;

        Ok(self)
    }

    pub fn get_image(mut self) -> Result<Self> {
        self.manifest.service.image =
            get_image_name(&self.api_client, &self.token, &self.org_name, &None, &None)?;
        Ok(self)
    }

    pub fn build(self) -> Manifest {
        self.manifest
    }
}

fn get_image_name(
    api_client: &APIClient,
    token: &str,
    org_name: &str,
    tag: &Option<String>,
    name: &Option<String>,
) -> Result<String> {
    let image_name = if name.is_some() {
        name.as_ref().unwrap().to_string()
    } else {
        let cur_dir = env::current_dir()?;
        let image_name = if let Some(dir_name) = cur_dir.file_name() {
            dir_name.to_str().unwrap()
        } else {
            return Err(anyhow!("Unable to get current directory for image name"));
        };
        image_name.to_string()
    };
    let org_id = api_client.get_org(token, org_name)?.id;

    let image_tag = if tag.is_some() {
        tag.as_ref().unwrap().to_string()
    } else {
        let git_output = Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("HEAD")
            .output()?;
        String::from_utf8_lossy(&git_output.stdout).to_string()
    };

    return Ok(format!(
        "register.molnett.org/{}/{}:{}",
        org_id,
        image_name,
        image_tag.trim()
    ));
}

#[derive(Parser, Debug)]
pub struct ImageName {
    #[arg(short, long, help = "Image tag to use")]
    tag: Option<String>,
    #[arg(
        short,
        long,
        help = "Path to a molnett manifest. The manifest's image field will be updated to the returned image name"
    )]
    update_manifest: Option<String>,
    #[arg(short, long, help = "Override image name. Default is directory name")]
    image_name: Option<String>,
}

impl ImageName {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let image_name = get_image_name(
            &base.api_client(),
            token,
            &base.get_org()?,
            &self.tag,
            &self.image_name,
        )?;
        if let Some(path) = self.update_manifest.clone() {
            let mut manifest = read_manifest(&path)?;
            manifest.service.image = image_name.clone();
            write_manifest(&path, &manifest)?;
        }

        println!("{}", image_name);
        Ok(())
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

#[derive(Debug, Parser)]
pub struct Logs {
    #[arg(help = "Path to molnett manifest", default_value("./molnett.yaml"))]
    manifest: String,
}

impl Logs {
    pub fn execute(&self, base: &CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let manifest = read_manifest(&self.manifest)?;
        let logurl: Uri = url::Url::parse(
            format!(
                "{}/orgs/{}/envs/{}/svcs/{}/logs",
                base.user_config().get_url().replace("http", "ws"),
                org_name,
                manifest.environment,
                manifest.service.name,
            )
            .as_str(),
        )
        .unwrap()
        .as_str()
        .parse()
        .unwrap();

        let builder = ClientRequestBuilder::new(logurl)
            .with_header("Authorization", format!("Bearer {}", token.to_owned()));

        let (mut socket, _) = connect(builder).expect("Could not connect");

        loop {
            let msg = socket.read().expect("Error reading message");
            println!("{}", msg.to_string().trim_end());
        }
    }
}

fn read_manifest(path: &str) -> Result<Manifest> {
    let mut file_content = String::new();
    File::open(path)?.read_to_string(&mut file_content)?;
    let manifest = serde_yaml::from_str(&file_content)?;
    Ok(manifest)
}

fn write_manifest(path: &str, manifest: &Manifest) -> Result<()> {
    let mut file = File::create(path)?;
    let yaml = serde_yaml::to_string(manifest)?;
    file.write_all(yaml.as_bytes())?;
    Ok(())
}
