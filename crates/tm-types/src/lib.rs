use serde_repr::{Deserialize_repr, Serialize_repr};

/// Forum base url.
pub const BASE_URL: &str = "https://tsdm39.com";

/// Platforms the content publisher currently using.
#[derive(Clone, Debug, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(i32)]
pub enum Platform {
    /// Mobile web UI.
    WebMobile = -1,

    /// Not specialized.
    Unknown = 0,

    /// Android platform
    ///
    /// Most commonly used.
    Android = 1,

    /// iOS platform.
    Ios = 2,
}
