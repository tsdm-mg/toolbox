use crate::thread::run_thread_command;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};

///////// Args /////////

#[derive(Clone, Debug, Args)]
pub(crate) struct ThreadArgs {
    #[arg(help = "Thread id")]
    pub(crate) tid: u32,

    #[arg(help = "Page number", default_value = "1")]
    pub(crate) page: u32,
}

///////// Subcommand /////////

#[derive(Clone, Debug, Parser)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Command {
    Thread(ThreadArgs),
}

/// Main entry of all subcommands.
pub async fn run_command_with_args(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Thread(thread_args) => run_thread_command(thread_args).await,
    }
}
