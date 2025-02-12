use crate::cmd::PointsArgs;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;
use tokio::fs;

static POINTS_RECORD_RE: OnceLock<Regex> = OnceLock::new();

const POINTS_TABLE_HEAD: &str = "[color=rgb(68, 68, 68)][backcolor=rgb(255, 255, 255)][font=Verdana, Helvetica, Arial, sans-serif][size=14px][color=Red][size=3]❤ 萌战版块荣耀成就B线：380分
❤ 萌战版块荣耀成就A线：443分
[/size][/color][/size][/font][/backcolor][/color]
[table=98%,rgb(255, 255, 255)]
[tr][td=72]ID[/td][td=64]萌战总积分[/td][td=64]发帖数量[/td][td=64]发帖积分[/td][td=64]特殊积分[/td][td=64]投票积分[/td][td=51]能量值[/td][/tr]";

const POINTS_TABLE_MID: &str = "[/table]

[table=98%,rgb(255, 255, 255)]
[tr][td=72]ID[/td][td=64]萌战总积分[/td][td=64]发帖数量[/td][td=64]发帖积分[/td][td=64]特殊积分[/td][td=64]投票积分[/td][td=51]能量值[/td][/tr]";

const POINTS_TABLE_TAIL: &str = "[/table]";

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
}

impl PointsRecord {
    fn init_re() -> Regex {
        Regex::new(r#"\[tr]\[td](?<username>[^\[]+)\[/td]\[td](?<points>\d+)\[/td]\[td](?<threads_count>\d+)\[/td]\[td](?<threads_points>\d+)\[/td]\[td](?<special_points>\d+)\[/td]\[td](?<poll_points>\d+)\[/td]\[td](?<energy>\d+)\[/td]\[/tr]"#).unwrap()
    }

    fn update_points(&mut self) {
        self.points = self.threads_points + self.special_points + self.poll_points;
    }

    fn apply_change(&mut self, change: &IncrementRecord) {
        if self.username != change.username {
            return;
        }

        // TODO: Update thread count and thread points from record.

        self.special_points += change.special_points;
        self.poll_points += change.poll_points;
        self.energy += change.energy;

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
        })
    }

    fn to_line(&self) -> String {
        format!("[tr][td]{}[/td][td]{}[/td][td]{}[/td][td]{}[/td][td]{}[/td][td]{}[/td][td]{}[/td][/tr]",
                self.username, self.points, self.threads_count, self.threads_points, self.special_points, self.poll_points, self.energy,
        )
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
}

impl IncrementRecord {
    fn increment(&mut self, other: &IncrementRecord) {
        if self.username != other.username {
            return;
        }

        self.energy += other.energy;
        self.special_points += other.special_points;
        self.poll_points += other.poll_points;
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

    let extra_changes = if let Some(extra_path) = args.extra_changes {
        populate_extra_record(extra_path)
            .await
            .context("failed to populate extra records")?
    } else {
        vec![]
    };

    let mut user_changes = ChangesMap::new();

    for record in [changes.into_iter(), extra_changes.into_iter()]
        .into_iter()
        .flatten()
    {
        if let Some(v) = user_changes.get_mut(&record.username) {
            v.increment(&record);
        } else {
            user_changes.insert(record.username.clone(), record);
        }
    }

    // Load current status.
    let mut general_data = load_current_statistics(args.general_data).await?;
    let mut workgroup_data = load_current_statistics(args.workgroup_data).await?;

    apply_changes(&mut general_data, &user_changes);
    apply_changes(&mut workgroup_data, &user_changes);

    println!("Workgroup users count: {}", workgroup_data.len());
    println!("General users count: {}", general_data.len());

    // println!("Workgroup users points after update: {workgroup_data:#?}");
    // println!("General users points after update: {general_data:#?}");

    let bbcode_result = generate_bbcode_result(&workgroup_data, &general_data);

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
        .double_quote(true)
        .from_path(data_path)?;

    let points_re = Regex::new(r#"(?<energy>\d+)能量值 \+ (?<points>\d+)积分"#)
        .expect("invalid points kind regex");

    let mut records = vec![];

    // Skip the title row.
    for maybe_record in csv.records().into_iter().skip(1) {
        let record = match maybe_record {
            Ok(v) => v,
            Err(e) => return Err(anyhow!("invalid increment record: {}", e)),
        };

        //   0, 1,  2,      3,  4,      5,
        // 楼层,ID,UID,参与范围,积分,宣传签名,备注,备注2（说明）,备注3（工具）
        let floor = record.get(0).unwrap();
        let username = record.get(1).unwrap().to_string();
        let points = record.get(4).unwrap();

        let capture = match points_re.captures(points) {
            Some(v) => v,
            None => continue,
        };

        let energy = capture
            .name("energy")
            .unwrap()
            .as_str()
            .to_string()
            .parse::<i32>()
            .with_context(|| format!("invalid energy value at floor {floor}"))?;
        let poll_points = capture
            .name("points")
            .unwrap()
            .as_str()
            .to_string()
            .parse::<i32>()
            .with_context(|| format!("invalid poll_points value at floor {floor}"))?;

        let record = IncrementRecord {
            username,
            energy,
            special_points: 0,
            poll_points,
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
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(records)
}

async fn load_current_statistics(path: String) -> Result<Vec<PointsRecord>> {
    let content = fs::read_to_string(path)
        .await
        .context("failed to read current statistics data")?;
    let records = content
        .lines()
        .into_iter()
        .filter_map(|x| PointsRecord::from_line(x))
        .collect::<Vec<_>>();
    Ok(records)
}

fn apply_changes(data: &mut Vec<PointsRecord>, changes_map: &ChangesMap) {
    for (username, change) in changes_map.iter() {
        if let Some(p) = data.iter_mut().find(|x| x.username.as_str() == username) {
            p.apply_change(change);
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

    [
        POINTS_TABLE_HEAD,
        workgroup_lines.as_str(),
        POINTS_TABLE_MID,
        general_lines.as_str(),
        POINTS_TABLE_TAIL,
    ]
    .join("\n")
}
