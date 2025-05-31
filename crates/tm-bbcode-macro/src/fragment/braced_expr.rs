use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseStream},
};

/// Expression wrapped by braced.
///
/// ```console
/// ${ /* rust expression */ }
/// ```
#[derive(Debug)]
pub(crate) struct BracedExpr {
    _dollar: syn::Token![$],
    _brace: syn::token::Brace,
    expr: syn::Expr,
}

impl BracedExpr {
    pub(crate) fn try_peek(input: &ParseStream) -> bool {
        input.peek(syn::Token![$]) && input.peek2(syn::token::Brace)
    }

    pub(crate) fn try_peek2(input: &ParseStream) -> bool {
        input.peek2(syn::Token![$]) && input.peek3(syn::token::Brace)
    }
}

impl Parse for BracedExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _dollar = input.parse::<syn::Token![$]>()?;
        let content;
        let _brace = braced!(content in input);
        let expr = syn::Expr::parse(&content)?;

        Ok(Self {
            _dollar,
            _brace,
            expr,
        })
    }
}

impl ToTokens for BracedExpr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.expr.to_tokens(tokens);
        return;
    }
}
