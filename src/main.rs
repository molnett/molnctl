use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::CommandBase;
mod api;
mod commands;
mod config;

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
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Orgs(commands::orgs::Orgs),
    Auth(commands::auth::Auth),
    Initialize(commands::initialize::Initialize),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut base = CommandBase::new();

    match &cli.command {
        Some(Commands::Orgs(orgs)) => orgs.execute(),
        Some(Commands::Auth(auth)) => auth.execute(&mut base),
        Some(Commands::Initialize(init)) => init.execute(&mut base),
        None => Ok(()),
    }
}
