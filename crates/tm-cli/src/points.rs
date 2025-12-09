use crate::cmd::PointsArgs;
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;
use tm_bbcode_macro::bbx;
use tm_bbcode_webcolor::WebColor;
use tokio::fs;

static POINTS_RECORD_RE: OnceLock<Regex> = OnceLock::new();

/// bbx!(
///     tr {
///         td { {72}, "ID", },
///         td { {64}, "萌战总积分", },
///         td { {64}, "发帖数量", },
///         td { {64}, "发帖积分", },
///         td { {64}, "特殊积分", },
///         td { {64}, "投票积分", },
///         td { {51}, "能量值", },
///     }
/// ).as_str(),
const TABLE_HEADER: &str = "[tr][td=72]ID[/td][td=64]萌战总积分[/td][td=64]发帖数量[/td][td=64]发帖积分[/td][td=64]特殊积分[/td][td=64]投票积分[/td][td=51]能量值[/td][/tr]";

type ChangesMap = HashMap<String, IncrementRecord>;

// TODO: Serialize and deserialize through bbcode semantics parsing.
// TODO: Increment thread_count and threads_points from record.
/// Record of a user points status.
///
/// Each record shall be parsed from raw bbcode text in the points statistics thread.
#[derive(Clone, Debug)]
struct PointsRecord {
    /// Username.
    ///
    /// Name, not the id.
    username: String,

    /// Total points count.
    ///
    /// Sum of [`threads_points`], [`special_points`] and [`poll_points`]
    points: i32,

    /// Threads posted.
    ///
    /// Not supported to increment yet.
    threads_count: i32,

    /// Points on threads posting.
    ///
    /// Not supported to increment yet.
    threads_points: i32,

    /// Points, the special one.
    special_points: i32,

    /// Points on polling.
    poll_points: i32,

    /// Energy value.
    energy: i32,

    /// Special reward for users reach 100 total points.
    ///
    /// Flag to records that.
    reach_100_points: bool,
}

impl PointsRecord {
    fn init_re() -> Regex {
        Regex::new(r#"\[tr]\[td](?<username>[^\[]+)\[/td]\[td](?<points>\d+)\[/td]\[td](?<threads_count>\d+)\[/td]\[td](?<threads_points>\d+)\[/td]\[td](?<special_points>\d+)\[/td]\[td](?<poll_points>\d+)\[/td]\[td](?<energy>\d+)\[/td]\[/tr]"#).unwrap()
    }

    /// Update total points.
    ///
    /// Also, for users reached 100 total points, set the special flag to `true`.
    fn update_points(&mut self) {
        let old_points = self.points;
        self.points = self.threads_points + self.special_points + self.poll_points;

        if old_points < 100 && self.points >= 100 {
            self.reach_100_points = true;
            println!(
                "{} reaches 100 total points (increased {})",
                self.username,
                self.points < old_points
            );
        }
    }

    /// Apply points change to user record.
    ///
    /// This step calculates the latest points for a given user specified by `change.username`.
    fn apply_change(&mut self, change: &IncrementRecord) {
        if self.username != change.username {
            return;
        }

        // TODO: Update thread count and thread points from record.

        self.special_points += change.special_points;
        self.poll_points += change.poll_points;
        self.energy += change.energy;
        self.threads_count += change.threads_count;
        self.threads_points += change.threads_points;

        self.update_points();
    }

    fn from_line(line: &str) -> Option<Self> {
        let cap = match POINTS_RECORD_RE.get_or_init(Self::init_re).captures(line) {
            Some(v) => v,
            None => return None,
        };

        let username = cap.name("username").unwrap().as_str().to_string();
        let points = cap.name("points").unwrap().as_str().parse::<i32>().unwrap();
        let threads_count = cap
            .name("threads_count")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap();
        let threads_points = cap
            .name("threads_points")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap();
        let special_points = cap
            .name("special_points")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap();
        let poll_points = cap
            .name("poll_points")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap();
        let energy = cap.name("energy").unwrap().as_str().parse::<i32>().unwrap();

        Some(Self {
            username,
            points,
            threads_count,
            threads_points,
            special_points,
            poll_points,
            energy,
            reach_100_points: false,
        })
    }

    fn to_line(&self) -> String {
        bbx!(
            tr {
                td { ("{}", self.username) },
                td { ("{}", self.points) },
                td { ("{}", self.threads_count) },
                td { ("{}", self.threads_points) },
                td { ("{}", self.special_points) },
                td { ("{}", self.poll_points) },
                td { ("{}", self.energy) },
            }
        )
    }
}

impl From<&IncrementRecord> for PointsRecord {
    fn from(value: &IncrementRecord) -> Self {
        Self {
            username: value.username.clone(),
            points: 0 + value.poll_points + value.special_points,
            threads_count: 0,
            threads_points: 0,
            poll_points: value.poll_points,
            special_points: value.special_points,
            energy: value.energy,
            reach_100_points: false,
        }
    }
}

// TODO: Record thread points and thread count.
/// Describe a record of points change on a user participated in.
///
/// Each record holds the participation status to finalize points changes.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct IncrementRecord {
    /// Name of the user.
    username: String,

    /// Energy value records in moe subreddit only.
    energy: i32,

    /// Special points.
    special_points: i32,

    /// Credits rewarded by participating in polling events.
    poll_points: i32,

    /// Count of published threads.
    threads_count: i32,

    /// Counts of points accumulated by publishing threads.
    threads_points: i32,
}

impl IncrementRecord {
    fn increase(&mut self, other: &IncrementRecord) {
        if self.username != other.username {
            return;
        }

        self.energy += other.energy;
        self.special_points += other.special_points;
        self.poll_points += other.poll_points;
        self.threads_count += other.threads_count;
        self.threads_points += other.threads_points;
    }
}

/// Record of extra points change on some users.
///
/// The record usually generated from workgroup rewards which may contain poll points and special
/// points.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ExtraRecord {
    /// Name of points kind.
    name: String,

    /// Value of the points.
    value: i32,

    /// Username on the changes.
    users: Vec<String>,
}

pub async fn run_points_command(args: PointsArgs) -> Result<()> {
    // Populate changes.

    let changes = populate_increment_record(args.changes)
        .await
        .context("failed to populate increment data")?;

    println!("changes count: {}", changes.len());

    let extra_changes = if let Some(extra_path) = args.extra_changes {
        populate_extra_record(extra_path)
            .await
            .context("failed to populate extra records")?
    } else {
        vec![]
    };

    if !extra_changes.is_empty() {
        println!("extra changes: {:#?}", extra_changes);
    }

    let mut user_changes = ChangesMap::new();

    for record in [changes.into_iter(), extra_changes.into_iter()]
        .into_iter()
        .flatten()
    {
        if let Some(v) = user_changes.get_mut(&record.username) {
            v.increase(&record);
        } else {
            user_changes.insert(record.username.clone(), record);
        }
    }

    // Load current status.
    let (mut workgroup_data, mut general_data) = load_current_statistics(args.current).await?;

    apply_changes(&mut workgroup_data, &mut general_data, &user_changes);

    println!("Workgroup users count: {}", workgroup_data.len());
    println!("General users count: {}", general_data.len());

    // println!("Workgroup users points after update: {workgroup_data:#?}");
    // println!("General users points after update: {general_data:#?}");

    workgroup_data.sort_by(|a, b| b.points.cmp(&a.points));
    general_data.sort_by(|a, b| b.points.cmp(&a.points));

    let bbcode_result = generate_bbcode_result(&workgroup_data, &general_data);

    println!("users reached 100 total points:");
    for user_record in workgroup_data.iter().filter(|x| x.reach_100_points) {
        // TODO: Use BBCode macro.
        println!("{}", user_record.username);
    }
    for user_record in general_data.iter().filter(|x| x.reach_100_points) {
        // TODO: Use BBCode macro.
        println!("{}", user_record.username);
    }
    println!();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(args.output)
        .context("failed to open output file")?;
    file.write(bbcode_result.as_bytes())?;
    println!("done");

    Ok(())
}

/// Populate changes from increment data.
///
/// The increment data in converted from statistics xlsx file.
/// Records in the that data are only expected to have poll points and energy, none special points.
async fn populate_increment_record(data_path: String) -> Result<Vec<IncrementRecord>> {
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .double_quote(true)
        .from_path(data_path)?;

    let energy_re = Regex::new(r#"(?<energy>\d+)能量值"#).expect("invalid points kind regex");
    let poll_points_re = Regex::new(r#"(?<points>\d+)积分"#).expect("invalid points kind regex");

    let mut records = vec![];

    // Skip the title row.
    for maybe_record in csv.records().into_iter() {
        let record = match maybe_record {
            Ok(v) => v,
            Err(e) => return Err(anyhow!("invalid increment record: {}", e)),
        };

        //   0, 1,  2,      3,  4,      5,
        // 楼层,ID,UID,参与范围,积分,宣传签名,备注,备注2（说明）,备注3（工具）
        let floor = record.get(0).unwrap();
        let username = record.get(1).unwrap().to_string();
        let points = record.get(4).unwrap();

        let energy_capture = energy_re.captures(points);
        let poll_points_capture = poll_points_re.captures(points);

        let energy = energy_capture.and_then(|x| {
            x.name("energy")
                .unwrap()
                .as_str()
                .to_string()
                .parse::<i32>()
                .ok()
        });
        let poll_points = poll_points_capture.and_then(|x| {
            x.name("points")
                .unwrap()
                .as_str()
                .to_string()
                .parse::<i32>()
                .ok()
        });

        if energy.is_none() && poll_points.is_none() {
            continue;
        }

        let record = IncrementRecord {
            username,
            energy: energy.unwrap_or_default(),
            special_points: 0,
            poll_points: poll_points.unwrap_or_default(),
            threads_points: 0,
            threads_count: 0,
        };

        records.push(record);
    }

    Ok(records)
}

/// Populate points changes from extra json data.
///
/// The data usually came from workgroup rewards.
async fn populate_extra_record(data_path: String) -> Result<Vec<IncrementRecord>> {
    let data = fs::read(data_path).await?;
    let extra_records: Vec<ExtraRecord> = serde_json::from_slice(data.as_slice())?;

    let records = extra_records
        .into_iter()
        .flat_map(|x| {
            x.users
                .into_iter()
                .map(|y| IncrementRecord {
                    username: y,
                    energy: 0,
                    special_points: if x.name == "特殊积分" { x.value } else { 0 },
                    poll_points: if x.name == "投票积分" { x.value } else { 0 },
                    threads_count: if x.name == "发帖数量" { x.value } else { 0 },
                    threads_points: if x.name == "发帖积分" { x.value } else { 0 },
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(records)
}

/// Load current statistics data from the file.
///
/// The file:
///
/// * Shall have the original bbcode holding all points status for all users.
/// * Shall group into two tables where the first one is points for moe workgroup users and the
///   later one is for general users.
async fn load_current_statistics(path: String) -> Result<(Vec<PointsRecord>, Vec<PointsRecord>)> {
    let content = fs::read_to_string(path)
        .await
        .context("failed to read current statistics data")?;

    let tables_data = content.split(TABLE_HEADER).collect::<Vec<&str>>();

    if tables_data.len() != 3 {
        bail!("invalid content statistics data: not all two points table found");
    }

    let workgroup_data = tables_data[1]
        .lines()
        .into_iter()
        .filter_map(|x| PointsRecord::from_line(x))
        .collect::<Vec<_>>();

    let general_data = tables_data[2]
        .lines()
        .into_iter()
        .filter_map(|x| PointsRecord::from_line(x))
        .collect::<Vec<_>>();

    Ok((workgroup_data, general_data))
}

/// Apply changes in `changes_map` on `workgroup_data` and `general_data`.
///
/// ## Parameters
///
/// * `workgroup_data`: Current points state for moe workgroup users.
/// * `general_data`: Current points state for general users those not in moe workgroup.
/// * `changes_map`: Hash map, key is username, value is all the changes need to apply on user.
///
/// ## Steps
///
/// This function iterate through the `changes_map`, for each user in map:
///
/// 1. If the user is in `workgroup_data`, add the changes on the same user record in
///   `workgroup_data`.
/// 2. If the user is in `general_data`, add the changes on the same user record in
///   `general_data`.
/// 3. If the user presents in neither `workgroup_data` nor `general_data`, then the user gained
///   moe rewards for the first time and append the data to the end of `general_data`. Perhaps we
///   can use extra configs to specify each new user is in workgroup or not, this step makes further
///   table sorting possible, but not implemented yet.
fn apply_changes(
    workgroup_data: &mut Vec<PointsRecord>,
    general_data: &mut Vec<PointsRecord>,
    changes_map: &ChangesMap,
) {
    for (username, change) in changes_map.iter() {
        if let Some(workgroup_record) = workgroup_data
            .iter_mut()
            .find(|x| x.username.as_str() == username)
        {
            workgroup_record.apply_change(change);
        } else if let Some(general_record) = general_data
            .iter_mut()
            .find(|x| x.username.as_str() == username)
        {
            general_record.apply_change(change);
        } else {
            general_data.push(change.into())
        }
    }
}

fn generate_bbcode_result(
    workgroup_data: &Vec<PointsRecord>,
    general_data: &Vec<PointsRecord>,
) -> String {
    let workgroup_lines = workgroup_data
        .iter()
        .map(|x| x.to_line())
        .collect::<Vec<_>>()
        .join("\n");
    let general_lines = general_data
        .iter()
        .map(|x| x.to_line())
        .collect::<Vec<_>>()
        .join("\n");

    // Line A is 0.3%
    let line_a = general_data[(general_data.len() as f64 * 0.003).ceil() as usize - 1].points;
    // Line B is 1%
    let line_b = general_data[(general_data.len() as f64 * 0.01).ceil() as usize - 1].points;

    let header = generate_header();

    bbx!(
        font {
            {"黑体"},
            color {
                {"#000000"},
                b { "特殊积分指特殊活动期间获得的萌战积分。" }
            },
        },
        "\n\n",
        b {
            color {
                {WebColor::Red},
                size {
                    {3},
                    "\n",
                    ("❤ 萌战版块荣耀成就B线：{}分\n", line_b),
                    ("❤ 萌战版块荣耀成就A线：{}分\n", line_a),
                }
            }
        },
        "\n",
        table {
            "\n",
            ("{}", header),
            "\n",
            workgroup_lines.as_str(),
            "\n",
        },
        "\n\n",
        table {
            "\n",
            ("{}", header),
            "\n",
            general_lines.as_str(),
            "\n",
        }
    )
}

/// Header row in points table.
fn generate_header() -> String {
    bbx!(
        tr {
            td { {72}, "ID", },
            td { {64}, "萌战总积分", },
            td { {64}, "发帖数量", },
            td { {64}, "发帖积分", },
            td { {64}, "特殊积分", },
            td { {64}, "投票积分", },
            td { {51}, "能量值", },
        },
    )
}
