use std::str::Chars;

/// Scanner on a string.
///
/// Provide convienient APIs on operation.
pub(crate) struct Scanner {
    /// characters splitted.
    chars: Vec<char>,

    /// Characters count.
    pub(crate) chars_count: usize,

    /// Current position.
    position: usize,
}

impl Scanner {
    pub(crate) fn new(source: Chars<'_>) -> Scanner {
        let chars = source.collect::<Vec<char>>();
        chars.iter().next();
        let chars_count = chars.len();
        Self {
            chars,
            chars_count,
            position: 0,
        }
    }

    /// Check the scanner process reached the end or not.
    pub fn done(&self) -> bool {
        self.position > self.chars_count
    }

    /// Get character at the current position.
    pub fn curr(&self) -> Option<&char> {
        self.chars.get(self.position)
    }

    /// Check the next character is [ch] or not, without advancing the current position.
    pub fn peek(&mut self) -> Option<&char> {
        if self.done() {
            return None;
        }

        self.chars.get(self.position + 1)
    }

    /// Move the position forward and return the character walked through.
    ///
    /// Return `None` if already finished.
    pub fn next(&mut self) -> Option<char> {
        if self.done() {
            return None;
        }
        let ch = self.chars.get(self.position).map(|x| {
            self.position += 1;
            x.to_owned()
        });
        ch
    }

    /// Move the position back 1 position.
    pub fn back(&mut self) {
        if self.position() == 0 {
            return;
        }
        self.position -= 1;
    }

    /// Get a slice of chars from [start] to [end], excluding end pos.
    ///
    /// The caller must ensure sizes not out of range.
    pub fn get_range(&self, start: usize, end: usize) -> &[char] {
        &self.chars[start..end]
    }

    pub fn position(&self) -> usize {
        self.position
    }
}
