use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use crate::config::user::UserConfig;
use commands::{CommandBase, environments};
mod api;
mod commands;
mod config;


#[derive(Debug)]
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[arg(short, long, value_name = "FILE", env("MOLNETT_CONFIG"), help = "config file, default is $HOME/.config/molnett/config.json")]
    config: Option<Utf8PathBuf>,

    #[arg(long, env("MOLNETT_API_URL"), help = "Url of the Molnett API, default is https://api.molnett.org")]
    url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug)]
#[derive(Subcommand)]
enum Commands {
    /// Manage organizations
    Orgs(commands::orgs::Orgs),
    /// Login to Molnett
    Auth(commands::auth::Auth),
    /// Deploy and manage services
    Services(commands::services::Services),
    /// Create and manage environments
    Environments(commands::environments::Environments),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(config_path) = cli.config.as_deref() {
        println!("Config path: {}", config_path.to_string());
    }

    let mut config = UserConfig::new(&cli);
    let mut base = CommandBase::new(&mut config);

    match cli.command {
        Some(Commands::Services(svcs)) => svcs.execute(&mut base),
        Some(Commands::Auth(auth)) => auth.execute(&mut base),
        Some(Commands::Environments(environments)) => environments.execute(&mut base),
        Some(Commands::Orgs(orgs)) => orgs.execute(&mut base),
        None => Ok(()),
    }
}
