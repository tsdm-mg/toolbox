use crate::cmd::AnalyzeArgs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

/// Moe stages.
///
/// Different stages holding different points.
#[derive(Debug, Serialize, Deserialize)]
enum Stage {
    /// Stage in each season.
    Season,

    /// One ending stage per year.
    Ending,
}

/// Reward to apply
///
/// Some special kinds of reward not listed here because they are mysterious.
#[derive(Debug, Serialize, Deserialize)]
struct Reward {
    /// Points ww.
    ww: i32,

    /// Points tsb.
    tsb: i32,

    /// Moe energy.
    energy: i32,

    /// Moe credit.
    credit: i32,
}

/// Describe how to calculate reward for all users according to their poll and registration state.
///
/// All policy is optional because different stages contains different counts of rounds.
/// A user will be considered "missing xxx round" if missed any thread in the given [Round] then
/// the reward decreased.
///
/// If user participation does not match all provided (not None) policies, a zero reward will be
/// applied.
///
/// The document on each field only shows current round status for each stage, the real applied
/// stage policy will be described in config file when running analyze. Do not take the doc too
/// serious.
#[derive(Debug, Serialize, Deserialize)]
struct RewardPolicy {
    /// Reward apply on users participated in all rounds.
    complete: Option<Reward>,

    /// Reward apply on users participated in one fewer round.
    ///
    /// Both in [Stage::Season] and [Stage::Ending].
    missing1: Option<Reward>,

    /// Reward apply on users participated in two fewer rounds.
    ///
    /// Both in [Stage::Season] and [Stage::Ending].
    missing2: Option<Reward>,

    /// Reward apply on users participated in tree fewer rounds.
    ///
    /// Only in [Stage::Ending].
    missing3: Option<Reward>,

    /// Reward apply on users participated in four fewer rounds.
    ///
    /// Only in [Stage::Ending].
    missing4: Option<Reward>,
}

/// Thread config.
#[derive(Debug, Serialize, Deserialize)]
struct Thread {
    /// Thread name, or call it id.
    name: String,

    /// File path.
    file: String,
}

/// Config definition for analyzing.
#[derive(Debug, Serialize, Deserialize)]
struct AnalyzeConfig {
    /// Specify the stage to analyzing.
    ///
    /// Different stage includes different rounds and rewards, causing different threads to analyze
    /// and different statistics change.
    stage: Stage,

    /// Apply what reward to each user participated in.
    reward_policy: RewardPolicy,

    /// Round definition.
    ///
    /// ```json
    /// {
    ///   "round_1": {
    ///     "thread_a": [
    ///       "json_file_1",
    ///       "json_file_2"
    ///     ]
    ///   }
    /// }
    /// ```
    ///
    /// Definition on each round.
    /// Calculate reward according to user fully participated in what rounds.
    ///
    /// A round consists of a series of `thread`. User are considered missing the round if any thread
    /// meets one or more following conditions:
    ///
    /// * Not posted in thread.
    /// * Not posted in thread with the correct format.
    ///
    /// String in the deepest vector are json file paths of a same thread.
    ///
    /// All thread data are serialized data in `Thread` format.
    ///
    /// Note that the value `String` is the path to the directory contains thread data, and the thread
    /// data is expected to be serialized [tm_api::thread] instance.
    round: HashMap<String, HashMap<String, String>>,

    /// Path to the file containing current statistics data in html format.
    current_data_path: String,
}

pub async fn run_analyze_command(args: AnalyzeArgs) -> Result<()> {
    let config_path = args.config;
    let data = fs::read_to_string(config_path)
        .await
        .context("when reading config file")?;
    let config: AnalyzeConfig = toml::from_str(data.as_str()).context("invalid config")?;

    println!("{config:#?}");
    Ok(())
}
