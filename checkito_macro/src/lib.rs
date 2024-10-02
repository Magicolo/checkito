#![forbid(unsafe_code)]

mod check;
mod regex;
mod utility;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::mem::{replace, take};
use syn::{ItemFn, Visibility, parse_macro_input};

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
    let visibility = replace(&mut function.vis, Visibility::Inherited);
    let mut attributes = take(&mut function.attrs);
    attributes.retain(|attribute| {
        if let Ok(check) = check::Check::try_from(attribute) {
            checks.push(check);
            false
        } else {
            true
        }
    });
    let mut runs = Vec::new();
    for check in checks {
        match check.run(&function.sig) {
            Ok(run) => runs.push(run),
            Err(error) => return error.to_compile_error().into(),
        }
    }
    quote! {
        #(#attributes)*
        #[test]
        #visibility fn #name() {
            #function
            #(#runs;)*
        }
    }
    .into()
}
