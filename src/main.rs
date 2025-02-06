use crate::config::user::UserConfig;
use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use commands::CommandBase;
use dialoguer::console::style;
use reqwest::blocking::Client;
use semver::Version;
use serde_json::Value;

mod api;
mod commands;
mod config;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(
        global = true,
        short,
        long,
        value_name = "FILE",
        env("MOLNETT_CONFIG"),
        help = "config file, default is $HOME/.config/molnett/config.json"
    )]
    config: Option<Utf8PathBuf>,

    #[arg(
        global = true,
        long,
        env("MOLNETT_API_URL"),
        help = "Url of the Molnett API, default is https://api.molnett.org"
    )]
    url: Option<String>,

    #[arg(
        global = true,
        long,
        env("MOLNETT_ORG"),
        help = "Organization to use (overrides default in config)"
    )]
    org: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Login to Molnett
    Auth(commands::auth::Auth),
    /// Create and manage environments
    Environments(commands::environments::Environments),
    /// Deploy a service
    Deploy(commands::services::Deploy),
    /// Tail logs from a service
    Logs(commands::services::Logs),
    /// Generate Dockerfile and Molnett manifest
    Initialize(commands::services::Initialize),
    /// Manage organizations
    Orgs(commands::orgs::Orgs),
    /// Create and manage secrets
    Secrets(commands::secrets::Secrets),
    /// Deploy and manage services
    Services(commands::services::Services),
}

fn main() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    if let Ok(Some(new_version)) = check_newer_release(current_version) {
        let message = style(format!("A new version ({}) is available at https://github.com/molnett/molnctl - you are running {}\n", new_version, current_version));
        println!("{}", message.bold().green());
    }

    let cli = Cli::parse();

    if let Some(config_path) = cli.config.as_deref() {
        println!("Config path: {}", config_path);
    }

    let mut config = UserConfig::new(&cli);
    let base = CommandBase::new(&mut config, cli.org);

    match cli.command {
        Some(Commands::Auth(auth)) => auth.execute(base),
        Some(Commands::Environments(environments)) => environments.execute(base),
        Some(Commands::Deploy(deploy)) => deploy.execute(base),
        Some(Commands::Logs(logs)) => logs.execute(base),
        Some(Commands::Initialize(init)) => init.execute(base),
        Some(Commands::Orgs(orgs)) => orgs.execute(base),
        Some(Commands::Secrets(secrets)) => secrets.execute(base),
        Some(Commands::Services(svcs)) => svcs.execute(base),
        None => Ok(()),
    }
}

pub fn check_newer_release(current_version: &str) -> Result<Option<String>> {
    let client = Client::new();
    let url = "https://api.github.com/repos/molnett/molnctl/releases/latest";

    let response = client.get(url).header("User-Agent", "molnctl").send()?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch release info: HTTP {}",
            response.status()
        ));
    }

    let body: Value = response.json()?;
    let latest_version = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to parse latest version"))?
        .trim_start_matches('v');

    let current = Version::parse(current_version)?;
    let latest = Version::parse(latest_version)?;

    if latest > current {
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}
