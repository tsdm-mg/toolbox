macro_rules! compiling_error {
    ($span: expr, $($arg: tt),*) => {
        syn::Error::new($span, format!($($arg),*)).to_compile_error().into()
    };
}

pub(crate) use compiling_error;
