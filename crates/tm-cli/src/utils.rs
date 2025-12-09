use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use regex::Regex;
use std::future::Future;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use tm_api::thread::Thread as ThreadModel;
use tokio::fs;

/// Model for loading thread data from files.
///
/// Each instance holds one page of post data in a thread.
///
/// The data serialized from APIs can not tell threads' ids, those ids and page numbers are manually
/// saved in data file names by other data fetching components. Those files are expected to be in
/// "${THREAD_ID}_${PAGE_NUMBER}.json" format, thread id and page number shall be parsed and saved
/// when loading data otherwise we lose those info forever.
#[derive(Debug)]
pub(crate) struct ThreadPageData {
    /// Thread id.
    ///
    /// Parsed from data file name.
    pub tid: String,

    /// Page number.
    ///
    /// Parsed from data file name.
    pub page: String,

    /// Thread data.
    ///
    /// Deserialized from data file contents.
    pub thread: ThreadModel,
}

/// Read one line from stdin and strip the trailing '\n'.
///
/// # Errors
///
/// When failed to io on stdin/stdout.
pub fn read_line(hint: impl Into<String>) -> io::Result<String> {
    print!("{} ", hint.into());
    io::stdout().flush()?;
    let mut result = String::new();
    io::stdin().read_line(&mut result)?;
    if result.ends_with('\n') {
        result.pop();
    }
    if result.ends_with('\r') {
        result.pop();
    }
    Ok(result)
}

/// Generate a series of tasks from [task_source] by running [`closure`], run those tasks and
/// return.
///
/// # Errors
///
/// Return error when any of the generated tasks failed.
#[allow(clippy::future_not_send)]
pub async fn parallel_future<T, U, W, V>(
    task_source: T,
    buffer_size: usize,
    closure: U,
) -> Result<Vec<V>>
where
    T: Iterator,
    U: FnMut(<T as Iterator>::Item) -> W,
    W: Future<Output = Result<V>> + Sized,
{
    let ret = futures::stream::iter(task_source.map(closure))
        .buffer_unordered(buffer_size)
        .collect::<Vec<Result<V>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<V>>>()?;
    Ok(ret)
}

/// If `path` exists, ask user to delete it.
///
/// ## Returns
///
/// * `Some(true)` if ok.
/// * `Some(false)` if user reject to delete it.
/// * `Err(_)` if any error occurred.
pub(crate) async fn ask_delete_if_exists(path: &PathBuf) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    let should_delete = match read_line(format!("Dir {path:?} already exists, delete it? [y/N]"))
        .context("failed to ask output dir decision")?
        .as_str()
    {
        "y" | "Y" => true,
        _ => false,
    };
    if !should_delete {
        println!("ok, do not delete it. Exit");
        return Ok(false);
    }

    println!("delete dir {path:?}");
    if path.is_dir() {
        fs::remove_dir_all(&path)
            .await
            .context("when removing output dir")?;
    } else {
        fs::remove_file(&path)
            .await
            .context("when removing output file")?;
    }

    Ok(true)
}

pub async fn load_thread_data_from_dir(path: &str) -> Result<Vec<ThreadPageData>> {
    let mut dir = fs::read_dir(path)
        .await
        .with_context(|| format!("failed to read dir {path}"))?;

    let mut data = vec![];

    // Regex to check data file name.
    // Each data file must contain one page of thread data for a thread and the file name should be
    // in "${THREAD_ID}_${PAGE_NUMBER}.json" format so that we can parse and save thread id and page
    // number as these data not exist in the response of server APIs means only the API caller know.
    let file_name_re = Regex::new(r#"(?<tid>\d+)_(?<page>\d+).json"#)
        .expect("invalid file name regex to validate data file names");

    let mut tid: Option<String> = None;

    while let Some(entry) = dir.next_entry().await.context("failed to get next entry")? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let capture = match file_name_re.captures(file_name.as_str()) {
            Some(v) => v,
            None => continue,
        };

        // tid and page are required to match the regex so it's safe to unwrap.
        let cap_tid = capture.name("tid").unwrap().as_str().to_string();
        // Check if all json files hold data for the same thread.
        //
        // Remember the thread id first met as it is the unique thread id intended to be in the
        // directory.
        if tid.is_none() {
            tid = Some(cap_tid.clone());
        } else if tid.as_ref().unwrap() != cap_tid.as_str() {
            return Err(anyhow!("invalid thread data storage: the directory {} is expected to only has thread {}, but also has {}. Did you mix two or more threads in that directory?", path, tid.unwrap(), cap_tid));
        }
        let page = capture.name("page").unwrap().as_str();
        let p: PathBuf = [path, file_name.as_str()].iter().collect();

        let content = fs::read(p).await;
        let thread: ThreadModel =
            serde_json::from_slice(content?.as_slice()).context("invalid thread json data")?;
        data.push(ThreadPageData {
            tid: cap_tid,
            page: page.to_string(),
            thread,
        });
    }

    Ok(data)
}
