use crate::utils::compiling_error;
use proc_macro::TokenStream;
use quote::quote;
use scraper::Selector;
use syn::{parse_macro_input, LitStr};

pub fn selector_internal(input: TokenStream) -> TokenStream {
    let selector = parse_macro_input!(input as LitStr);
    let selector_str = selector.value();
    let stream = match Selector::parse(selector_str.as_str()) {
        Ok(_) => quote!(
            ::scraper::Selector::parse(#selector_str).unwrap()
        )
        .into(),
        Err(e) => compiling_error!(
            proc_macro2::Span::call_site(),
            "invalid scraper selector: {}",
            e
        ),
    };

    stream
}
