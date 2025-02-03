use serde::{Deserialize, Serialize};
use tracing::warn;

/// Forum base url.
pub const BASE_URL: &str = "https://tsdm39.com";

/// Platforms the content publisher currently using.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Mobile web UI.
    WebMobile,

    /// Not specialized.
    Unknown,

    /// Android platform
    ///
    /// Most commonly used.
    Android,

    /// iOS platform.
    Ios,
}

/// Wrapper type for platform field in post data.
///
/// Unfortunately the value can be either a number or string and may both exist in a single post
/// list responded by server. Serde will assume the field to a certain type once deserialized for
/// the first time forcing later posts holding the same type which conflicts with the
/// implementation.
///
/// Use this field to dynamically hold platform field values and parse them to known [Platform] type
/// when needed, this step had to be at runtime caused by the limitation mentioned above.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PlatformValue {
    IntValue(i32),
    StringValue(String),
}

impl PlatformValue {
    #[tracing::instrument]
    pub fn platform(&self) -> Platform {
        match self {
            PlatformValue::StringValue(v) => match v.as_str() {
                "-1" => Platform::WebMobile,
                "0" => Platform::Unknown,
                "1" => Platform::Android,
                "2" => Platform::Ios,
                v => {
                    warn!("unknown platform string value {v}, fallback to unknown");
                    Platform::Unknown
                }
            },
            PlatformValue::IntValue(v) => match v {
                -1 => Platform::WebMobile,
                0 => Platform::Unknown,
                1 => Platform::Android,
                2 => Platform::Ios,
                v => {
                    warn!("unknown platform i32 value {v}, fallback to unknown");
                    Platform::Unknown
                }
            },
        }
    }
}
