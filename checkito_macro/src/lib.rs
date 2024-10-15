#![forbid(unsafe_code)]

#[cfg(feature = "check")]
mod check;
#[cfg(feature = "regex")]
mod regex;

#[cfg(feature = "regex")]
#[proc_macro]
pub fn regex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use quote::quote;
    use syn::parse_macro_input;
    let regex::Regex(pattern, repeats) = parse_macro_input!(input);
    let pattern = pattern.token();
    let repeats = match repeats {
        Some(repeats) => quote!({ #repeats }.into()),
        None => quote!(None),
    };
    quote!(::checkito::regex(#pattern, #repeats).unwrap()).into()
}

#[cfg(feature = "check")]
#[proc_macro_attribute]
pub fn check(
    attribute: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    use core::mem::{replace, take};
    use quote::{format_ident, quote};
    use syn::{ItemFn, Visibility, parse_macro_input};

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
