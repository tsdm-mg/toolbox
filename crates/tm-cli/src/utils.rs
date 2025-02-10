use anyhow::{Context, Result};
use futures::StreamExt;
use std::future::Future;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use tokio::fs;

/// Read one line from stdin and strip the trailing '\n'.
///
/// # Errors
///
/// When failed to io on stdin/stdout.
pub fn read_line(hint: impl Into<String>) -> io::Result<String> {
    print!("{:?} ", hint.into());
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
