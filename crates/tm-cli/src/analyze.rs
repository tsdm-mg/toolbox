use crate::cmd::AnalyzeArgs;
use crate::utils::parallel_future;
use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tm_api::post::Post as PostModel;
use tm_api::thread::Thread as ThreadModel;
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

    /// Path to the file containing registration data.
    registration_path: String,
}

impl AnalyzeConfig {
    fn generate_thread_map(&self) -> Vec<ThreadFlag> {
        self.round
            .iter()
            .flat_map(|(round_name, thread_group)| {
                thread_group
                    .iter()
                    .map(|(thread_name, _)| ThreadFlag {
                        round: round_name.clone(),
                        name: thread_name.clone(),
                        flag: false,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// Struct stores thread info and flag state.
#[derive(Debug)]
struct ThreadFlag {
    /// Round name.
    round: String,

    /// Thread name.
    name: String,

    /// Stored flag.
    flag: bool,
}

/// Container of loaded thread data.
///
/// Use as flattened `AnalyzeConfig::round`.
#[derive(Debug)]
struct LoadedThread {
    /// Round name.
    round: String,

    /// Thread name describes usage.
    name: String,

    /// Original parsed thread data.
    thread: Vec<ThreadModel>,
}

impl LoadedThread {
    /// Find the post by post id.
    fn find_post(&self, pid: &str) -> Option<&PostModel> {
        for thread in self.thread.iter() {
            if let Some(v) = thread.post_list.iter().find(|x| x.id == pid) {
                return Some(v);
            } else {
                continue;
            }
        }
        None
    }
}

pub async fn run_analyze_command(args: AnalyzeArgs) -> Result<()> {
    let config_path = args.config;
    let data = fs::read_to_string(config_path)
        .await
        .context("when reading config file")?;
    let config: AnalyzeConfig = toml::from_str(data.as_str()).context("invalid config")?;
    println!("{config:#?}");

    let reg_data = load_thread_data_from_dir(config.registration_path.as_str())
        .await
        .context("failed to load registration data")?;

    let post_data = parallel_future(config.round.iter(), 2, |(round_name, round)| async move {
        let result = parallel_future(round.into_iter(), 4, |(name, thread_path)| {
            let round = round_name.clone();
            async move {
                let thread = load_thread_data_from_dir(thread_path.as_str())
                    .await
                    .with_context(|| format!("when loading thread data from {thread_path}"))?;
                Ok(LoadedThread {
                    round,
                    name: name.to_owned(),
                    thread,
                })
            }
        })
        .await;
        result
    })
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    println!(
        "loaded reg_data, post count {}",
        reg_data.iter().fold(0, |acc, x| acc + x.post_list.len())
    );
    println!(
        "loaded post_data, post count {}",
        post_data.iter().fold(0, |acc, x| acc
            + x.thread
                .iter()
                .fold(0, |acc2, x2| acc2 + x2.post_list.len()))
    );

    let link_selector = Selector::parse("a").expect("invalid link selector");

    for reg_page in reg_data.iter() {
        for reg in reg_page.post_list.iter() {
            if reg.floor == 1 {
                // Skip the first floor.
                continue;
            }
            let mut flag_map = config.generate_thread_map();

            let doc = Html::parse_fragment(reg.body.as_str());
            while let Some(node) = doc.select(&link_selector).next() {
                let link = node.value().attr("href");
                if link.is_none() {
                    continue;
                }
                let link_str = link.unwrap();
                if let Some(prev) = node.prev_sibling() {
                    let v = prev.value();
                    // Find prev text if is group.
                    if !v.is_text() {
                        continue;
                    }
                    let text = v.as_text().unwrap().to_string();

                    let target_thread = flag_map.iter().find(|x| text.contains(x.name.as_str()));
                    if target_thread.is_none() {
                        // invalid text node.
                        continue;
                    }

                    // Here we find the target thread.
                    // Now check the link.
                    //
                    // Link format:
                    // forum.php?mod=redirect&amp&amp;goto=findpost&amp;ptid=$THREAD_ID&amp;pid=$PID_NEED_TO_CAPTURE
                    //
                    // Capture the pid and check related post satisfy all the following conditions:
                    //
                    // 1. Exists in loaded thread.
                    // 2. Lives in the correct thread (not incorrect thread or round).
                    // 3. The author is the same with user currently checking.
                    //
                    // If so, set the related flag in flag_map to true.
                    //
                    // After all links are validated, check if all keys in flag_map is `true`:
                    //
                    // * If so, check passed.
                    // * If not, generate info about missing round.
                    unimplemented!()
                }
            }
        }
    }

    Ok(())
}

async fn load_thread_data_from_dir(path: &str) -> Result<Vec<ThreadModel>> {
    let mut dir = fs::read_dir(path)
        .await
        .with_context(|| format!("failed to read dir {path}"))?;

    let mut data = vec![];

    while let Some(entry) = dir.next_entry().await.context("failed to get next entry")? {
        let file_name = entry.file_name();
        if !file_name.to_string_lossy().ends_with(".json") {
            continue;
        }
        let mut p = PathBuf::from(path);
        p.push(file_name);

        let content = fs::read(p).await;
        let thread: ThreadModel =
            serde_json::from_slice(content?.as_slice()).context("invalid thread json data")?;
        data.push(thread);
    }

    Ok(data)
}
