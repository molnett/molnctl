use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use crate::config::user::UserConfig;
use commands::CommandBase;
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
    #[arg(short, long, value_name = "FILE", env("MOLNETT_CONFIG"))]
    config: Option<Utf8PathBuf>,

    #[arg(long, env("MOLNETT_HOST"))]
    host: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug)]
#[derive(Subcommand)]
enum Commands {
    Orgs(commands::orgs::Orgs),
    Auth(commands::auth::Auth),
    Initialize(commands::initialize::Initialize),
    Test
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(config_path) = cli.config.as_deref() {
        println!("Value for config: {}", config_path.to_string());
    }

    println!("MyObject: {:?}", cli);

    // load or write default config file
    let config = UserConfig::new(&cli);
    println!("Config: {:?}", config);

    // overwrite some options if certain env variables are defined

    let mut base = CommandBase::new();

    match cli.command {
        Some(Commands::Orgs(orgs)) => orgs.execute(),
        Some(Commands::Auth(auth)) => auth.execute(&mut base),
        Some(Commands::Initialize(init)) => init.execute(&mut base),
        Some(Commands::Test) => Ok(()),
        None => Ok(()),
    }
}
