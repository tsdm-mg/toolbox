use crate::decompress_response_to_string;
use crate::utils::http_get;
use crate::ApiError::WebRequestError;
use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::StatusCode;
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};
// use scraper::selectable::Selectable;
// use scraper::Html;
use std::collections::HashMap;
use tm_html::HtmlElementExt;
use tm_types::BASE_URL;
use tracing::{debug, trace};

/// User profile model for plain web page implementation.
///
/// The fields listed here are those visitable to anyone visiting user profile. Fields requires
/// user permission like ip address are excluded.
#[derive(Clone, Debug)]
pub struct Profile {
    /// Url of user avatar.
    pub avatar: Option<String>,

    /// Username.
    pub username: String,

    /// User id.
    pub uid: String,

    // /// User email is verified or not.
    // pub email_verified: bool,

    // /// User verified through video.
    // pub video_verified: bool,

    // /// User custom title.
    // pub custom_title: Option<String>,
    /// User signature.
    ///
    /// Html format.
    pub signature: Option<String>,
    // /// Friends count.
    // pub friends_count: Option<String>,

    // /// Year of birthday.
    // pub birth_year: Option<String>,

    // /// Month of birthday.
    // pub birth_month: Option<String>,

    // /// Day of birthday.
    // pub birth_day: Option<String>,

    // /// Start zodiac.
    // pub zodiac: Option<String>,

    // /// MSN number.
    // ///
    // /// Note that this field have no validator so it's common to have non-number characters.
    // pub msn: Option<String>,

    // /// Personal introduction.
    // pub introduction: Option<String>,

    // /// Custom nickname.
    // pub nickname: Option<String>,

    // /// User gender.
    // pub gender: Option<String>,

    // /// Place user came from.
    // pub from_where: Option<String>,

    // /// Optional qq number.
    // pub qq: Option<String>,

    // /// Count of days checked in.
    // ///
    // /// Always non-zero value.
    // pub check_in_days_count: Option<usize>,

    // /// Count of days checked in during this month.
    // pub check_in_this_month_count: Option<String>,

    // /// Time of last check in.
    // pub check_in_recent_time: Option<String>,

    // /// All coins got by checking in.
    // pub check_in_all_coins: Option<String>,

    // /// Level of check in.
    // pub check_in_level: Option<String>,

    // /// Next level of check in.
    // pub check_in_next_level: Option<String>,

    // pub check_in_next_level_days: Option<String>,

    // /// Today checked in or not.
    // pub check_in_today_status: Option<String>,

    // /// Moderator group name.
    // pub moderator_group: Option<String>,

    // /// General user group.
    // pub user_group: String,

    /*     Activity status     */
    // /// Total online time.
    // pub online_time: Option<String>,

    // /// Date time of registration.
    // pub register_time: String,

    // /// Time of last visit.
    // ///
    // /// Usually the latest time to tell a user online.
    // pub last_visit_time: String,

    // /// Time of active most recently.
    // ///
    // /// May be before `last_visit_time`.
    // ///
    // /// This field is not as clear as `last_visit_time` or `last_post_time`, between them in time.
    // pub last_active_time: String,

    // /// Time of last posting.
    // ///
    // /// Did some activity, posting, check in, etc.
    // /// Be `None` if user hasn't posted anything.
    // ///
    // /// Before `last_visit_time` and `last_active_time`.
    // pub last_post_time: Option<String>,

    // /// Timezone.
    // pub timezone: String,

    // /// Credits map.
    // pub credits: HashMap<String, String>,
}

/// Fetch user profile info by user id.
pub async fn fetch_user_profile_by_id(uid: impl AsRef<str>) -> Result<Profile> {
    let target = format!("{}/home.php?mod=space&uid={}", BASE_URL, uid.as_ref());
    debug!("fetch user profile (by uid) on url {target}");
    let resp = http_get(target.as_str())
        .await
        .context("failed to get user profile by id")?;
    if resp.status() != StatusCode::OK {
        debug!("bad response status: {}", resp.status());
        return Err(WebRequestError(target, resp.status().to_string()).into());
    }
    let data = decompress_response_to_string(resp)
        .await
        .context("when parsing user profile data")?;

    trace!("document: {data:?}");

    parse_profile_data(data)
}

/// Fetch user profile by username.
pub async fn fetch_user_profile_by_name(username: impl AsRef<str>) -> Result<Profile> {
    let target = format!(
        "{}/home.php?mod=space&username={}",
        BASE_URL,
        username.as_ref()
    );
    debug!("fetch user profile (by name) on url {target}");
    let resp = http_get(target.as_str())
        .await
        .context("failed to get user profile by name")?;
    if resp.status() != StatusCode::OK {
        debug!("bad response status: {}", resp.status());
        return Err(WebRequestError(target, resp.status().to_string()).into());
    }
    let data = decompress_response_to_string(resp)
        .await
        .context("when parsing user profile data")?;

    trace!("document: {data:?}");

    parse_profile_data(data)
}

pub fn parse_profile_data<'a>(html: impl AsRef<str>) -> Result<Profile> {
    let doc = Document::from(html.as_ref());
    let root_node = match doc
        .find(Attr("id", "pprl").child(Class("bm").and(Class("bbda"))))
        .next()
    {
        Some(v) => v,
        None => bail!("root node not found"),
    };

    let avatar = match doc
        .find(
            Attr("id", "ct")
                .descendant(Class("hm"))
                .child(Name("p"))
                .child(Name("a"))
                .child(Name("img")),
        )
        .next()
    {
        Some(v) => v.image_url(),
        None => bail!("avatar node not found"),
    };

    let username = match root_node.find(Name("h2").and(Class("mbn"))).next() {
        Some(v) => match v.first_child_text() {
            Some(vv) => vv.trim().to_string(),
            None => bail!("username text not found"),
        },
        None => bail!("username not found"),
    };

    let uid_re = Regex::new(r#"\(UID: (?<uid>\d+)\)"#).expect("invalid uid regex");

    let uid = root_node
        .find(
            Name("h2")
                .and(Class("mbn"))
                .child(Name("span").and(Class("xw0"))),
        )
        .next()
        .and_then(|x| {
            x.first_child_text()
                .and_then(|x| match uid_re.captures(x.as_str()) {
                    Some(v) => Some(v.name("uid").unwrap().as_str().to_string()),
                    None => None,
                })
        });
    if uid.is_none() {
        bail!("uid not found");
    }

    let basic_info_map = root_node
        .find(Name("ul").and(Class("pbm")).child(Name("li")))
        .filter_map(|x| x.parse_li_em_group(false))
        .collect::<HashMap<_, _>>();

    let signature = basic_info_map
        .get("个人签名")
        .and_then(|x| Some(x.to_owned()));

    let profile = Profile {
        avatar,
        uid: uid.unwrap(),
        username,
        signature,
    };

    Ok(profile)
}
