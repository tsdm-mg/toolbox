use crate::fragment::braced_expr::BracedExpr;
use proc_macro::TokenStream;
use proc_macro2 as pm2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input};

#[derive(Debug)]
struct NodeRoot(Punctuated<Node, syn::Token![,]>);

impl Parse for NodeRoot {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse_terminated(Node::parse, syn::Token![,])?))
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

impl Node {
    fn try_peek(input: &ParseStream) -> bool {
        Element::try_peek(&input) || Text::try_peek(&input)
    }
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if Element::try_peek(&input) {
            return Ok(Self::Element(input.parse()?));
        }

        if Text::try_peek(&input) {
            return Ok(Self::Text(input.parse()?));
        }

        Err(syn::Error::new(input.span(), "invalid node"))
    }
}

impl ToTokens for Node {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Node::Element(v) => v.to_tokens(tokens),
            Node::Text(v) => v.to_tokens(tokens),
        }
    }
}

/// Text holds plain text, no bbcode.
#[derive(Debug)]
enum Text {
    Str(syn::LitStr),
    Expr(BracedExpr),
}

impl Parse for Text {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitStr) {
            return Ok(Self::Str(input.parse()?));
        }

        if BracedExpr::try_peek(&input) {
            return Ok(Self::Expr(input.parse()?));
        }

        Err(syn::Error::new(input.span(), "invalid text"))
    }
}

impl Text {
    fn try_peek(input: &ParseStream) -> bool {
        input.peek(syn::LitStr) || BracedExpr::try_peek(&input)
    }
}

impl ToTokens for Text {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Text::Str(v) => v.to_tokens(tokens),
            Text::Expr(v) => {
                let data = v.to_token_stream();
                tokens.append_all(quote! { format!("{}", #data)})
            }
        }
    }
}

/// Element represents a bbcode tag.
///
/// BBCode format: `[$name=$attr]$children[/$name]`
#[derive(Debug)]
struct Element {
    /// Name is the bbcode tag name.
    name: syn::Ident,

    /// Hold the brace.
    _brace: syn::token::Brace,

    /// Optional attribute.
    ///
    /// Attribute in the constructor macro is in format of `key: value`.
    ///
    /// In bbcode format, only reserve the value text.
    attr: Option<Attr>,

    /// Children nodes.
    children: ElementChildren,
    // `_brace` ends here.
}

impl Element {
    fn try_peek(input: &ParseStream) -> bool {
        input.peek(syn::Ident) && input.peek2(syn::token::Brace)
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
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
        let children: ElementChildren = content.parse()?;

        Ok(Self {
            name,
            _brace,
            attr,
            children,
        })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        let attr_tokens = match &self.attr {
            Some(v) => {
                let d = v.to_token_stream();
                quote! {format!("={}", #d)}
            }
            None => quote! { String::new() },
        };
        let name = self.name.to_token_stream().to_string();
        let head = quote! { format!("[{}{}]", #name, #attr_tokens)};

        if self.children.is_self_closing() {
            // Process finishes if is self-closing tag.
            tokens.append_all(quote! { #head });
            return;
        }

        let tail = format!("[/{}]", self.name.to_token_stream());
        if self.children.is_empty() {
            tokens.append_all(quote! { format!("{}{}", #head, #tail) })
        } else {
            let body = self.children.to_token_stream();
            tokens.append_all(quote! { format!("{}{}{}", #head, #body, #tail) })
        }
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
    Str(syn::LitStr),
    Expr(BracedExpr),
}

impl Parse for AttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitStr) {
            return Ok(Self::Str(input.parse()?));
        }

        if BracedExpr::try_peek(&input) {
            return Ok(Self::Expr(input.parse()?));
        }

        Err(syn::Error::new(input.span(), "invalid attribute value"))
    }
}

impl ToTokens for AttrValue {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        match &self {
            AttrValue::Str(v) => v.to_tokens(tokens),
            AttrValue::Expr(v) => v.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
enum ElementChildren {
    Nodes(Punctuated<Node, syn::Token![,]>),
    /// A self closing tag.
    ///
    /// ```console
    /// br { / }
    /// ```
    SelfClose,
}

impl ElementChildren {
    fn try_peek(input: &ParseStream) -> bool {
        input.peek(syn::Token![/]) || Node::try_peek(&input)
    }

    fn is_self_closing(&self) -> bool {
        match self {
            ElementChildren::Nodes(_) => false,
            ElementChildren::SelfClose => true,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            ElementChildren::Nodes(v) => v.is_empty(),
            ElementChildren::SelfClose => false
        }
    }
}

impl Parse for ElementChildren {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        //return Ok(Self::Nodes(input.parse_terminated(Node::parse, syn::Token![,])?));
        if input.peek(syn::Token![/]) {
            let _: syn::Token![/] = input.parse()?;
            Ok(Self::SelfClose)
        } else {
            Ok(Self::Nodes(input.parse_terminated(Node::parse, syn::Token![,])?))
        }
    }
}

impl ToTokens for ElementChildren {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ElementChildren::Nodes(nodes) => {
                if !nodes.is_empty() {
                    let mut data = vec![];
                    for child in nodes {
                        data.push(child.to_token_stream());
                    }
                    tokens.append_all(quote! { {vec![#(#data,)*].join("")} })
                }
            }
            ElementChildren::SelfClose => {
                // Do nothing.
            }
        }
    }
}

pub(super) fn bbx_internal(input: TokenStream) -> TokenStream {
    let node_root = parse_macro_input!(input as NodeRoot);
    let mut output = vec![];
    for code in node_root.0.into_iter() {
        output.push(code.to_token_stream());
    }

    if output.is_empty() {
        return TokenStream::new();
    }

    let expanded = quote! { {vec![#(#output,)*].join("")} };
    expanded.into()
}