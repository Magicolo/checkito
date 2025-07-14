#![forbid(unsafe_code)]

use quote::ToTokens;

#[cfg(feature = "check")]
mod check;
#[cfg(feature = "constant")]
mod constant;
#[cfg(feature = "regex")]
mod regex;

/**
Converts the input string literal to a regex while validating proper syntax.
*/
#[cfg(feature = "regex")]
#[proc_macro]
pub fn regex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as regex::Regex).into()
}

/**
Converts the input expressions to a constant representation of it. The
added benefit of doing this is mainly to be able to compute statically
the cardinality of generators which, in some cases, helps with
determining the correct number of iterations for a `Checker::checks`.
 - Primitives ('u8', 'isize', 'bool', 'char', etc.) will be converted to a
  wrapper that uses `const N: {T}` (ex: `Usize::<100>`).
- Ranges ('0..100', '0..=100', etc.) will be converted such that the
  bounds use the primitive wrappers.
- Some constant expressions that wrap the previous ones will be converted
  (ex: `{ 100i8 }`, `{ 1u16 + 13 }`).
- All other expressions will be left as is.
*/
#[cfg(feature = "constant")]
#[proc_macro]
pub fn constant(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let expression = syn::parse_macro_input!(input);
    match constant::convert(&expression) {
        Some(tokens) => tokens.into(),
        None => expression.into_token_stream().into(),
    }
}

/**
An in-place replacement for the `#[test]` attribute that allows adding
parameters to test functions and providing `Generate` expressions as
arguments to this attribute. See `examples::cheats.` for usage examples.
*/
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
