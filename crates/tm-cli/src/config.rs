use crate::utils::{load_thread_data_from_dir, parallel_future};
use anyhow::Result;
use anyhow::{anyhow, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tm_api::post::{generate_find_post_link, Post as PostModel};
use tm_api::thread::Thread as ThreadModel;
use tm_bbcode::{bbcode_to_string, Color, Url, WebColor};

/// Describe the thread usage.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum ThreadType {
    Poll,
    PollResult,
}

/// Regex to match choice not voted.
static UNSELECTED_RE: OnceLock<Regex> = OnceLock::new();

/// Regex to match choice voted.
static SELECTED_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Debug)]
enum Choice {
    Selected(String),
    Unselected(String),
}

#[derive(Debug, Eq, PartialEq)]
enum ChoiceState {
    NotDetermined,
    Unselected,
    Selected,
}

/// Parse the choice in poll line.
///
/// Use this function to validate poll result.
///
/// Each line shall be in the format of choices with selected state of unselected state.
fn parse_choice(line: &str) -> Option<Choice> {
    if let Some(capture) =
        SELECTED_RE.get_or_init(|| Regex::new(r#"^<strong><font color="White"><font style="background-color:Orange">(?<character>[^@<]+)@(?<bangumi>.+)</font></font></strong>$"#).unwrap()).captures(line) {
        Some(Choice::Selected(format!("{}@{}", capture.name("character").unwrap().as_str().to_string(), capture.name("bangumi").unwrap().as_str().to_string())))
    } else if let Some(capture) = UNSELECTED_RE.get_or_init(|| Regex::new(r#"^(?<character>[^@<]+)@(?<bangumi>.+)$"#).unwrap()).captures(line) {
        Some(Choice::Unselected(format!("{}@{}", capture.name("character").unwrap().as_str().to_string(), capture.name("bangumi").unwrap().as_str().to_string())))
    } else {
        None
    }
}

/// Moe stages.
///
/// Different stages holding different points.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Stage {
    /// Stage in each season.
    Season,

    /// One ending stage per year.
    Ending,
}

/// Reward to apply
///
/// Some special kinds of reward not listed here because they are mysterious.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Reward {
    /// Points ww.
    pub(crate) ww: i32,

    /// Points tsb.
    pub(crate) tsb: i32,

    /// Moe energy.
    pub(crate) energy: i32,

    /// Moe credit.
    pub(crate) credit: i32,
}

impl Reward {
    /// Generate the reward text.
    fn generate_reward_text(&self) -> String {
        let ww = if self.ww > 0 {
            Some(format!("{}ww", self.ww))
        } else {
            None
        };

        let tsb = if self.tsb > 0 {
            Some(format!("{}tsb", self.tsb))
        } else {
            None
        };

        let energy = if self.energy > 0 {
            Some(format!("{}能量值", self.energy))
        } else {
            None
        };

        let credit = if self.credit > 0 {
            Some(format!("{}积分", self.credit))
        } else {
            None
        };

        let reward = [ww, tsb, energy, credit]
            .into_iter()
            .filter_map(|x| if x.is_some() { Some(x.unwrap()) } else { None })
            .collect::<Vec<_>>()
            .join(" + ");

        reward
    }
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
pub(crate) struct RewardPolicy {
    /// Reward apply on users participated in all rounds.
    #[serde(default)]
    pub(crate) complete: Reward,

    /// Reward apply on users participated in one fewer round.
    ///
    /// Both in [Stage::Season] and [Stage::Ending].
    #[serde(default)]
    pub(crate) missing1: Reward,

    /// Reward apply on users participated in two fewer rounds.
    ///
    /// Both in [Stage::Season] and [Stage::Ending].
    #[serde(default)]
    pub(crate) missing2: Reward,

    /// Reward apply on users participated in tree fewer rounds.
    ///
    /// Only in [Stage::Ending].
    #[serde(default)]
    pub(crate) missing3: Reward,

    /// Reward apply on users participated in four fewer rounds.
    ///
    /// Only in [Stage::Ending].
    #[serde(default)]
    pub(crate) missing4: Reward,
}

impl RewardPolicy {
    /// Generate reward description according to the count of missing rounds.
    pub(crate) fn generate_reward_text(&self, missing_rounds: usize) -> String {
        match missing_rounds {
            0 => self.complete.generate_reward_text(),
            1 => self.missing1.generate_reward_text(),
            2 => self.missing2.generate_reward_text(),
            3 => self.missing3.generate_reward_text(),
            4.. => self.missing4.generate_reward_text(),
        }
    }
}

/// Config definition for analyzing.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    /// Specify the stage to analyzing.
    ///
    /// Different stage includes different rounds and rewards, causing different threads to analyze
    /// and different statistics change.
    stage: Stage,

    /// Apply what reward to each user participated in.
    pub(crate) reward_policy: RewardPolicy,

    /// Stage is consists of a series of rounds.
    pub(crate) round: Vec<Round>,

    /// Path to the file containing registration data.
    pub(crate) registration_path: String,
}

impl Config {
    /// Load thread from config directory.
    ///
    /// Specify `thread_type` if only want to load a specified thread type.
    pub(crate) async fn load_thread_data(
        &self,
        thread_type: Option<ThreadType>,
    ) -> Result<Vec<LoadedThreadPage>> {
        let post_data = parallel_future(
            self.round.iter(),
            2,
            |Round {
                 name: round,
                 group: thread,
             }| async {
                let result = parallel_future(thread.into_iter(), 4, |thread_group| {
                    let round = round.clone();
                    let thread_type = thread_type.clone();

                    async move {
                        let thread_type = thread_type.clone();

                        // Here we got a map of thread.
                        // Assume we are in a group called "初赛", then the name is "初赛" and thread
                        // can be: { "A组": "path_to_dir", "B组": "path_to_dir", ... }
                        // To get the same result as `ThreadGroup::Single in the adjacent arm, join
                        // the group name "初赛" and thread name "A组" together: "初赛A组".

                        let mut result: Vec<LoadedThreadPage> = vec![];

                        for Thread {
                            name,
                            path,
                            thread_type: curr_thread_type,
                            ..
                        } in thread_group.thread.iter()
                        {
                            if thread_type.is_some()
                                && thread_type.as_ref().unwrap() != curr_thread_type
                            {
                                continue;
                            }

                            let thread = load_thread_data_from_dir(path.as_str())
                                .await
                                .with_context(|| format!("when loading thread data from {path}"))?;
                            if thread.is_empty() {
                                return Err(anyhow!("empty thread data parsed from file {}", path));
                            }
                            let mut all_thread_in_the_group = thread
                                .into_iter()
                                .map(|x| LoadedThreadPage {
                                    round: round.clone(),
                                    group: thread_group.name.clone(),
                                    name: name.clone(),
                                    page: x.page,
                                    tid: x.tid,
                                    thread: x.thread,
                                })
                                .collect::<Vec<_>>();
                            result.append(&mut all_thread_in_the_group);
                        }

                        Ok(result)
                    }
                })
                .await;
                result
            },
        )
        .await?
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

        Ok(post_data)
    }
}

/// Round definition.
///
/// ```json
/// [
///   {
///     name: "round 1",
///     thread: [
///     <ThreadGroup>
///     ]
///   }
///   {
///     name: "round 2",
///     thread: [
///     <ThreadGroup>
///     ]
///   }
/// ]
/// ```
///
/// Definition on each round.
/// Calculate reward according to user fully participated in what rounds.
///
/// A round consists of a series of [ThreadGroup]. User are considered missing the round if any thread
/// meets one or more following conditions:
///
/// * Not posted in thread.
/// * Not posted in thread with the correct format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Round {
    /// Human-readable name.
    pub(crate) name: String,

    /// Thread can be a list of mix of group and single.
    pub(crate) group: Vec<ThreadGroup>,
}

impl Round {
    /// Check if the current round is missed.
    ///
    /// A round is considered as missed if user missed any thread in it.
    pub(crate) fn is_missed(&self) -> bool {
        self.group.iter().any(|group| group.missed_info().is_some())
    }

    /// Produce a plain text result for missed rounds info.
    ///
    /// In this format: `missed 第一轮【A组】 结果`
    ///
    /// Return `Some(info)` if any or `None` if not.
    pub(crate) fn missed_info(&self, indent: usize) -> Option<String> {
        let all_missed_info = self
            .group
            .iter()
            .filter_map(|group| group.missed_info())
            .collect::<Vec<_>>();
        if all_missed_info.is_empty() {
            None
        } else {
            Some(format!(
                "{}missed {} {}",
                " ".repeat(indent),
                self.name,
                all_missed_info.join(" ")
            ))
        }
    }

    /// Generate round status in bbcode format.
    ///
    /// The code is a single line text that can be placed into a table data (aka `[td][/td]`).
    ///
    /// In format:
    ///
    /// `${round_index}. ${all group generated bbcode}`
    ///
    /// where the `group`s are space separated.
    ///
    /// e.g. `1. 初赛【A组；B组；C组；D组】 结果`
    pub(crate) fn generate_bbcode(&self, idx: usize) -> String {
        let content = self
            .group
            .iter()
            .map(|x| x.generate_bbcode())
            .collect::<Vec<_>>()
            .join(" ");

        format!("{idx}. {}", content)
    }
}

/// Collects a group of thread that belongs to the same kind.
///
/// Structure between round and thread.
///
/// Can contain be either:
///
/// * Grouped: A series of threads with a name.
/// * Single: Only one thread with `None` as  name.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ThreadGroup {
    /// Group name.
    pub name: Option<String>,

    /// Value is pairs of thread name and thread data path.
    ///
    /// e.g. 初赛: { A组, B组, C组, D组 } => `{ name: "初赛", thread: { "A组": "path_a", "B组": "path_b", } }`
    pub thread: Vec<Thread>,
}

impl ThreadGroup {
    pub fn new_group(name: String, thread: Vec<Thread>) -> Self {
        ThreadGroup {
            name: Some(name),
            thread,
        }
    }

    pub fn new_single(thread: Thread) -> Self {
        ThreadGroup {
            name: None,
            thread: vec![thread],
        }
    }

    /// Generate missed thread info
    ///
    /// Example info: `初赛【A组；B组】 结果`
    ///
    /// Return `Some(info)` if any missed or `None` if all not missed.
    pub fn missed_info(&self) -> Option<String> {
        match self.name.as_ref() {
            Some(group_name) => {
                let missed_thread = self
                    .thread
                    .iter()
                    .filter_map(|x| (x.state != Participation::Ok).then(|| x.name.to_owned()))
                    .collect::<Vec<_>>();
                if missed_thread.is_empty() {
                    None
                } else {
                    if group_name.is_empty() {
                        Some(missed_thread.join("；"))
                    } else {
                        Some(format!("{group_name}【{}】", missed_thread.join("；")))
                    }
                }
            }
            None => match self.thread[0].state {
                Participation::Ok => None,
                Participation::Missed | Participation::Invalid => {
                    Some(self.thread[0].name.to_owned())
                }
            },
        }
    }

    /// Generate group status in bbcode format.
    ///
    /// Format is decided by the group thread count.
    ///
    /// * If the group is a real group (with more than one thread and a name)
    ///   `${GROUP_NAME}【${all thread generated code}】`
    ///   where the thread generated code is joined by `；`
    ///   e.g. `初赛【A组；B组；C组；D组】`
    /// * If the group is actually a single thread (with only one thread and no name).
    ///   `${thread generated code}`
    ///   e.g. `结果`
    pub(crate) fn generate_bbcode(&self) -> String {
        let code_vec = self
            .thread
            .iter()
            .map(|x| x.generate_bbcode())
            .collect::<Vec<_>>();

        if let Some(name) = self.name.as_ref() {
            if name.is_empty() {
                code_vec.join("；")
            } else {
                format!("{}【{}】", name, code_vec.join("；"))
            }
        } else {
            code_vec.join("；")
        }
    }
}

/// Thread config.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Thread {
    /// Thread name, or call it id.
    pub name: String,

    /// File path.
    pub path: String,

    /// User participation in the current thread.
    ///
    /// Actually this field differs among users and not presented in config. But we need a struct
    /// to carry user participation status so keep it here.
    #[serde(default, skip_serializing)]
    pub state: Participation,

    /// Floor number of the user participation.
    #[serde(default, skip_serializing)]
    pub floor: usize,

    /// Post id.
    ///
    /// Record here to make a redirect link.
    #[serde(default, skip_serializing)]
    pub pid: String,

    /// Floors violate duplicate poll rule.
    #[serde(default, skip_serializing)]
    pub duplicate: Vec<usize>,

    /// All available choices in poll.
    ///
    /// As the poll result shall be in the given format, use this field to load allowed choice.
    ///
    /// The reason why each choice is a `Vec<String>` is to tolerant the correcting on choices if
    /// the choice is spelled wrong. Users are advised to update their poll result when those wrong
    /// choices are fixed, but what if user did not do that, when calculating result those polls
    /// shall be considered as valid.
    ///
    /// ```toml
    /// choices = [
    ///     [
    ///         "和泉纱雾@情色漫画老师",
    ///         "情色漫画老师@和泉纱雾",
    ///    ]
    /// ]
    /// ```
    #[serde(skip_serializing)]
    pub choices: Option<Vec<Vec<String>>>,

    /// Allowed max choice count.
    ///
    /// The selected choices count in this thread MUST no more than the value.
    #[serde(default, skip_serializing)]
    pub max_choice: Option<usize>,

    /// Thread type, or call it usage.
    pub thread_type: ThreadType,
}

impl Thread {
    /// Generate bbcode status for current thread.
    ///
    /// The thread can be generated into different format of bbcode according to participation state.
    ///
    /// * [Participation::Ok] `[url=${FLOOR_LINK}]${THREAD_NAME}#${FLOOR}[/url]`
    /// * [Participation::Missed] `[color=Gray]${THREAD_NAME}[/color]`
    /// * [Participation::Invalid] `[url=${FLOOR_LINK}][color=DarkRed]${THREAD_NAME}#${FLOOR}[/color][/url]`
    pub fn generate_bbcode(&self) -> String {
        match self.state {
            Participation::Ok => bbcode_to_string(&Url::new(
                generate_find_post_link(self.pid.as_str()),
                vec![Box::new(format!("{}#{}", self.name.as_str(), self.floor))],
            )),
            Participation::Missed => {
                // With color
                // bbcode_to_string(&Color::new(
                //     WebColor::Gray,
                //     vec![Box::new(self.name.clone())],
                // ))

                // Without color
                self.name.clone()
            }
            Participation::Invalid => bbcode_to_string(&Url::new(
                generate_find_post_link(self.pid.as_str()),
                vec![Box::new(Color::new(
                    WebColor::DarkRed,
                    vec![Box::new(format!("{}#{}", self.name.as_str(), self.floor))],
                ))],
            )),
        }
    }

    /// Validate the poll is in correct format or not.
    ///
    /// `poll_data` shall be the html post body data in poll floor.
    pub fn validate_poll_format(&self, poll_data: &str) -> bool {
        let choices = match &self.choices {
            Some(v) => v,
            None => return true,
        };
        let mut flag_map = choices
            .iter()
            .map(|x| (x, ChoiceState::NotDetermined))
            .collect::<HashMap<&Vec<String>, ChoiceState>>();

        for poll_line in poll_data.split("<br />") {
            match parse_choice(poll_line.trim()) {
                Some(Choice::Unselected(ch)) => {
                    match flag_map
                        .iter_mut()
                        .find(|(choices, _)| choices.contains(&ch))
                    {
                        None => {
                            println!("invalid poll: thread {} floor {} has incorrect unselected choice {}", self.name, self.floor, ch);
                            return false;
                        }
                        Some((_, state)) => {
                            if *state == ChoiceState::Selected || *state == ChoiceState::Unselected
                            {
                                println!("invalid poll: thread {} floor {} has multiple unselected choices on {}", self.name, self.floor, ch);
                                return false;
                            }

                            *state = ChoiceState::Unselected;
                        }
                    }
                }
                Some(Choice::Selected(ch)) => {
                    match flag_map
                        .iter_mut()
                        .find(|(choices, _)| choices.contains(&ch))
                    {
                        None => {
                            println!(
                                "invalid poll: thread {} floor {} has incorrect selected choice {}",
                                self.name, self.floor, ch
                            );
                            return false;
                        }
                        Some((_, state)) => {
                            if *state == ChoiceState::Selected || *state == ChoiceState::Unselected
                            {
                                println!("invalid poll: thread {} floor {} has multiple selected choices on {}", self.name, self.floor, ch);
                                return false;
                            }

                            *state = ChoiceState::Selected;
                        }
                    }
                }
                None => continue,
            }
        }

        let mut selected_count = 0;

        for (choice, choice_state) in flag_map {
            match choice_state {
                ChoiceState::NotDetermined => {
                    println!(
                        "invalid poll: thread {} floor {} didn't polled choice {:?}",
                        self.name, self.floor, choice
                    );
                    return false;
                }
                ChoiceState::Selected => selected_count += 1,
                ChoiceState::Unselected => continue,
            }
        }

        if selected_count <= 0 || selected_count > self.max_choice.unwrap() {
            println!(
                "invalid poll: thread {} floor {} selected {} choices which out of range",
                self.name, self.floor, selected_count
            );
            return false;
        }

        true
    }
}

/// Container of loaded thread data.
///
/// Use as flattened `AnalyzeConfig::round`.
///
/// Each loaded thread instance holds one page of post data for a thread in a round.
#[derive(Debug)]
pub(crate) struct LoadedThreadPage {
    /// Round name.
    pub round: String,

    /// Group name.
    pub group: Option<String>,

    /// Thread name describes usage.
    pub name: String,

    /// Thread id.
    pub tid: String,

    /// Page in thread.
    pub page: String,

    /// Original parsed thread data.
    pub thread: ThreadModel,
}

impl LoadedThreadPage {
    /// Find the post by the author's uid.
    ///
    /// Only find in target round and group to avoid evaluating result from incorrect threads.
    pub(crate) fn find_post(
        &self,
        round: &str,
        group: Option<&String>,
        name: &str,
        uid: &str,
    ) -> Option<&PostModel> {
        if self.round != round || self.group.as_ref() != group || self.name != name {
            return None;
        }

        self.thread.post_list.iter().find(|x| x.author_id == uid)
    }
}

/// Enum represent participate state.
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub(crate) enum Participation {
    /// Participated with the correct format.
    Ok,

    /// Missed a thread in round.
    #[default]
    Missed,

    /// Invalid participation.
    Invalid,
}
