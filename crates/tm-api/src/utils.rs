use reqwest::{Error, Response};
use std::future::Future;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

pub(crate) fn http_get(
    url: impl AsRef<str>,
) -> impl Future<Output = Result<Response, Error>> + Sized {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .unwrap()
        .get(url.as_ref())
        .send()
}
