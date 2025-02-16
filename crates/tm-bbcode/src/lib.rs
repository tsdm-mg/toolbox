use std::fmt;
use std::fmt::Formatter;
use std::vec::Vec;

mod web_color;
pub use web_color::WebColor;

/// Wrap like bbcode text.
#[macro_export]
macro_rules! bbcode {
    ($str: literal) => { vec![Box::new($str)] };

    ($var: expr) => { vec![Box::new($var)] };

    ($($vars: expr),*) => { vec![$(Box::new($vars)),*] }
}

/// The main trait defining bbcode tags.
pub trait BBCode {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result;
}

struct BBCodeWrapper<'a, T>(&'a T);

impl<'a, T: BBCode> fmt::Debug for BBCodeWrapper<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.to_bbcode(f)
    }
}

impl fmt::Debug for dyn BBCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_bbcode(f)
    }
}

pub fn bbcode_to_string<T: BBCode>(code: &T) -> String {
    format!("{:?}", BBCodeWrapper(code))
}

/// Represents all kinds of bbcode tags.
pub type AnyBBCode = Vec<Box<dyn BBCode>>;

impl BBCode for AnyBBCode {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for tag in self.iter() {
            tag.to_bbcode(formatter)?
        }
        Ok(())
    }
}

/// Tag `[table][/table]`. Table itself.
///
/// [Table] can hold a list of table rows [TableRow].
pub struct Table(Vec<TableRow>);

impl Table {
    /// Construct a table with rows data.
    pub fn new(rows: Vec<TableRow>) -> Self {
        Self(rows)
    }

    /// Construct an empty table.
    ///
    /// Equal to `[table][/table]`.
    pub fn empty() -> Self {
        Table(vec![])
    }
}

impl BBCode for Table {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[table]")?;
        for child in self.0.iter() {
            child.to_bbcode(formatter)?
        }
        formatter.write_str("[/table]")?;
        Ok(())
    }
}

/// Tag `[tr][/tr]`. Table row type.
///
/// [TableRow] can hold a list of table data.
pub struct TableRow(Vec<TableData>);

impl TableRow {
    /// Construct a table row with data.
    pub fn new(data: Vec<TableData>) -> Self {
        TableRow(data)
    }
}

impl BBCode for TableRow {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[tr]")?;
        for child in self.0.iter() {
            child.to_bbcode(formatter)?
        }
        formatter.write_str("[/tr]")?;
        Ok(())
    }
}

/// Tag `[td][/td]`. Cell in table.
///
/// [TableData] can hold a list of element, any type.
pub struct TableData {
    width: Option<usize>,
    children: AnyBBCode,
}

impl TableData {
    /// Build a [TableData] with all option parameters.
    pub fn new(width: Option<usize>, children: AnyBBCode) -> Self {
        TableData { width, children }
    }

    /// Build a [TableData] that exactly has [width] size.
    pub fn with_size(width: usize, children: AnyBBCode) -> Self {
        TableData {
            width: Some(width),
            children,
        }
    }

    /// Build a [TableData] that exactly has no size.
    pub fn no_size(children: AnyBBCode) -> Self {
        TableData {
            width: None,
            children,
        }
    }
}

impl BBCode for TableData {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(width) = self.width {
            formatter.write_fmt(format_args!("[td={width}]"))?;
        } else {
            formatter.write_str("[td]")?;
        }
        self.children.to_bbcode(formatter)?;
        formatter.write_str("[/td]")?;
        Ok(())
    }
}

/// Tag `[b][/b]`. Bold text.
pub struct Bold(AnyBBCode);

impl BBCode for Bold {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[b]")?;
        self.0.to_bbcode(formatter)?;
        formatter.write_str("[/b]")?;
        Ok(())
    }
}

/// Tag `[url=$URL]$DATA[/url]`. Url links.
pub struct Url {
    link: String,
    children: AnyBBCode,
}

impl Url {
    pub fn new(link: impl Into<String>, children: AnyBBCode) -> Self {
        Self {
            link: link.into(),
            children,
        }
    }
}

impl BBCode for Url {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_fmt(format_args!("[url={}]", self.link))?;
        self.children.to_bbcode(formatter)?;
        formatter.write_str("[/url]")?;
        Ok(())
    }
}

/// Tag `[color=$COLOR]$DATA[/color]`. Text color.
pub struct Color {
    // TODO: Restrict valid colors only.
    color: WebColor,
    children: AnyBBCode,
}

impl Color {
    pub fn new(color: WebColor, children: AnyBBCode) -> Self {
        Color { color, children }
    }
}

impl BBCode for Color {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_fmt(format_args!("[color={}]", self.color.to_string()))?;
        self.children.to_bbcode(formatter)?;
        formatter.write_str("[/color]")?;
        Ok(())
    }
}

impl BBCode for String {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl BBCode for &str {
    fn to_bbcode(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self)
    }
}
