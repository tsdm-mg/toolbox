use crate::decompress_response_to_string;
use crate::utils::http_get;
use crate::ApiError::WebRequestError;
use anyhow::{bail, Context, Result};
use racros::AutoDebug;
use regex::Regex;
use reqwest::StatusCode;
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};
use std::collections::HashMap;
use tm_html::HtmlElementExt;
use tm_types::BASE_URL;
use tracing::{debug, trace};

/// User profile model for plain web page implementation.
///
/// The fields listed here are those visitable to anyone visiting user profile. Fields requires
/// user permission like ip address are excluded.
#[derive(Clone, AutoDebug)]
pub struct Profile {
    /// Url of user avatar.
    #[debug_debug_not_pretty]
    pub avatar: Option<String>,

    /// Username.
    pub username: String,

    /// User id.
    pub uid: String,

    /// User email is verified or not.
    pub email_verified: bool,

    /// User verified through video.
    pub video_verified: bool,

    /// User custom title.
    #[debug_debug_not_pretty]
    pub custom_title: Option<String>,
    /// User signature.
    ///
    /// Html format.
    pub signature: Option<String>,

    /// Friends count.
    #[debug_debug_not_pretty]
    pub friends_count: Option<String>,

    /// Year of birthday.
    #[debug_debug_not_pretty]
    pub birth_year: Option<String>,

    /// Month of birthday.
    #[debug_debug_not_pretty]
    pub birth_month: Option<String>,

    /// Day of birthday.
    #[debug_debug_not_pretty]
    pub birth_day: Option<String>,

    /// Start zodiac.
    #[debug_debug_not_pretty]
    pub zodiac: Option<String>,

    /// MSN number.
    ///
    /// Note that this field have no validator so it's common to have non-number characters.
    #[debug_debug_not_pretty]
    pub msn: Option<String>,

    /// Personal introduction.
    pub introduction: Option<String>,

    /// Custom nickname.
    #[debug_debug_not_pretty]
    pub nickname: Option<String>,

    /// User gender.
    #[debug_debug_not_pretty]
    pub gender: Option<String>,

    /// Place user came from.
    #[debug_debug_not_pretty]
    pub from_where: Option<String>,

    /// Optional qq number.
    #[debug_debug_not_pretty]
    pub qq: Option<String>,

    /// Count of days checked in.
    ///
    /// Always non-zero value.
    #[debug_debug_not_pretty]
    pub check_in_days_count: Option<usize>,

    /// Count of days checked in during this month.
    #[debug_debug_not_pretty]
    pub check_in_this_month_count: Option<String>,

    /// Time of last check in.
    #[debug_debug_not_pretty]
    pub check_in_recent_time: Option<String>,

    /// All coins got by checking in.
    #[debug_debug_not_pretty]
    pub check_in_all_coins: Option<String>,

    /// Coins got when last check in.
    #[debug_debug_not_pretty]
    pub check_in_last_time_coins: Option<String>,

    /// Level of check in.
    #[debug_debug_not_pretty]
    pub check_in_level: Option<String>,

    /// Next level of check in.
    #[debug_debug_not_pretty]
    pub check_in_next_level: Option<String>,

    /// Days till next check in level.
    #[debug_debug_not_pretty]
    pub check_in_next_level_days: Option<String>,

    /// Today checked in or not.
    #[debug_debug_not_pretty]
    pub check_in_today_status: Option<String>,

    /// Moderator group name.
    #[debug_debug_not_pretty]
    pub moderator_group: Option<String>,

    /// General user group.
    #[debug_debug_not_pretty]
    pub user_group: Option<String>,

    /// Total online time.
    #[debug_debug_not_pretty]
    pub online_time: Option<String>,

    /// Date time of registration.
    #[debug_debug_not_pretty]
    pub register_time: Option<String>,

    /// Time of last visit.
    ///
    /// Usually the latest time to tell a user online.
    #[debug_debug_not_pretty]
    pub last_visit_time: Option<String>,

    /// Time of active most recently.
    ///
    /// May be before `last_visit_time`.
    ///
    /// This field is not as clear as `last_visit_time` or `last_post_time`, between them in time.
    #[debug_debug_not_pretty]
    pub last_active_time: Option<String>,

    /// Time of last posting.
    ///
    /// Did some activity, posting, check in, etc.
    /// Be `None` if user hasn't posted anything.
    ///
    /// Before `last_visit_time` and `last_active_time`.
    #[debug_debug_not_pretty]
    pub last_post_time: Option<String>,

    /// Timezone.
    #[debug_debug_not_pretty]
    pub timezone: Option<String>,

    /// Credits map.
    pub credits: HashMap<String, String>,
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
        .find(
            Name("div")
                .and(Class("pbm"))
                .and(Class("bbda"))
                .and(Class("cl"))
                .descendant(Name("li")),
        )
        .filter_map(|x| x.parse_li_em_group(false))
        .collect::<HashMap<_, _>>();

    let birthday_re = Regex::new(r#"((?<y>\d+) 年)? ?((?<m>\d+) 月)? ?((?<d>\d+) 日)?"#)
        .expect("invalid birthday regex");

    let email_verified = basic_info_map
        .get("邮箱状态")
        .and_then(|x| Some(x == "已验证"))
        .unwrap_or(false);
    let video_verified = basic_info_map
        .get("视频认证")
        .and_then(|x| Some(x == "已验证"))
        .unwrap_or(false);
    let custom_title = basic_info_map
        .get("自定义头衔")
        .and_then(|x| Some(x.to_owned()));
    let signature = basic_info_map
        .get("个人签名")
        .and_then(|x| Some(x.to_owned()));
    let friends_count = basic_info_map
        .get("统计信息")
        .and_then(|x| Some(x.to_owned()));

    let (birth_year, birth_month, birth_day) =
        match birthday_re.captures(basic_info_map.get("生日").unwrap_or(&String::new())) {
            Some(m) => (
                Some(
                    m.name("y")
                        .and_then(|x| Some(x.as_str().to_string()))
                        .unwrap_or_default(),
                ),
                Some(
                    m.name("m")
                        .and_then(|x| Some(x.as_str().to_string()))
                        .unwrap_or_default(),
                ),
                Some(
                    m.name("d")
                        .and_then(|x| Some(x.as_str().to_string()))
                        .unwrap_or_default(),
                ),
            ),
            None => (None, None, None),
        };

    let zodiac = basic_info_map.get("星座").and_then(|x| Some(x.to_owned()));
    let msn = basic_info_map.get("MSN").and_then(|x| Some(x.to_owned()));
    let introduction = basic_info_map
        .get("自我介绍")
        .and_then(|x| Some(x.to_owned()));
    let nickname = basic_info_map.get("昵称").and_then(|x| Some(x.to_owned()));
    let gender = basic_info_map.get("性别").and_then(|x| Some(x.to_owned()));
    let from_where = basic_info_map.get("来自").and_then(|x| Some(x.to_owned()));
    let qq = basic_info_map.get("QQ").and_then(|x| Some(x.to_owned()));

    /* Check in data */

    let check_in_node = root_node
        .find(
            Class("pbm")
                .and(Class("mbm"))
                .and(Class("bbda"))
                .and(Class("c")),
        )
        .next();
    let mut check_in_iter = check_in_node.and_then(|x| Some(x.find(Name("p"))));
    // p:nth-child(2)
    let check_in_days_count = check_in_iter.as_mut().and_then(|x| {
        x.next()
            .and_then(|x| x.first_end_deep_text().parse::<usize>().ok())
    });
    // p:nth-child(3)
    let check_in_this_month_count = check_in_iter
        .as_mut()
        .and_then(|x| x.next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(4)
    let check_in_recent_time = check_in_iter
        .as_mut()
        .and_then(|x| x.next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(5) font:nth-child(1)
    let mut child5 = check_in_iter.as_mut().and_then(|x| x.next());
    let check_in_all_coins = child5
        .and_then(|x| x.find(Name("font")).next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(5) font:nth-child(2)
    let check_in_last_time_coins = child5
        .as_mut()
        .and_then(|x| x.next())
        .and_then(|y| y.find(Name("font")).skip(1).next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(6) font:nth-child(1)
    let mut child6 = check_in_iter.as_mut().and_then(|x| x.next());
    let check_in_level = child6
        .and_then(|x| x.find(Name("font")).next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(6) font:nth-child(2)
    let check_in_next_level = child6
        .as_mut()
        .and_then(|x| x.find(Name("font")).skip(1).next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(6) font:nth-child(3)
    let check_in_next_level_days = child6
        .as_mut()
        .and_then(|x| x.find(Name("font")).skip(2).next())
        .and_then(|x| Some(x.first_end_deep_text()));
    // p:nth-child(7)
    let check_in_today_status = check_in_iter
        .as_mut()
        .and_then(|x| x.next())
        .and_then(|x| Some(x.first_end_deep_text()));

    /* User groups */

    let user_groups = root_node
        .find(
            Name("div")
                .and(Class("pbm"))
                .and(Class("bbda"))
                .and(Class("cl")),
        )
        .last()
        .and_then(|x| {
            Some(
                x.find(Name("ul").child(Name("li")).descendant(Name("a")))
                    .map(|x| x.first_end_deep_text())
                    .collect::<Vec<_>>(),
            )
        });
    let (moderator_group, user_group) = match user_groups {
        Some(mut v) => match v.len() {
            0 => (None, None),
            1 => (None, Some(v.remove(0))),
            2.. => (Some(v.remove(0)), Some(v.remove(0))),
        },
        None => (None, None),
    };

    /* Activity status */

    let activity_node = root_node.find(Name("ul").and(Attr("id", "pbbs"))).next();
    let activity_info_map = activity_node
        .and_then(|x| {
            Some(
                x.find(Name("li"))
                    .into_selection()
                    .into_iter()
                    .map(|x| x.parse_li_em_group(false))
                    .filter_map(|x| x)
                    .collect::<HashMap<String, String>>(),
            )
        })
        .unwrap_or_default();

    let online_time = activity_info_map
        .get("在线时间")
        .and_then(|x| Some(x.to_owned()));
    let register_time = activity_info_map
        .get("注册时间")
        .and_then(|x| Some(x.to_owned()));
    let last_visit_time = activity_info_map
        .get("最后访问")
        .and_then(|x| Some(x.to_owned()));
    let last_active_time = activity_info_map
        .get("上次活动时间")
        .and_then(|x| Some(x.to_owned()));
    let last_post_time = activity_info_map
        .get("上次发表时间")
        .and_then(|x| Some(x.to_owned()));
    let timezone = activity_info_map
        .get("所在时区")
        .and_then(|x| Some(x.to_owned()));

    let credits = root_node
        .find(
            Name("div")
                .and(Attr("id", "psts"))
                .child(Name("ul"))
                .child(Name("li")),
        )
        .into_selection()
        .into_iter()
        .filter_map(|x| x.parse_li_em_group(false))
        .collect::<HashMap<_, _>>();

    let profile = Profile {
        avatar,
        uid: uid.unwrap(),
        username,
        email_verified,
        video_verified,
        custom_title,
        signature,
        friends_count,
        birth_year,
        birth_month,
        birth_day,
        zodiac,
        msn,
        introduction,
        nickname,
        gender,
        from_where,
        qq,
        check_in_days_count,
        check_in_this_month_count,
        check_in_recent_time,
        check_in_all_coins,
        check_in_last_time_coins,
        check_in_level,
        check_in_next_level,
        check_in_next_level_days,
        check_in_today_status,
        moderator_group,
        user_group,
        online_time,
        register_time,
        last_visit_time,
        last_active_time,
        last_post_time,
        timezone,
        credits,
    };

    Ok(profile)
}
