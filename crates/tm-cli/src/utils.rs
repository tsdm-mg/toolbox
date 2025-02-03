use std::io;
use std::io::Write;

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
    Ok(result)
}
