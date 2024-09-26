#![forbid(unsafe_code)]

mod check;
mod regex;
mod utility;

use std::mem::replace;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn};

#[proc_macro]
pub fn regex(input: TokenStream) -> TokenStream {
    let regex::Regex(string) = parse_macro_input!(input);
    quote!(::checkito::regex::Regex::new(#string).unwrap()).into()
}

#[proc_macro_attribute]
pub fn check(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let check: check::Check = parse_macro_input!(attribute);
    let mut checks = vec![check];
    let mut function: ItemFn = parse_macro_input!(item);
    let name = replace(&mut function.sig.ident, format_ident!("check"));
    function.attrs.retain(|attr| {
        if let Ok(check) = check::Check::try_from(attr) {
            checks.push(check);
            false
        } else {
            true
        }
    });
    let mut runs = Vec::new();
    for check in checks {
        match check.run(&function) {
            Ok(run) => runs.push(run),
            Err(error) => return error.to_compile_error().into(),
        }
    }

    let visibility = &function.vis;
    let attributes = &function.attrs;
    quote! {
        #(#attributes)*
        #[test]
        #visibility fn #name() {
            #function

            #[allow(
                clippy::useless_conversion,
                clippy::unnecessary_cast,
                clippy::unnecessary_fallible_conversions,
                clippy::unused_enumerate_index)]
            {
                #(#runs;)*
            }
        }
    }
    .into()
}
