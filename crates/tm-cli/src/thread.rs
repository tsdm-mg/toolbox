use crate::cmd::ThreadArgs;
use crate::utils::read_line;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tm_api::thread::fetch_thread_content;
use tokio::fs;
use tokio::time::sleep;
use tracing::trace;

pub async fn run_thread_command(args: ThreadArgs) -> Result<()> {
    trace!("running thread command with args: {args:?}");

    let timestamp = chrono::offset::Local::now().format("%Y%m%d%H%M%S");
    let output_dir_raw_path = args
        .output
        .unwrap_or(format!("fetched_thread_{timestamp}",));

    let output_dir_path = PathBuf::from(output_dir_raw_path.as_str());
    if output_dir_path.exists() {
        let should_delete = match read_line(format!(
            "Dir {output_dir_raw_path} already exists, delete it? [y/N]"
        ))
        .context("failed to ask output dir decision")?
        .as_str()
        {
            "y" | "Y" => true,
            v => false,
        };
        if !should_delete {
            println!("ok, do not delete it. Exit");
            return Ok(());
        }

        println!("delete dir {output_dir_raw_path}");
        if output_dir_path.is_dir() {
            fs::remove_dir_all(&output_dir_path)
                .await
                .context("when removing output dir")?;
        } else {
            fs::remove_file(&output_dir_path)
                .await
                .context("when removing output file")?;
        }
    }

    fs::create_dir(&output_dir_path)
        .await
        .context("when creating output_dir")?;

    let tid = args.tid;

    // Fetch all pages in thread.
    if args.all == Some(true) {
        let mut page = 1;
        let mut total_pages = 1;

        while page <= total_pages {
            println!("fetch page: tid={tid}, page={page}, total_pages={total_pages}");
            let mut output_file = output_dir_path.clone();
            output_file.push(format!("{tid}_{page}.json"));
            let (post_per_page, total_post) = download_single_page(output_file, tid, page)
                .await
                .context("when downloading single page")?;

            if post_per_page == 0 {
                total_pages = 1;
            } else {
                total_pages = total_post.div_ceil(post_per_page);
            }

            sleep(Duration::from_millis(700)).await;
            page += 1;
        }
        println!("done");
        return Ok(());
    }

    // Fetch single page.
    let page = args.page;
    println!("fetch page: tid={tid}, page={page}");
    let mut output_file = output_dir_path.clone();
    output_file.push(format!("{timestamp}_{tid}_{page}.json"));
    download_single_page(output_file, tid, page)
        .await
        .context("when fetching single page")?;
    println!("done");
    Ok(())
}

/// Download a single page specified by `tid` and `page` to file `output_file`.
///
/// Return post per page and total post as a tuple count.
async fn download_single_page(output_file: PathBuf, tid: u32, page: u32) -> Result<(u32, u32)> {
    trace!("fetching content for thread {} page {}", tid, page);

    let content = fetch_thread_content(tid, page)
        .await
        .context("when running thread content")?;
    trace!(
        "saving content for thread {} page {} to {:?}",
        tid,
        page,
        output_file.as_path()
    );

    let post_per_page = content.post_per_page.value().parse::<u32>().unwrap_or(1);
    let totals = content.total_post.parse::<u32>().unwrap_or(0);

    fs::write(
        output_file,
        serde_json::to_string_pretty(&content).context("when serializing thread content")?,
    )
    .await
    .with_context(|| format!("when saving for thread {tid} page {page}"))?;

    Ok((post_per_page, totals))
}
