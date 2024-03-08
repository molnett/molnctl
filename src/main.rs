use crate::config::user::UserConfig;
use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use commands::CommandBase;
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
    /// Manage organizations
    Orgs(commands::orgs::Orgs),
    /// Create and manage secrets
    Secrets(commands::secrets::Secrets),
    /// Deploy and manage services
    Services(commands::services::Services),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(config_path) = cli.config.as_deref() {
        println!("Config path: {}", config_path);
    }

    let mut config = UserConfig::new(&cli);
    let mut base = CommandBase::new(&mut config, cli.org);

    match cli.command {
        Some(Commands::Auth(auth)) => auth.execute(&mut base),
        Some(Commands::Environments(environments)) => environments.execute(&mut base),
        Some(Commands::Orgs(orgs)) => orgs.execute(&mut base),
        Some(Commands::Secrets(secrets)) => secrets.execute(&mut base),
        Some(Commands::Services(svcs)) => svcs.execute(&mut base),
        None => Ok(()),
    }
}
