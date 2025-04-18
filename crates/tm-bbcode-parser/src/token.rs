/// All types of tokens.
///
/// Tokens are units of of bbcode tags.
#[derive(Debug)]
pub(crate) enum Token {
    /// Tag head.
    Head(TagHead),

    /// Tag tail.
    Tail(TagTail),

    /// Plain text.
    Text(String),
}

/// Tag head.
///
/// `[$name=$attr]` or `[$name]`
#[derive(Debug)]
pub(crate) struct TagHead {
    /// Tag name.
    pub name: String,

    /// Optional attribute.
    pub attr: Option<String>,
}

/// Tag tail.
///
/// `[/$name]`
#[derive(Debug)]
pub(crate) struct TagTail {
    /// Tag name.
    pub name: String,
}
