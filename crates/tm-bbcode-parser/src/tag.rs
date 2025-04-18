pub trait Tag {
    /// Convert into BBCode.
    fn to_bbcode(&self) -> String;

    /// Get the attribute, if any.
    fn attr(&self) -> Option<&str>;
}
