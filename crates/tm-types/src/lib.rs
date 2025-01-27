use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Formatter;

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

struct MultilineStringVisitor;

impl<'de> Visitor<'de> for MultilineStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("expected to get string or multiline string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT STR");
        Ok(String::from("test"))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT BORROWED STR");
        Ok(String::from("test"))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT STRING");
        Ok(String::from("test"))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT bytes");
        Ok(String::from("test"))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT borrowed bytes");
        Ok(String::from("test"))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        println!(">>>> VISIT byte buf");
        Ok(String::from("test"))
    }
}

/// Hold multiline string as the original `String` type in serde_json causes control flow character
/// error when deserializing.
///
/// Use it on fields may contain line feed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MultilineString {
    pub content: String,
}

impl<'de> Deserialize<'de> for MultilineString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        println!(">>>> DESERIALIZING");
        let content = deserializer.deserialize_string(MultilineStringVisitor)?;
        Ok(Self { content })
    }
}
