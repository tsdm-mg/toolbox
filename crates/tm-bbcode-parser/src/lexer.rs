use std::str::Chars;

use crate::scanner::Scanner;
use crate::token::{TagHead, TagTail, Token};
use crate::{CLOSE, EQUAL, OPEN, SLASH};

pub struct Lexer {
    /// Inner scanner.
    source: Scanner,

    /// Start position of current constrcuting token.
    start: usize,

    /// Scanned tokens.
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(data: Chars) -> Self {
        Self {
            source: Scanner::new(data),
            start: 0,
            tokens: vec![],
        }
    }

    /// Run the process.
    pub fn scan(&mut self) {
        while let Some(ch) = self.source.next() {
            let token = match ch {
                OPEN => self.scan_head_or_tail(),
                _ => self.scan_text(),
            }
            .or_else(|| self.try_fallback());

            if let Some(token) = token {
                self.tokens.push(token);
            }
        }
    }

    pub fn print_tokens(&self) {
        println!("{:#?}", self.tokens);
    }

    /// Try construct a [Token::Head] from input.
    ///
    /// The caller shall ensure current position is on the `[`.
    fn scan_head(&mut self) -> Option<Token> {
        if self.source.done() {
            return None;
        }

        while let Some(ch) = self.source.next() {
            if ch == CLOSE {
                // Reach the end of head.
                let token = self.collect_head();
                self.start = self.source.position();
                return Some(token);
            } else if ch == OPEN {
                // Another `[` before the tag closes, invalid head.
                // Keep the unexpected `[` out of text.
                self.source.back();
                let token = self.collect_text();
                self.start = self.source.position();
                return Some(token);
            }
        }

        self.try_fallback()
    }

    /// The caller shall ensure current position is on the `[`.
    fn scan_tail(&mut self) -> Option<Token> {
        if self.source.done() {
            return None;
        }

        while let Some(ch) = self.source.next() {
            if ch == CLOSE {
                // Reach the end of text.
                let token = self.collect_tail();
                self.start = self.source.position();
                return Some(token);
            } else if ch == OPEN {
                // Another `[` before the tag closes, invalid head.
                // Keep the unexpected `[` out of text.
                self.source.back();
                let token = self.collect_text();
                self.start = self.source.position();
                return Some(token);
            }
        }

        self.try_fallback()
    }

    /// Reached the first character of tag head or tail, which is exactly a `[`.
    fn scan_head_or_tail(&mut self) -> Option<Token> {
        match self.source.curr() {
            Some(v) if v == &SLASH => self.scan_tail(),
            _ => self.scan_head(),
        }
    }

    fn scan_text(&mut self) -> Option<Token> {
        if self.source.done() {
            return None;
        }

        while let Some(ch) = self.source.next() {
            if ch == OPEN {
                self.source.back();
                // Reach the point where may have an open tag ahead.
                // Return now.
                let head = self.collect_text();
                self.start = self.source.position();
                return Some(head);
            }
        }
        let token = Some(self.collect_text());
        self.start = self.source.position();
        token
    }

    /// Fallback current in-process [Token] into [Token::Text].
    ///
    /// Consume chars bewteen `start` and `curr` as plain text.
    fn try_fallback(&mut self) -> Option<Token> {
        if self.source.done() {
            return None;
        }
        let token = self.collect_text();
        self.start = self.source.position();
        Some(token)
    }

    fn collect_text(&self) -> Token {
        Token::Text(
            self.source
                .get_range(self.start, self.source.position())
                .iter()
                .collect::<String>(),
        )
    }

    /// The caller shall ensure current range is on the first and last character
    /// of tag:
    ///
    /// ```console
    /// [ n a m e ]
    ///   |       |
    ///   |       |-> self.source.position
    ///   |-> self.start
    /// ```
    fn collect_head(&self) -> Token {
        let head_content = self.source.get_range(self.start, self.source.position());

        // The start position + 1 to skip `[` and end position -1 to exclude `]`
        let head_tag = match head_content.into_iter().position(|x| x == &EQUAL) {
            Some(v) => TagHead {
                name: head_content[1..v].into_iter().collect::<String>(),
                attr: Some(
                    head_content[v + 1..head_content.len() - 1]
                        .into_iter()
                        .collect::<String>(),
                ),
            },
            None => TagHead {
                name: head_content[1..head_content.len() - 1]
                    .into_iter()
                    .collect::<String>(),
                attr: None,
            },
        };

        Token::Head(head_tag)
    }

    /// The caller shall ensure current range is on the first and last character
    /// of tag:
    ///
    /// ```console
    /// [ / n a m e ]
    ///     |       |
    ///     |       |-> self.source.position
    ///     |-> self.start
    /// ```
    fn collect_tail(&self) -> Token {
        // The start position + 2 to skip `[/` and end position -1 to exclude `]`
        Token::Tail(TagTail {
            name: self
                .source
                .get_range(self.start + 2, self.source.position() - 1)
                .iter()
                .collect::<String>(),
        })
    }
}
