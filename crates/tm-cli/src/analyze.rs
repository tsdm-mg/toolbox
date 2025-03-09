use crate::cmd::AnalyzeArgs;
use crate::config::{Config, LoadedThreadPage, Participation, RewardPolicy, Round, DUPLICATE_INFO};
use crate::utils::{load_thread_data_from_dir, ThreadPageData};
use anyhow::{bail, Context, Result};
use std::cmp::Ordering;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use tm_api::post::generate_find_post_link;
use tm_bbcode::{bbcode, bbcode_to_string, Color, Table, TableData, TableRow, Url, WebColor};
use tokio::fs;
use tracing::trace;

const TABLE_WIDTH_30: usize = 30;
const TABLE_WIDTH_110: usize = 110;

/// Participation status on a user.
#[derive(Debug)]
pub(crate) struct UserParticipation {
    /// User's username.
    pub username: String,

    /// User's uid.
    pub uid: String,

    /// Post id of floor in registration thread.
    pub reg_pid: String,

    /// Post floor number in registration thread.
    pub floor: usize,

    /// Pairs of round index and threads in the round.
    ///
    /// [Round]s in this field is expected in the sort of the ones from external data source, the
    /// sort shall not be rearranged otherwise may break group order.
    ///
    /// A `None` value indicates user has duplicate registraion on current floor.
    pub rounds: Option<Vec<Round>>,
}

impl UserParticipation {
    /// Count rounds that not completely participated in.
    pub(crate) fn count_missing_rounds(&self) -> usize {
        match &self.rounds {
            Some(v) => v.iter().filter(|x| x.is_missed()).count(),
            None => 100,
        }
    }

    /// Generate rounds info text.
    pub(crate) fn missed_info(&self, indent: usize) -> String {
        match &self.rounds {
            Some(v) => v
                .iter()
                .filter_map(|x| x.missed_info(indent))
                .collect::<Vec<_>>()
                .join("\n"),
            None => DUPLICATE_INFO.to_string(),
        }
    }

    /// Generate a single record.
    pub(crate) fn generate_csv_record(&self, reward_policy: &RewardPolicy) -> Vec<String> {
        let missed_count = self.count_missing_rounds();
        let pat = match missed_count {
            0 => "全过程",
            1 => "少一轮",
            2 => "少两轮",
            3 => "少三轮",
            4.. => "少四轮",
        };

        vec![
            self.floor.to_string(),
            self.username.clone(),
            self.uid.to_string(),
            pat.to_string(),
            reward_policy.generate_reward_text(missed_count),
            String::new(),
            self.missed_info(0).trim().to_string(),
        ]
    }

    pub(crate) fn generate_bbcode(&self) -> TableData {
        if self.rounds.is_none() {
            return TableData::no_size(vec![Box::new(Color::new(
                WebColor::DarkRed,
                vec![Box::new(DUPLICATE_INFO)],
            ))]);
        }

        let data = self
            .rounds
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(idx, x)| x.generate_bbcode(idx + 1))
            .collect::<Vec<_>>()
            .join("\n");
        TableData::no_size(vec![Box::new(data)])
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

    /// Return a vec of [UserParticipation]s that combine all user participation held by current
    /// instance and sort by floor in registration thread.
    fn combine_and_sort(&self) -> Vec<&UserParticipation> {
        let mut data = [
            self.complete.as_slice(),
            self.missing1.as_slice(),
            self.missing2.as_slice(),
            self.missing3.as_slice(),
            self.missing4.as_slice(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        data.sort_by(sort_user_participation_ref);
        data
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
                        "  {}({} #{})\n{}\n",
                        p.username,
                        p.uid,
                        p.floor,
                        p.missed_info(4)
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
                        "  {}({} #{})\n{}\n",
                        p.username,
                        p.uid,
                        p.floor,
                        p.missed_info(4)
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
                        "  {}({} #{})\n{}\n",
                        p.username,
                        p.uid,
                        p.floor,
                        p.missed_info(4)
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
                        "  {}({} #{})\n{}\n",
                        p.username,
                        p.uid,
                        p.floor,
                        p.missed_info(4)
                    )
                    .as_str(),
                );
            }
        }

        result
    }

    /// Generate csv format text result.
    ///
    /// ## Columns
    ///
    /// * Floor
    /// * ID
    /// * UID
    /// * Participation
    /// * Reward
    /// * Signature link **not implemented yet**
    /// * Tip **optional**
    fn generate_csv_result(&self, reward_policy: &RewardPolicy) -> Vec<Vec<String>> {
        self.combine_and_sort()
            .into_iter()
            .map(|x| x.generate_csv_record(reward_policy))
            .collect()
    }

    /// Build a bbcode table describing user participation status.
    fn generate_participation_table(&self) -> Table {
        let records = self.combine_and_sort();
        let mut table = vec![TableRow::new(vec![
            TableData::with_size(TABLE_WIDTH_30, bbcode!("楼层")),
            TableData::with_size(TABLE_WIDTH_110, bbcode!("ID")),
            TableData::no_size(bbcode!("参与情况")),
        ])];
        for p in records.iter() {
            let row = TableRow::new(vec![
                TableData::no_size(vec![Box::new(Url::new(
                    generate_find_post_link(p.reg_pid.as_str()),
                    vec![Box::new(format!("{}", p.floor))],
                ))]),
                TableData::no_size(vec![Box::new(p.username.clone())]),
                p.generate_bbcode(),
            ]);

            table.push(row);
        }

        Table::new(table)
    }
}

pub async fn run_analyze_command(args: AnalyzeArgs) -> Result<()> {
    let config_path = args.config;
    let data = fs::read_to_string(config_path)
        .await
        .context("when reading config file")?;
    let config: Config = toml::from_str(data.as_str()).context("invalid config")?;
    trace!("{config:#?}");

    let mut reg_data = load_thread_data_from_dir(config.registration_path.as_str())
        .await
        .context("failed to load registration data")?;

    // Filter floors need to skip.
    // Those floors are the ones in registration thread but used for other usage rather than sign
    // the registration.
    if let Some(floors_to_skip) = args.skip_reg_floors {
        reg_data.iter_mut().for_each(|page| {
            page.thread
                .post_list
                .retain_mut(|post| !floors_to_skip.contains(&post.floor))
        });
    };

    if reg_data.is_empty() {
        bail!("error: empty registration data")
    }

    let post_data = config
        .load_thread_data(None)
        .await
        .context("failed to load thread from config")?;

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

    trace!("producing participation result");
    let participation_result = produce_participation_result(reg_data, post_data, config.round);

    trace!("producing analyze result");
    let analyze_result = produce_analyze_result(participation_result);

    println!("{}", analyze_result.generate_text_result());

    if let Some(csv_path) = args.save_csv_path {
        println!("writing csv data to {csv_path}");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(csv_path.clone())?;
        let mut builder = csv::WriterBuilder::new()
            .double_quote(true)
            .from_writer(file);
        for csv_record in analyze_result.generate_csv_result(&config.reward_policy) {
            builder
                .write_record(csv_record.as_slice())
                .with_context(|| {
                    format!("failed to write csv record \"{csv_record:?}\" to {csv_path}")
                })?
        }
        builder
            .flush()
            .with_context(|| format!("failed to flush csv output file {csv_path}"))?;
        println!("csv data saved in {csv_path}");
    }

    if let Some(status_path) = args.save_status_path {
        println!("writing participation status data to {status_path}");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(status_path.clone())?;
        let table = analyze_result.generate_participation_table();

        let data = bbcode_to_string(&table);
        let mut writer = BufWriter::new(file);
        writer
            .write_all(data.as_bytes())
            .context("failed to save bbcode participation status")?;
        writer.flush()?;
        println!("bbcode participation status saved in {status_path}");
    }

    Ok(())
}

fn produce_participation_result(
    reg_data: Vec<ThreadPageData>,
    post_data: Vec<LoadedThreadPage>,
    flags_template: Vec<Round>,
) -> Vec<UserParticipation> {
    let mut analyze_result = Vec::with_capacity(reg_data.len());

    trace!("traversing registration data");
    for (reg_page_number, reg_page) in reg_data.iter().enumerate() {
        trace!("traversing registration data page={}", reg_page_number);
        // Each reg is a post in the registration thread, where one user registered.
        for reg in reg_page.thread.post_list.iter() {
            trace!(
                "checking registration data floor={}, user={}, uid={}",
                reg.floor,
                reg.author,
                reg.author_id
            );
            if reg.floor == 1 {
                trace!("skip the first floor");
                // Skip the first floor, it is the announcement.
                continue;
            }

            // Find the current registration's user again, try to find the first occurred one.
            // If the position is not same, then the current registration is a duplicate one.
            let maybe_another_reg = reg_data
                .iter()
                .find_map(|page| {
                    page.thread
                        .post_list
                        .iter()
                        .find(|post| post.author_id == reg.author_id)
                        .map(|p| p.floor)
                })
                .unwrap();
            if maybe_another_reg != reg.floor {
                // Found another in the registration floor has the save author, so current one
                // is duplicate.
                // Traverse finish, produce result for the user.
                analyze_result.push(UserParticipation {
                    username: reg.author.clone(),
                    uid: reg.author_id.clone(),
                    floor: reg.floor.clone(),
                    reg_pid: reg.id.clone(),
                    // A `None` value means duplicate floor.
                    rounds: None,
                });
                continue;
            }

            // Map to store check result.
            // A user is considered to completely participated in the stage only if all flags in
            // this map are set to true.
            let mut flags = flags_template.clone();

            for round in flags.iter_mut() {
                trace!("analyzing round={}", round.name);
                for group in round.group.iter_mut() {
                    for thread in group.thread.iter_mut() {
                        match post_data.iter().find_map(|x| {
                            x.find_post_not_first_floor(
                                round.name.as_str(),
                                group.name.as_ref(),
                                thread.name.as_str(),
                                reg.author_id.as_str(),
                            )
                        }) {
                            Some(post) => {
                                thread.pid = post.id.clone();
                                thread.floor = post.floor;
                                if thread.duplicate.contains(&post.floor) {
                                    // Duplicate floor, invalid.
                                    thread.state = Participation::Invalid;
                                } else if !thread.validate_poll_format(post.body.as_str()) {
                                    // Incorrect format, invalid.
                                    thread.state = Participation::Invalid;
                                } else {
                                    thread.state = Participation::Ok;
                                }
                            }
                            None => {
                                thread.state = Participation::Missed;
                            }
                        }
                        trace!(
                            "analyzing round={}, group={:?}, thread={}, flag={:?}",
                            round.name,
                            group.name,
                            thread.name,
                            thread.state
                        );
                    }
                }
            }

            // Traverse finish, produce result for the user.
            analyze_result.push(UserParticipation {
                username: reg.author.clone(),
                uid: reg.author_id.clone(),
                floor: reg.floor.clone(),
                reg_pid: reg.id.clone(),
                rounds: Some(flags),
            });
        }
    }

    analyze_result
}

fn sort_user_participation(lhs: &UserParticipation, rhs: &UserParticipation) -> Ordering {
    if lhs.floor < rhs.floor {
        Ordering::Less
    } else if lhs.floor > rhs.floor {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

fn sort_user_participation_ref(lhs: &&UserParticipation, rhs: &&UserParticipation) -> Ordering {
    if lhs.floor < rhs.floor {
        Ordering::Less
    } else if lhs.floor > rhs.floor {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
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

    analyze_result.complete.sort_by(sort_user_participation);
    analyze_result.missing1.sort_by(sort_user_participation);
    analyze_result.missing2.sort_by(sort_user_participation);
    analyze_result.missing3.sort_by(sort_user_participation);
    analyze_result.missing4.sort_by(sort_user_participation);

    analyze_result
}
