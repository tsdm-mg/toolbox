use proc_macro::TokenStream;

#[cfg(feature = "selector")]
mod selector;
mod utils;

/// Macro to generate Scraper::Selector from static string with compile time check.
///
/// ```
/// use tm_macros::selector;
///
/// let my_selector = selector!("div > a");
/// ```
#[cfg(feature = "selector")]
#[proc_macro]
pub fn selector(input: TokenStream) -> TokenStream {
    selector::selector_internal(input)
}
