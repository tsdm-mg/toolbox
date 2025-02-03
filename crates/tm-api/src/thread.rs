use crate::post::Post;
use crate::ApiError::{ServerRespError, WebRequestError};
use crate::{decompress_response_to_string, ErrorResponse};
use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tm_types::BASE_URL;
use tracing::{debug, trace};

/// Thread model
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thread {
    /// Thread subject.
    #[serde(rename = "subject")]
    pub title: String,

    /// Fetched post.
    #[serde(rename = "postlist")]
    pub post_list: Vec<Post>,

    /// All post count in the thread.
    #[serde(rename = "totalpost")]
    pub total_post: String,

    /// Post count in each page if page is fulfilled.
    #[serde(rename = "tpp")]
    pub post_per_page: String,

    /// ID of subreddit the thread currently in.
    #[serde(rename = "fid")]
    pub forum_id: String,

    /// Username of author.
    #[serde(rename = "thread_author")]
    pub author: String,

    /// UID of author.
    #[serde(rename = "thread_authorid")]
    pub author_id: u32,

    /// Current author is the moderator of current thread or not.
    #[serde(rename = "ismoderator")]
    pub moderator: u32,

    /// Price of the thread, if any.
    #[serde(rename = "thread_price")]
    pub price: String,

    /// User already purchased the thread or not.
    #[serde(rename = "thread_paid")]
    pub paid: u32,

    /// All user points name and id.
    ///
    /// Most points are not dynamic but parse it for safety.
    #[serde(rename = "extcreditsname")]
    pub points_map: HashMap<String, String>,
}

// TODO: Some steps in this function are common steps in all APIs, extract them when completed.
/// Fetch thread content from server.
#[tracing::instrument]
pub async fn fetch_thread_content(tid: u32, page: u32) -> Result<Thread> {
    let target =
        format!("{BASE_URL}/forum.php?mobile=yes&tsdmapp=1&mod=viewthread&tid={tid}&page={page}");
    debug!("fetch thread on url {target}");
    let resp = reqwest::get(target.as_str())
        .await
        .context("failed to get thread content")?;
    if resp.status() != StatusCode::OK {
        debug!("bad response status: {}", resp.status());
        return Err(WebRequestError(target, resp.status().to_string()).into());
    }
    let thread_data = decompress_response_to_string(resp)
        .await
        .context("when parsing thread data")?
        // Remove CR and LF
        // The CR is useless and remove it is safe.
        // The LF only follows "<br />" which is useless and can be safely removed, too.
        .replace(['\u{000A}', '\u{000D}'], "");

    // Check if error occurred.
    // Currently, we are checking error response by try to deserialize the data into pre-defined
    // format. It's expensive if the deserializing step does not early return, but it shall have.
    if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(thread_data.as_str()) {
        return Err(
            ServerRespError(error_resp.status, error_resp.message.unwrap_or_default()).into(),
        );
    }

    trace!("thread data: {thread_data}");

    let thread: Thread =
        serde_json::from_str(thread_data.as_str()).context("when deserializing thread data")?;

    Ok(thread)
}
