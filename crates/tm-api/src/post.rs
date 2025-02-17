use serde::{Deserialize, Serialize};
use tm_types::PlatformValue;

/// Post in thread.
///
/// Each post represents a floor in thread.
///
/// Not all information provided by upstream are included in this model.
///
/// ## TODO
///
/// * Rate log.
/// * Rate total statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    /// Post id.
    #[serde(rename = "pid")]
    pub id: String,

    /// Author name.
    pub author: String,

    /// Uid of `author`.
    #[serde(rename = "authorid")]
    pub author_id: String,

    /// Author avatar url.
    pub avatar: String,

    /// Title of author.
    ///
    /// HTML text.
    #[serde(rename = "authortitle")]
    pub author_title: String,

    /// Title of author.
    ///
    /// HTML text.
    #[serde(rename = "authorgid")]
    pub author_group_id: String,

    /// Nickname of author.
    pub author_nickname: String,

    /// Timestamp in second.
    pub timestamp: String,

    /// Optional post subject.
    ///
    /// TIPS: Seems only with the first floor.
    #[serde(rename = "subject")]
    pub title: Option<String>,

    /// Post body.
    ///
    /// HTML text.
    #[serde(rename = "message")]
    pub body: String,

    /// Is first floor or not.
    ///
    /// "1" for true, other for false.
    #[serde(rename = "first")]
    pub first_floor: String,

    /// Floor number.
    pub floor: u32,

    /// User platform.
    ///
    /// Use the wrapper type [PlatformValue] to hold the value and parse to `Platform` when needed.
    /// See [PlatformValue] for details.
    pub platform: PlatformValue,
}

/// Generate a find post link for post specified by post id `pid`.
pub fn generate_find_post_link(pid: impl AsRef<str>) -> String {
    format!("forum.php?mod=redirect&goto=findpost&pid={}", pid.as_ref())
}
