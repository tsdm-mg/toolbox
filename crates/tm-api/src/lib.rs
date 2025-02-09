use anyhow::{bail, Result};
use flate2::bufread::{DeflateDecoder, GzDecoder};
use reqwest::header::CONTENT_ENCODING;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::num::NonZeroI32;

pub mod post;
pub mod profile;
pub mod thread;
mod utils;

#[derive(Clone, Debug, thiserror::Error)]
pub enum ApiError {
    /// Http request failed.
    ///
    /// * 0: url.
    /// * 1: status code.
    #[error("bad response code {1:?} in http request on: {0:?}")]
    WebRequestError(String, String),

    /// Server returned an error.
    ///
    /// Like 4xx, not 5xx, some incorrect value in the request.
    ///
    /// * 0: `status` field in reply.
    /// * 1: `message` the error message in reply.
    #[error("server responded an error: status={0:?}, message={1:?}")]
    ServerRespError(NonZeroI32, String),
}

/// Represents the common format of the response when server replied an error>
///
/// This response format is only intended to use in APIs that returned a message indicating some
/// expected error caused by request, which means is something like 4xx not 5xx, the server still
/// working but the request ended with error on some invalid request no matter what the root cause
/// is.
///
/// The status code is expected to always present and be a value of non-zero, indicating an error.
/// Other fields are optional as there is no guarantee on existence.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ErrorResponse {
    /// Status code considered to be non-zero, indicating error happened.
    status: NonZeroI32,

    /// Error message.
    message: Option<String>,

    /// Url field.
    url: Option<String>,
}

pub(crate) async fn decompress_response_to_string(resp: Response) -> Result<String> {
    let data = match resp.headers().get(CONTENT_ENCODING) {
        Some(v) => match v.to_str()? {
            "gzip" => {
                let b = resp.bytes().await?;
                let mut d = GzDecoder::new(b.iter().as_slice());
                let mut s = String::new();
                d.read_to_string(&mut s)?;
                s
            }
            "deflate" => {
                let b = resp.bytes().await?;
                let mut d = DeflateDecoder::new(b.iter().as_slice());
                let mut s = String::new();
                d.read_to_string(&mut s)?;
                s
            }
            v => bail!(format!("unsupported http response encoding format: {}", v)),
        },
        None => resp.text().await?,
    };

    Ok(data)
}
