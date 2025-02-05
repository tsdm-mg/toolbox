use crate::cmd::AnalyzeArgs;
use crate::utils::parallel_future;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::Filter;
use std::path::PathBuf;
use std::slice::Iter;
use tm_api::post::Post as PostModel;
use tm_api::thread::Thread as ThreadModel;
use tokio::fs;
use tracing::{debug, trace, Instrument};

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
                        flag: Participation::Missed,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// Struct stores thread info and flag state.
#[derive(Clone, Debug)]
struct ThreadFlag {
    /// Round name.
    round: String,

    /// Thread name.
    name: String,

    /// Stored flag.
    flag: Participation,
}

/// Container of loaded thread data.
///
/// Use as flattened `AnalyzeConfig::round`.
///
/// Each loaded thread instance holds one page of post data for a thread in a round.
#[derive(Debug)]
struct LoadedThreadPage {
    /// Round name.
    round: String,

    /// Thread name describes usage.
    name: String,

    /// Thread id.
    tid: String,

    /// Page in thread.
    page: String,

    /// Original parsed thread data.
    thread: ThreadModel,
}

impl LoadedThreadPage {
    /// Find the post by post id `pid` in a specified thread where id is `tid`.
    fn find_post(&self, tid: &str, pid: &str) -> Option<&PostModel> {
        if self.tid != tid {
            return None;
        }

        self.thread.post_list.iter().find(|x| x.id == pid)
    }
}

/// Model for loading thread data from files.
///
/// Each instance holds one page of post data in a thread.
///
/// The data serialized from APIs can not tell threads' ids, those ids and page numbers are manually
/// saved in data file names by other data fetching components. Those files are expected to be in
/// "${THREAD_ID}_${PAGE_NUMBER}.json" format, thread id and page number shall be parsed and saved
/// when loading data otherwise we lose those info forever.
#[derive(Debug)]
struct ThreadPageData {
    /// Thread id.
    ///
    /// Parsed from data file name.
    tid: String,

    /// Page number.
    ///
    /// Parsed from data file name.
    page: String,

    /// Thread data.
    ///
    /// Deserialized from data file contents.
    thread: ThreadModel,
}

/// Enum represent participate state.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Participation {
    /// Participated with the correct format.
    Ok,

    /// Missed a thread in round.
    Missed,

    /// Incorrect registration.
    Incorrect,
}

/// Participation status on a user.
#[derive(Debug)]
struct UserParticipation {
    /// User's username.
    username: String,

    /// User's uid.
    uid: String,

    /// Post floor number in registration thread.
    floor: u32,

    /// Threads participated with the correct action.
    ok: HashMap<String, Vec<ThreadFlag>>,

    /// Threads missed.
    missed: HashMap<String, Vec<ThreadFlag>>,

    /// Threads user intend to register an incorrect post.
    incorrect: HashMap<String, Vec<ThreadFlag>>,
}

impl UserParticipation {
    fn count_missing_rounds(&self) -> usize {
        self.missed.len() + self.incorrect.len()
    }

    fn generate_rounds(&self, indent: usize) -> String {
        let mut result = String::new();
        for (round, threads) in self.missed.iter() {
            result.push_str(
                format!(
                    "{}missed {} {}\n",
                    " ".repeat(indent),
                    round,
                    threads
                        .iter()
                        .map(|x| x.name.to_owned())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
                .as_str(),
            );
        }
        for (round, threads) in self.incorrect.iter() {
            result.push_str(
                format!(
                    "{}incorrect {} {}\n",
                    " ".repeat(indent),
                    round,
                    threads
                        .iter()
                        .map(|x| x.name.to_owned())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
                .as_str(),
            );
        }
        result
    }
}

/// Produced result on user participation.
///
/// User participation result grouped by missing rounds count.
#[derive(Debug)]
struct AnalyzeResult {
    /// Users participated in all threads of all rounds.
    complete: Vec<UserParticipation>,

    /// Users participated in one fewer round
    missing1: Vec<UserParticipation>,

    /// Reward apply on users participated in two fewer rounds.
    missing2: Vec<UserParticipation>,

    /// Users participated in tree fewer rounds.
    missing3: Vec<UserParticipation>,

    /// Users participated in four fewer rounds.
    missing4: Vec<UserParticipation>,
}

impl AnalyzeResult {
    fn new() -> Self {
        AnalyzeResult {
            complete: vec![],
            missing1: vec![],
            missing2: vec![],
            missing3: vec![],
            missing4: vec![],
        }
    }
    fn generate_text_result(&self) -> String {
        let mut result = String::new();

        let complete_count = self.complete.len();
        let missing1_count = self.missing1.len();
        let missing2_count = self.missing2.len();
        let missing3_count = self.missing3.len();
        let missing4_count = self.missing4.len();

        result.push_str(
            format!(
                "Total users: {}\n",
                complete_count + missing1_count + missing2_count + missing3_count + missing4_count
            )
            .as_str(),
        );
        if complete_count > 0 {
            result.push_str(format!("Users complete all rounds: {complete_count}\n").as_str());
            for p in self.complete.iter() {
                result.push_str(format!("  {}({} #{})\n", p.username, p.uid, p.floor).as_str());
            }
        }

        if missing1_count > 0 {
            result.push_str(format!("Users missing 1 round: {missing1_count}\n").as_str());
            for p in self.missing1.iter() {
                result.push_str(
                    format!(
                        "  {}({} #{})\n{}",
                        p.username,
                        p.uid,
                        p.floor,
                        p.generate_rounds(4)
                    )
                    .as_str(),
                );
            }
        }

        if missing2_count > 0 {
            result.push_str(format!("Users missing 2 rounds: {missing2_count}\n").as_str());
            for p in self.missing2.iter() {
                result.push_str(
                    format!(
                        "  {}({} #{})\n{}",
                        p.username,
                        p.uid,
                        p.floor,
                        p.generate_rounds(4)
                    )
                    .as_str(),
                );
            }
        }

        if missing3_count > 0 {
            result.push_str(format!("Users missing 3 rounds: {missing3_count}\n").as_str());
            for p in self.missing3.iter() {
                result.push_str(
                    format!(
                        "  {}({} #{})\n{}",
                        p.username,
                        p.uid,
                        p.floor,
                        p.generate_rounds(4)
                    )
                    .as_str(),
                );
            }
        }

        if missing4_count > 0 {
            result.push_str(format!("Users missing 4 or more rounds: {missing4_count}\n").as_str());
            for p in self.missing4.iter() {
                result.push_str(
                    format!(
                        "  {}({} #{})\n{}",
                        p.username,
                        p.uid,
                        p.floor,
                        p.generate_rounds(4)
                    )
                    .as_str(),
                );
            }
        }

        result
    }
}

pub async fn run_analyze_command(args: AnalyzeArgs) -> Result<()> {
    let config_path = args.config;
    let data = fs::read_to_string(config_path)
        .await
        .context("when reading config file")?;
    let config: AnalyzeConfig = toml::from_str(data.as_str()).context("invalid config")?;
    trace!("{config:#?}");

    let reg_data = load_thread_data_from_dir(config.registration_path.as_str())
        .await
        .context("failed to load registration data")?;

    let post_data = parallel_future(config.round.iter(), 2, |(round_name, round)| async move {
        let result = parallel_future(round.into_iter(), 4, |(name, thread_path)| {
            let round = round_name.clone();
            async move {
                // Here the vec of thread data are expected to in the same thread but different
                // pages, one page one element.
                let thread = load_thread_data_from_dir(thread_path.as_str())
                    .await
                    .with_context(|| format!("when loading thread data from {thread_path}"))?;
                if thread.is_empty() {
                    return Err(anyhow!(
                        "empty thread data parsed from file {}",
                        thread_path
                    ));
                }

                let result = thread
                    .into_iter()
                    .map(|x| LoadedThreadPage {
                        round: round.clone(),
                        name: name.to_owned(),
                        page: x.page,
                        tid: x.tid,
                        thread: x.thread,
                    })
                    .collect::<Vec<_>>();

                Ok(result)
            }
        })
        .await;
        result
    })
    .await?
    .into_iter()
    .flatten()
    .flatten()
    .collect::<Vec<_>>();

    println!(
        "loaded reg_data, post count {}",
        reg_data
            .iter()
            .fold(0, |acc, x| acc + x.thread.post_list.len())
    );
    println!(
        "loaded post_data, post count {}",
        post_data
            .iter()
            .fold(0, |acc, x| acc + x.thread.post_list.len())
    );

    trace!("generating thread map template");
    let flag_template = config.generate_thread_map();

    trace!("producing participation result");
    let participation_result = produce_participation_result(reg_data, post_data, flag_template);

    trace!("producing analyze result");
    let analyze_result = produce_analyze_result(participation_result);

    println!("{}", analyze_result.generate_text_result());

    Ok(())
}

async fn load_thread_data_from_dir(path: &str) -> Result<Vec<ThreadPageData>> {
    let mut dir = fs::read_dir(path)
        .await
        .with_context(|| format!("failed to read dir {path}"))?;

    let mut data = vec![];

    // Regex to check data file name.
    // Each data file must contain one page of thread data for a thread and the file name should be
    // in "${THREAD_ID}_${PAGE_NUMBER}.json" format so that we can parse and save thread id and page
    // number as these data not exist in the response of server APIs means only the API caller know.
    let file_name_re = Regex::new(r#"(?<tid>\d+)_(?<page>\d+).json"#)
        .expect("invalid file name regex to validate data file names");

    let mut tid: Option<String> = None;

    while let Some(entry) = dir.next_entry().await.context("failed to get next entry")? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let capture = match file_name_re.captures(file_name.as_str()) {
            Some(v) => v,
            None => continue,
        };

        // tid and page are required to match the regex so it's safe to unwrap.
        let cap_tid = capture.name("tid").unwrap().as_str().to_string();
        // Check if all json files hold data for the same thread.
        //
        // Remember the thread id first met as it is the unique thread id intended to be in the
        // directory.
        if tid.is_none() {
            tid = Some(cap_tid.clone());
        } else if tid.as_ref().unwrap() != cap_tid.as_str() {
            return Err(anyhow!("invalid thread data storage: the directory {} is expected to only has thread {}, but also has {}. Did you mix two or more threads in that directory?", path, tid.unwrap(), cap_tid));
        }
        let page = capture.name("page").unwrap().as_str();
        let p: PathBuf = [path, file_name.as_str()].iter().collect();

        let content = fs::read(p).await;
        let thread: ThreadModel =
            serde_json::from_slice(content?.as_slice()).context("invalid thread json data")?;
        data.push(ThreadPageData {
            tid: cap_tid,
            page: page.to_string(),
            thread,
        });
    }

    Ok(data)
}

fn produce_participation_result(
    reg_data: Vec<ThreadPageData>,
    post_data: Vec<LoadedThreadPage>,
    flags_template: Vec<ThreadFlag>,
) -> Vec<UserParticipation> {
    let link_selector = Selector::parse("a").expect("invalid link selector");
    let link_re = Regex::new(r#"forum\.php\?mod=redirect((&amp;)|(&))+goto=findpost((&amp;)|(&))+ptid=(?<tid>\d+)((&amp;)|(&))+pid=(?<pid>\d+)"#).expect("invalid registration link format regex");

    let mut analyze_result = Vec::with_capacity(reg_data.len());

    trace!("traversing registration data");
    for (reg_page_number, reg_page) in reg_data.into_iter().enumerate() {
        trace!("traversing registration data page={}", reg_page_number);
        // Each reg is a post in the registration thread, where one user registered.
        for reg in reg_page.thread.post_list.into_iter() {
            trace!(
                "traversing registration data floor={}, user={}, uid={}",
                reg.floor,
                reg.author,
                reg.author_id
            );
            if reg.floor == 1 {
                trace!("skip the first floor");
                // Skip the first floor, it is the announcement.
                continue;
            }
            // Map to store check result.
            // A user is considered to completely participated in the stage only if all flags in
            // this map are set to true.
            let mut flags = flags_template.clone();

            // Traverse and check all `a` tags.
            let doc = Html::parse_fragment(reg.body.as_str());
            for node in doc.select(&link_selector) {
                let link = match node.value().attr("href") {
                    Some(v) => v,
                    None => continue,
                };
                trace!("visiting href node");

                // The thread name expected to be.
                // Note that the thread name may be longer than the one defined in analyze config,
                // but always contains that one.
                let thread_name = match node.prev_sibling() {
                    Some(v) if v.value().is_text() => v.value().as_text().unwrap().to_string(),
                    _ => continue,
                };
                trace!("validating thread_name={}", thread_name);

                // Expect text node.
                // Here we get the thread flag user intend to register for.
                let target_thread_flag = match flags
                    .iter_mut()
                    .find(|x| thread_name.contains(x.name.as_str()))
                {
                    Some(v) => v,
                    None => continue,
                };
                trace!("validating target_thread_flag={:?}", target_thread_flag);

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
                let capture = match link_re.captures(link) {
                    Some(v) => v,
                    None => {
                        trace!("link re not matched on link: {link}");
                        continue;
                    }
                };
                // tid and pid always required in regex so they be never none.
                let cap_tid = capture.name("tid").unwrap().as_str();
                let cap_pid = capture.name("pid").unwrap().as_str();
                trace!("capture link tid={}, pid={}", cap_tid, cap_pid);

                target_thread_flag.flag =
                    match post_data.iter().find_map(|x| x.find_post(cap_tid, cap_pid)) {
                        Some(v) => {
                            if v.author_id == reg.author_id {
                                Participation::Ok
                            } else {
                                trace!(
                                    "incorrect registration on author_id: expected: {}, got {}",
                                    reg.author_id,
                                    v.author_id
                                );
                                Participation::Incorrect
                            }
                        }
                        None => Participation::Missed,
                    };
                trace!("updating a target_thread_flag={:?}", target_thread_flag);
            }
            let x: Filter<Iter<'_, ThreadFlag>, _> =
                flags.iter().filter(|x| x.flag == Participation::Ok);

            // Traverse finish, produce result for the user.
            analyze_result.push(UserParticipation {
                username: reg.author,
                uid: reg.author_id,
                floor: reg.floor,
                ok: group_thread_flag_by_round(
                    flags.iter().filter(|x| x.flag == Participation::Ok),
                ),
                missed: group_thread_flag_by_round(
                    flags.iter().filter(|x| x.flag == Participation::Missed),
                ),
                incorrect: group_thread_flag_by_round(
                    flags.iter().filter(|x| x.flag == Participation::Incorrect),
                ),
            });
        }
    }

    analyze_result
}

fn produce_analyze_result(user_participation: Vec<UserParticipation>) -> AnalyzeResult {
    let mut analyze_result = AnalyzeResult::new();

    for p in user_participation.into_iter() {
        match p.count_missing_rounds() {
            0 => analyze_result.complete.push(p),
            1 => analyze_result.missing1.push(p),
            2 => analyze_result.missing2.push(p),
            3 => analyze_result.missing3.push(p),
            4.. => analyze_result.missing4.push(p),
        }
    }

    analyze_result
}

fn group_thread_flag_by_round(
    filter: Filter<Iter<'_, ThreadFlag>, fn(&&ThreadFlag) -> bool>,
) -> HashMap<String, Vec<ThreadFlag>> {
    let mut map = HashMap::<String, Vec<ThreadFlag>>::new();
    for element in filter {
        if let Some(v) = map.get_mut(element.round.as_str()) {
            (*v).push(element.to_owned());
        } else {
            map.insert(element.round.clone(), vec![element.to_owned()]);
        }
    }
    map
}
