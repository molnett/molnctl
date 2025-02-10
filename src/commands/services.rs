use super::CommandBase;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dialoguer::{FuzzySelect, Input};
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

use crate::api::types::{ComposeService, DisplayHashMap, DisplayOption, DisplayVec, Port, Service};
use crate::api::APIClient;
use difference::{Changeset, Difference};
use term;

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
    pub fn execute(self, base: CommandBase) -> Result<()> {
        match self.command {
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
    #[arg(long, help = "Environment to deploy to (new compose format only)")]
    env: Option<String>,
    #[arg(
        help = "Path to compose or manifest file",
        default_value("./molnett.yaml")
    )]
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
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let compose = read_manifest(&self.manifest)?;
        // TODO: remove this once we remove the old manifest format
        let environment = if let Some(env) = &self.env {
            env.clone()
        } else if let Ok(manifest) =
            serde_yaml::from_str::<Manifest>(&std::fs::read_to_string(&self.manifest)?)
        {
            manifest.environment
        } else {
            return Err(anyhow!("No environment specified"));
        };

        let env_exists = base
            .api_client()
            .get_environments(token, &org_name)?
            .environments
            .iter()
            .any(|env| env.name == environment);
        if !env_exists {
            return Err(anyhow!("Environment {} does not exist", environment));
        }

        if compose.services.is_empty() {
            return Err(anyhow!("No services found in compose file"));
        }

        for compose_service in compose.services.iter() {
            let service_name = &compose_service.name;
            println!("\nDeploying service: {}", service_name);

            // Convert compose service to our service type
            let service = Service {
                name: service_name.clone(),
                image: compose_service.image.clone(),
                container_port: compose_service
                    .ports
                    .first()
                    .map(|p| p.target)
                    .unwrap_or(80),
                env: compose_service.environment.clone(),
                secrets: compose_service.secrets.clone(),
                command: compose_service.command.clone(),
            };

            let response =
                base.api_client()
                    .get_service(token, &org_name, &environment, &service.name);

            if let Some(false) = self.no_confirm {
                let existing_svc_yaml = match response? {
                    Some(existing_service) => {
                        // Convert existing service to ComposeService for comparison
                        let existing_compose_service = ComposeService {
                            name: existing_service.name,
                            image: existing_service.image,
                            ports: vec![Port {
                                target: existing_service.container_port,
                                published: None,
                            }],
                            environment: existing_service.env.clone(),
                            secrets: existing_service.secrets.clone(),
                            command: existing_service.command.clone(),
                        };

                        if existing_compose_service == *compose_service {
                            println!("No changes detected for service {}", service_name);
                            continue;
                        }
                        serde_yaml::to_string(&existing_compose_service)?
                    }
                    None => String::new(),
                };
                let new_svc_yaml = serde_yaml::to_string(compose_service)?;
                self.render_diff(existing_svc_yaml, new_svc_yaml)?;
                let selection = self.user_confirmation();
                if selection == 0 {
                    println!("Skipping service {}", service_name);
                    continue;
                }
            }

            let result =
                base.api_client()
                    .deploy_service(token, &org_name, &environment, service)?;

            println!("Service {} deployed: {:?}", service_name, result);
        }

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
        for diff in &diffs {
            match diff {
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
    arg_required_else_help = false,
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
    pub fn execute(self, base: CommandBase) -> Result<()> {
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

        let compose = ComposeBuilder::new(token.to_string(), base.api_client(), base.get_org()?)
            .add_services()?
            .build();

        write_manifest(&self.manifest, &compose)?;

        println!("Wrote manifest to {}...", &self.manifest);
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ComposeFile {
    version: u16,
    services: Vec<ComposeService>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Value {
    String(String),
    SecretRef {
        #[serde(rename = "secretRef")]
        secret_ref: String,
    },
}

struct ComposeBuilder {
    token: String,
    api_client: APIClient,
    org_name: String,
    compose: ComposeFile,
}

impl ComposeBuilder {
    pub fn new(token: String, api_client: APIClient, org_name: String) -> Self {
        ComposeBuilder {
            token,
            api_client,
            org_name,
            compose: ComposeFile {
                version: 2,
                services: Vec::new(),
            },
        }
    }

    pub fn add_services(mut self) -> Result<Self> {
        let num_services: usize = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("How many services do you want to add?")
            .interact_text()?;

        for i in 0..num_services {
            println!("\nConfiguring service {} of {}", i + 1, num_services);
            let service_name: String =
                Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Please enter a name for your service")
                    .interact_text()?;

            let container_port: u16 =
                Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Please enter the port your container is listening on")
                    .interact_text()?;

            let image_name = get_image_name(&self.api_client, &self.token, &self.org_name, &None)?;
            let image_tag = get_image_tag(&None)?;
            let full_image = format!("{}:{}", image_name, image_tag);

            let mut service = ComposeService {
                name: service_name,
                image: full_image,
                ports: vec![Port {
                    target: container_port,
                    published: None,
                }],
                environment: DisplayOption(None),
                secrets: DisplayOption(None),
                command: DisplayOption(None),
            };

            // Ask for entrypoint
            let add_entrypoint =
                FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Do you want to specify an entrypoint?")
                    .items(&["no", "yes"])
                    .default(0)
                    .interact()?;

            if add_entrypoint == 1 {
                let mut entrypoint = Vec::new();
                loop {
                    let arg: String =
                        Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Enter entrypoint argument (empty to finish)")
                            .allow_empty(true)
                            .interact_text()?;

                    if arg.is_empty() {
                        break;
                    }

                    entrypoint.push(arg);
                }
                if !entrypoint.is_empty() {
                    service.command = DisplayOption(Some(DisplayVec(entrypoint)));
                }
            }

            // Ask for environment variables
            let add_env = FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Do you want to add environment variables?")
                .items(&["no", "yes"])
                .default(0)
                .interact()?;

            if add_env == 1 {
                let mut environment = IndexMap::new();
                loop {
                    let key: String =
                        Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Enter environment variable name (empty to finish)")
                            .allow_empty(true)
                            .interact_text()?;

                    if key.is_empty() {
                        break;
                    }

                    let is_secret =
                        FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Is this a secret reference?")
                            .items(&["no", "yes"])
                            .default(0)
                            .interact()?;

                    let value = if is_secret == 1 {
                        let secret_name: String =
                            Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                                .with_prompt("Enter secret name")
                                .interact_text()?;
                        secret_name
                    } else {
                        let value: String =
                            Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                                .with_prompt("Enter environment variable value")
                                .interact_text()?;
                        value
                    };

                    environment.insert(key, value);
                }
                if !environment.is_empty() {
                    service.environment = DisplayOption(Some(DisplayHashMap(environment)));
                }
            }

            self.compose.services.push(service);
        }

        Ok(self)
    }

    pub fn build(self) -> ComposeFile {
        self.compose
    }
}

fn get_image_name(
    api_client: &APIClient,
    token: &str,
    org_name: &str,
    name: &Option<String>,
) -> Result<String> {
    let image_name = if let Some(name) = name {
        name.clone()
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

    Ok(format!("register.molnett.org/{}/{}", org_id, image_name))
}

fn get_image_tag(tag: &Option<String>) -> Result<String> {
    if let Some(tag) = tag {
        Ok(tag.clone())
    } else {
        let git_output = Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("HEAD")
            .output()?;
        Ok(String::from_utf8_lossy(&git_output.stdout)
            .trim()
            .to_string())
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about, arg_required_else_help = true)]
pub struct ImageName {
    #[arg(help = "If updating a compose file, the service name to update")]
    service: Option<String>,
    #[arg(short, long, help = "Image tag to use")]
    tag: Option<String>,
    #[arg(
        short,
        long,
        help = "Path to a manifest file. Can be either a compose file (with service provided) or a molnett manifest"
    )]
    update_manifest: Option<String>,
    #[arg(short, long, help = "Override image name. Default is directory name")]
    image_name: Option<String>,
}

impl ImageName {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let image_name = get_image_name(
            &base.api_client(),
            token,
            &base.get_org()?,
            &self.image_name,
        )?;
        let image_tag = get_image_tag(&self.tag)?;
        let full_image = format!("{}:{}", image_name, image_tag);

        if let Some(path) = self.update_manifest.clone() {
            match &self.service {
                Some(service_name) => {
                    // Handle compose file format
                    let mut compose = read_manifest(&path)?;
                    let service = compose
                        .services
                        .iter_mut()
                        .find(|s| s.name == *service_name)
                        .ok_or_else(|| {
                            anyhow!("Service {} not found in compose file", service_name)
                        })?;
                    service.image = full_image.clone();
                    write_manifest(&path, &compose)?;
                }
                None => {
                    // Handle old manifest format
                    let mut manifest: Manifest =
                        serde_yaml::from_str(&std::fs::read_to_string(&path)?)?;
                    manifest.service.image = full_image.clone();
                    let mut file = File::create(&path)?;
                    serde_yaml::to_writer(&mut file, &manifest)?;
                }
            }
        }

        println!("{}", full_image);
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct List {
    #[arg(long, help = "Environment to list the services of")]
    env: String,
}

impl List {
    pub fn execute(self, base: CommandBase) -> Result<()> {
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
    pub fn execute(self, base: CommandBase) -> Result<()> {
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
    #[arg(help = "Environment to get logs from")]
    environment: String,
    #[arg(
        help = "Path to compose or manifest file",
        default_value("./compose.yaml")
    )]
    manifest: String,
}

impl Logs {
    pub fn execute(self, base: CommandBase) -> Result<()> {
        let org_name = base.get_org()?;
        let token = base
            .user_config()
            .get_token()
            .ok_or_else(|| anyhow!("No token found. Please login first."))?;

        let compose = read_manifest(&self.manifest)?;
        let service = compose
            .services
            .first()
            .ok_or_else(|| anyhow!("No services found in compose file"))?;

        let logurl: Uri = url::Url::parse(
            format!(
                "{}/orgs/{}/envs/{}/svcs/{}/logs",
                base.user_config().get_url().replace("http", "ws"),
                org_name,
                self.environment,
                service.name,
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

fn read_manifest(path: &str) -> Result<ComposeFile> {
    let mut file_content = String::new();
    File::open(&path)?.read_to_string(&mut file_content)?;

    // Try to parse as new compose format first
    match serde_yaml::from_str::<ComposeFile>(&file_content) {
        Ok(compose) => return Ok(compose),
        Err(e) => println!("Failed to parse as compose file: {}", e),
    }

    println!("Trying old manifest format");
    // If not a compose file, try to parse as our old manifest format
    let manifest: Manifest = serde_yaml::from_str(&file_content)
        .map_err(|e| anyhow!("Failed to parse manifest: {}", e))?;

    // Convert old manifest to new compose format
    let service = ComposeService {
        name: manifest.service.name,
        image: manifest.service.image,
        ports: vec![Port {
            target: manifest.service.container_port,
            published: None,
        }],
        environment: DisplayOption(manifest.service.env.0.map(|env_map| {
            let mut environment = IndexMap::new();
            for (k, v) in env_map.0 {
                environment.insert(k, v.clone());
            }
            DisplayHashMap(environment)
        })),
        secrets: manifest.service.secrets.clone(),
        command: manifest.service.command.clone(),
    };

    Ok(ComposeFile {
        version: 2,
        services: vec![service],
    })
}

fn write_manifest(path: &str, compose: &ComposeFile) -> Result<()> {
    let mut file = File::create(path)?;
    let yaml = serde_yaml::to_string(compose)?;
    file.write_all(yaml.as_bytes())?;
    Ok(())
}
