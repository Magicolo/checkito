use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use regex_syntax::Parser;
use syn::{parse_macro_input, Lit};

#[proc_macro]
pub fn regex(input: TokenStream) -> TokenStream {
    if let Lit::Str(string) = parse_macro_input!(input) {
        match Parser::new().parse(&string.value()) {
            Ok(_) => {
                quote_spanned!(string.span() => ::checkito::regex::Regex::new(#string).unwrap()).into()
            }
            Err(error) => {
                let error = format!("{error}");
                quote_spanned!(string.span() => ::core::compile_error!(#error)).into()
            }
        }
    } else {
        quote!(::core::compile_error!("Expected string literal.")).into()
    }
}
