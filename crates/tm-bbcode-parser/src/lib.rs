pub mod lexer;
pub mod parser;
mod scanner;
pub mod tag;
pub(crate) mod token;

const OPEN: char = '[';
const CLOSE: char = ']';
const SLASH: char = '/';
const EQUAL: char = '=';

pub fn parse_bbcode(data: impl AsRef<str>) {}
