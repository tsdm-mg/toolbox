use crate::cmd::{run_command_with_args, Cli};
use clap::Parser;
use std::process::exit;
use tracing::trace;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod cmd;
mod parse;
mod thread;
mod utils;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::OFF.into())
        .with_env_var("TM_CLI_LOG")
        .from_env_lossy();
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global cli tracing subscriber");

    trace!("starting cli");

    let cli = Cli::parse();

    if let Err(e) = run_command_with_args(cli).await {
        eprintln!("failed to run command: {e:?}");
        exit(1)
    }
}
