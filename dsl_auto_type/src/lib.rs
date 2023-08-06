pub mod auto_type;

use proc_macro2::TokenStream;

pub use auto_type::{Case, DeriveSettings};

enum Error {
    Syn(syn::Error),
    Darling(darling::Error),
}

pub fn auto_type_proc_macro_attribute(
    attr: TokenStream,
    input: TokenStream,
    config: auto_type::DeriveSettings,
) -> TokenStream {
    match auto_type::auto_type_impl(attr, &input, config) {
        Ok(token_stream) => token_stream,
        Err(e) => {
            let mut out = input;
            match e {
                Error::Syn(e) => {
                    out.extend(e.into_compile_error());
                }
                Error::Darling(e) => {
                    out.extend(e.write_errors());
                }
            }
            out
        }
    }
}

impl From<syn::Error> for Error {
    fn from(e: syn::Error) -> Self {
        Error::Syn(e)
    }
}
impl From<darling::Error> for Error {
    fn from(e: darling::Error) -> Self {
        Error::Darling(e)
    }
}
