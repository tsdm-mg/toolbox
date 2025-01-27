use serde::{Deserialize, Serialize};
use tm_types::Platform;

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
    id: String,

    /// Author name.
    author: String,

    /// Uid of `author`.
    #[serde(rename = "authorid")]
    author_id: String,

    /// Author avatar url.
    avatar: String,

    /// Title of author.
    ///
    /// HTML text.
    #[serde(rename = "authortitle")]
    author_title: String,

    /// Title of author.
    ///
    /// HTML text.
    #[serde(rename = "authorgid")]
    author_group_id: String,

    /// Nickname of author.
    author_nickname: String,

    /// Timestamp in second.
    timestamp: String,

    /// Optional post subject.
    ///
    /// TIPS: Seems only with the first floor.
    #[serde(rename = "subject")]
    title: Option<String>,

    /// Post body.
    ///
    /// HTML text.
    #[serde(rename = "message")]
    body: String,

    /// Is first floor or not.
    ///
    /// "1" for true, other for false.
    #[serde(rename = "first")]
    first_floor: String,

    /// Floor number.
    floor: u32,

    /// User platform.
    platform: Platform,
}
