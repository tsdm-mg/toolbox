use crate::cmd::ThreadArgs;
use anyhow::{Context, Result};
use tm_api::thread::fetch_thread_content;
use tracing::trace;

pub async fn run_thread_command(args: ThreadArgs) -> Result<()> {
    trace!("running thread command with args: {args:?}");

    let content = fetch_thread_content(args.tid, args.page)
        .await
        .context("when running thread content")?;
    println!("thread content: {content:#?}");
    Ok(())
}
