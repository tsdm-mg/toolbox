use crate::bbx::bbx_internal;
use proc_macro::TokenStream;

mod bbx;
mod fragment;

/// The macro renders BBCode.
///
/// # Usage
///
/// ## Render empty tag
///
/// Tag name is not constrained, followed by a pair of brace.
///
/// ```rust
/// use tm_bbcode_macro::bbx;
///
/// let bbcode = bbx!(b{});
/// assert_eq!(bbcode, "[b][/b]");
///
/// let bbcode = bbx!(url{});
/// assert_eq!(bbcode, "[url][/url]");
/// ```
///
/// ## Render tag contains plain text
///
/// The inner plain is considered as the child as tag.
///
/// ```rust
/// use tm_bbcode_macro::bbx;
///
/// let bbcode = bbx!(bold{"bold text"});
/// assert_eq!(bbcode, "[bold]bold text[/bold]");
///
/// // Of course indents and line breaks are allowed and would not change the result.
///
/// let bbcode = bbx!(
///     bold {
///         "bold text"
///     }
/// );
/// assert_eq!(bbcode, "[bold]bold text[/bold]");
/// ```
///
/// ## Render tag with attribute
///
/// ### What is bbcode attribute
///
/// Attribute is plain text in the head of BBCode tag, after tag name and an equal sign:
///
/// `[url=https://crates.io]crates.io: The Rust package registry[/url]`
///
/// where the `https://crates.io` after `url=` is attribute.
///
/// Equivalent html code:
///
/// `<a href="https://crates.io">crates.io: The Rust package registry</url>`.
///
/// Attribute is optional, tags may have it or not.
///
/// ### Format attribute in code
///
/// When using `bbx`, attribute is the first part in tag body wrapped by brace:
///
/// ```rust
/// use tm_bbcode_macro::bbx;
///
/// let bbcode = bbx!(
///     url {
///         {"https://crates.io"},
///         "crates.io: The Rust package registry"
///     }
/// );
///
/// assert_eq!(bbcode, "[url=https://crates.io]crates.io: The Rust package registry[/url]");
/// ```
///
/// ### Render tag contains multiple children
///
/// Tags can hold multiple children, separated by comma `,`, children can be mixed list of
/// `Element` and `Text`.
///
/// * `Element` is tag kind having tag name, may have optional attribute and children tags.
///   * With attribute: `[$TAG_NAME=$ATTRIBUTE][/$TAG_NAME]`
///   * With children tags: `[$TAG_NAME]$CHILDREN[/$TAG_NAME]`
///   * The comma between attribute and the first child is optional. `url{("a"),"b"}` and `url{("a") "b"}` are both valid.
/// * `Text` is plain text can not have attribute nor children.
///
/// ```rust
/// use tm_bbcode_macro::bbx;
///
/// let bbcode = bbx!(
///     underline {
///         "underline text",
///         color {
///             {"#cc0000"},
///             "text colored #cc0000",
///             bold {
///                 "bold text colored #cc0000"
///             }
///         },
///     },
///     italic {
///         "italic text"
///     }
/// );
///
/// assert_eq!(bbcode, "[underline]underline text[color=#cc0000]text colored #cc0000[bold]bold text colored #cc0000[/bold][/color][/underline][italic]italic text[/italic]");
/// ```
///
/// ### Render tag with attributes referring to variables
///
/// `bbx` works like `println!`, the following code refer to local variables:
///
/// ```rust
/// use tm_bbcode_macro::bbx;
///
/// let crates_io_url = "https://crates.io";
/// let rust = "Rust";
/// let package = String::from("package");
///
/// // Custom type `T` can be referred once implements the `toString` method, aka `impl From<T> for String`.
/// struct Registry {
///     value: String,
/// }
///
/// impl From<Registry> for String {
///     fn from(value: Registry) -> Self {
///         value.value.clone()
///     }
/// }
///
/// let registry = Registry {
///     value: String::from("registry"),
/// };
///
/// let bbcode = bbx!(
///     url {
///         {crates_io_url}
///     },
///     italic {
///         ("The {}", rust)
///     },
///     bold {
///         package,
///         " ",
///         registry,
///     }
/// );
///
/// assert_eq!(bbcode, "[url=https://crates.io][/url][italic]The Rust[/italic][bold]package registry[/bold]");
/// ```
///
/// The example above uses variables:
///
/// * Variable `crates_io_url` in attributes.
/// * Variable `the_rust` in format args `The {}`.
///   * **Note that shorthand format args `The {rust}` is not supported yet.**
/// * Variable `package_registry` as text child.
#[proc_macro]
pub fn bbx(input: TokenStream) -> TokenStream {
    bbx_internal(input)
}
