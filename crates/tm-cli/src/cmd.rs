use crate::analyze::run_analyze_command;
use crate::parse::run_parse_command;
use crate::profile::run_profile_command;
use crate::thread::run_thread_command;
use anyhow::Result;
use clap::ArgAction;
use clap::{Args, Parser, Subcommand};
///////// Groups /////////

#[derive(Clone, Debug, Args)]
#[group(required = true, multiple = false)]
pub struct ProfileTargetGroups {
    #[arg(long = "name", help = "specify user by username")]
    pub name: Option<String>,

    #[arg(long = "uid", help = "specify user by uid")]
    pub uid: Option<String>,

    #[arg(
        long = "download-reg",
        help = "download all profiles in registration thread"
    )]
    pub download_reg: Option<String>,
}

///////// Args /////////

#[derive(Clone, Debug, Args)]
pub struct ThreadArgs {
    #[arg(short = 't', long = "tid", help = "Thread id to fetch")]
    pub tid: u32,

    #[arg(
        short = 'p',
        long = "page",
        help = "Page number to fetch, single number value",
        default_value = "1"
    )]
    pub page: u32,

    #[arg(
        short = 'a',
        long = "all",
        help = "All pages. Override -p/--page when presents",
        default_value = "false",
        action = ArgAction::SetTrue,
    )]
    pub all: Option<bool>,

    #[arg(
        short = 'o',
        long = "output",
        help = "Directory to save fetched content"
    )]
    pub output: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ParseArgs {
    #[arg(help = "File path to parse content")]
    pub file: String,
}

#[derive(Clone, Debug, Args)]
pub struct AnalyzeArgs {
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to the config file defining analyze configuration"
    )]
    pub config: String,

    #[arg(long = "csv", help = "Path to output csv format analyze result")]
    pub csv: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileArgs {
    #[command(flatten)]
    pub profile_target: ProfileTargetGroups,

    #[arg(
        short = 'o',
        long = "output",
        help = "Directory to save fetched content"
    )]
    pub output: Option<String>,
}

///////// Subcommand /////////

#[derive(Clone, Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    #[command(about = "fetch content in thread")]
    Thread(ThreadArgs),

    #[command(about = "parse data from file")]
    Parse(ParseArgs),

    #[command(about = "analyze and produce statistics data")]
    Analyze(AnalyzeArgs),

    #[command(about = "fetch user profile")]
    Profile(ProfileArgs),
}

/// Main entry of all subcommands.
pub async fn run_command_with_args(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Thread(thread_args) => run_thread_command(thread_args).await,
        Command::Parse(parse_args) => run_parse_command(parse_args).await,
        Command::Analyze(analyze_args) => run_analyze_command(analyze_args).await,
        Command::Profile(profile_args) => run_profile_command(profile_args).await,
    }
}
