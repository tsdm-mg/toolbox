use crate::cmd::ParseArgs;
use anyhow::Context;
use tm_api::thread::Thread;
use tokio::fs;

pub async fn run_parse_command(args: ParseArgs) -> anyhow::Result<()> {
    let content = fs::read(args.file)
        .await
        .context("when reading content file")?;
    let thread: Thread = serde_json::from_slice(content.as_slice())?;
    println!("{thread:#?}");
    Ok(())
}
