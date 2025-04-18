use proc_macro::TokenStream;
use proc_macro2 as pm2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, parse_macro_input};

/// The trait `BBCode` defines common methods for all types that need to
/// interact with bbcode format.
trait BBCode {
    /// Convert current instance into bbcode format.
    ///
    /// The converted value is plain bbcode text.
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>);
}

#[derive(Debug)]
struct NodeRoot(Punctuated<Node, syn::Token![,]>);

impl Parse for NodeRoot {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let nodes = input.parse_terminated(Node::parse, syn::Token![,])?;
        Ok(Self(nodes))
    }
}

/// Node is a unit in the context.
///
/// Each node represents a bbcode tag and may contain other tags as children.
#[derive(Debug)]
enum Node {
    /// Element node is a named tag, may contain other nodes.
    Element(Element),

    /// Text node represents plain text, does not contain bbcode tags.
    Text(Text),
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Either an element or a text.
        if input.peek(syn::Ident) {
            if input.peek2(syn::token::Brace) {
                // ident {
                input.parse().map(Node::Element)
            } else {
                // ident as text
                input.parse::<Text>().map(Node::Text)
            }
        } else if input.peek(syn::LitStr) {
            // "literal"
            input.parse().map(Node::Text)
        } else if input.peek(syn::token::Paren) {
            // (formatted args)
            input.parse().map(|x| Node::Text(Text::FormattedArgs(x)))
        } else if input.peek(syn::token::Brace) {
            // Helper text for locating error:
            //
            // 1. User may forget to write the element name before brace.
            // 2. User intends to write the attribute, but the attribute is placed after children nodes.
            Err(syn::Error::new(input.span(), "unexpected '{': If you intend to write element, add a name before '{'; Or attribute that shall be placed before children nodes"))
        } else {
            // Unknown node type.
            Err(syn::Error::new(input.span(), "invalid node type"))
        }
    }
}

impl BBCode for Node {
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>) {
        let mut t = pm2::TokenStream::new();
        match self {
            Node::Element(v) => v.to_tokens(&mut t),
            Node::Text(v) => v.to_tokens(&mut t),
        }
        tokens.push(t)
    }
}

/// Text holds plain text, no bbcode.
#[derive(Debug)]
enum Text {
    TextExpr(syn::Expr),
    FormattedArgs(FormattedArgs),
}

impl ToTokens for Text {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Text::TextExpr(v) => {
                let data = v.clone();
                tokens.append_all(quote! {
                    String::from(#data)
                });
            }
            Text::FormattedArgs(v) => v.to_tokens(tokens),
        }
    }
}

impl Parse for Text {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if syn::Expr::peek(input) {
            return Ok(Self::TextExpr(input.parse()?));
        }

        if input.peek(syn::token::Brace) {
            return Ok(Self::FormattedArgs(input.parse()?));
        }


        Err(syn::Error::new(input.span(), "invalid text"))
    }
}

impl BBCode for Text {
    fn to_bbcode(&self, tokens: &mut Vec<proc_macro2::TokenStream>) {
        match self {
            Text::TextExpr(v) =>
                tokens.push(v.to_token_stream()),
            Text::FormattedArgs(v) => v.to_bbcode(tokens),
        }
    }
}

/// Text holds plain text, no bbcode.
#[derive(Debug)]
struct TextLiteral {
    content: pm2::Literal,
}

impl ToTokens for TextLiteral {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        // Convert from &str to String
        let data = self.content.clone();
        tokens.append_all(quote! {
            String::from(#data)
        });
    }
}

impl Parse for TextLiteral {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            content: input.parse()?,
        })
    }
}

impl BBCode for TextLiteral {
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>) {
        let mut t = pm2::TokenStream::new();
        self.content.to_tokens(&mut t);
        tokens.push(t);
    }
}

/// Element represents a bbcode tag.
///
/// BBCode format: `[$name=$attr]$children[/$name]`
#[derive(Debug)]
struct Element {
    /// Name is the bbcode tag name.
    name: pm2::Ident,

    /// Hold the brace.
    _brace: syn::token::Brace,

    /// Optional attribute.
    ///
    /// Attribute in the constructor macro is in format of `key: value`.
    ///
    /// In bbcode format, only reserve the value text.
    attr: Option<Attr>,

    /// Children nodes.
    children: Punctuated<Node, syn::Token![,]>,
    // `_brace` ends here.
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        let name = self.name.to_string();
        let mut head_vec = Vec::<pm2::TokenStream>::new();
        head_vec.push(quote! {
            {format!("[{}", #name)}
        });
        if let Some(attr) = &self.attr {
            head_vec.push(quote! {{format!("=")}});
            attr.to_bbcode(&mut head_vec);
        }
        head_vec.push(quote! {{format!("]")}});
        tokens.append_all(quote! {
            {let x = "1"; let v: Vec<String> = vec![#(#head_vec),*]; v.join("")},
        });

        let mut children_vec = Vec::<pm2::TokenStream>::new();
        for child in &self.children {
            child.to_bbcode(&mut children_vec);
        }
        if !children_vec.is_empty() {
            tokens.append_all(quote! {
                {let x = "2";let v: Vec<String> = vec![#(#children_vec),*]; v.join("")},
            });
        }

        tokens.append_all(quote! {
            {format!("[/{}]", #name)}
        });
    }
}

impl BBCode for Element {
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>) {
        let mut t = pm2::TokenStream::new();
        self.to_tokens(&mut t);
        tokens.push(t);
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<pm2::Ident>()?;
        let content;
        let _brace = braced!(content in input);

        let attr;
        if content.peek(syn::token::Brace) {
            attr = Some(content.parse::<Attr>()?);
            // Optional comma after attribute, before children.
            let _ = content.parse::<syn::Token![,]>();
        } else {
            attr = None;
        }
        let children = content.parse_terminated(Node::parse, syn::Token![,])?;

        Ok(Self {
            name,
            _brace,
            attr,
            children,
        })
    }
}

/// Attr represents an optional value in bbcode tag where attribute is in
/// the head tag, exactly after the tag name and with a leading `=`.
///
/// The ident here is to tell attribute apart from plain text [Text] type.
#[derive(Debug)]
struct Attr {
    /// Holds the brace.
    _brace: syn::token::Brace,

    /// Only the value matters.
    value: AttrValue,
    // `_brace` ends here.
}

impl BBCode for Attr {
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>) {
        let mut t = pm2::TokenStream::new();
        self.value.to_tokens(&mut t);
        tokens.push(t);
    }
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _brace = braced!(content in input);

        Ok(Self {
            _brace,
            value: content.parse()?,
        })
    }
}

impl ToTokens for Attr {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        self.value.to_tokens(tokens)
    }
}

/// Attribute value, can be an ident or literal.
#[derive(Debug)]
enum AttrValue {
    AttrExpr(syn::Expr),
    FormattedArgs(FormattedArgs),
}

impl Parse for AttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if syn::Expr::peek(input) {
            return Ok(Self::AttrExpr(input.parse()?));
        }

        if input.peek(syn::token::Paren) {
            return Ok(Self::FormattedArgs(input.parse()?));
        }

        Err(syn::Error::new(input.span(), "invalid AttrValue"))
    }
}

impl ToTokens for AttrValue {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        match &self {
            AttrValue::AttrExpr(v) => tokens.append_all(quote! {
                #v.to_string()
            }),
            AttrValue::FormattedArgs(v) => {
                let mut t_vec = Vec::<pm2::TokenStream>::new();
                v.to_bbcode(&mut t_vec);
                tokens.append_all(quote! {
                    {#(#t_vec)*}
                })
            }
        }
    }
}

#[derive(Debug)]
struct FormattedArgs {
    _paren: syn::token::Paren,
    format_string: pm2::Literal,
    _punct: pm2::Punct,
    args: Punctuated<syn::Expr, syn::Token![,]>,
    // _open ends here.
}

impl ToTokens for FormattedArgs {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        let (fmt, seg) = (&self.format_string, &self.args);
        // let fmt = format!("{{0:{fmt}}}");
        tokens.append_all(quote! {
            {format!(#fmt, #seg)}
        })
    }
}

impl Parse for FormattedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _paren = parenthesized!(content in input);
        let format_string = content.parse::<pm2::Literal>()?;
        let _punct = content.parse()?;
        let args = content.parse_terminated(syn::Expr::parse, syn::Token![,])?;

        Ok(Self {
            _paren,
            format_string,
            _punct,
            args,
        })
    }
}

impl BBCode for FormattedArgs {
    fn to_bbcode(&self, tokens: &mut Vec<pm2::TokenStream>) {
        let (fmt, args) = (&self.format_string, &self.args);
        tokens.push(quote! {{
            format!(#fmt, #args)
        }});
    }
}

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
    let node_root = parse_macro_input!(input as NodeRoot);
    let mut output = vec![];
    for code in node_root.0.into_iter() {
        code.to_bbcode(&mut output);
    }
    let expanded = quote! {
        {let v: Vec<String> = vec![#(#output),*]; v.join("")}
    };
    expanded.into()
}