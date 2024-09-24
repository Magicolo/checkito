mod utility;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use regex_syntax::Parser;
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, Expr, ExprAssign,
    ExprLit, FnArg, ItemFn, Lit, PatType, ReturnType, Signature,
};

#[proc_macro]
pub fn regex(input: TokenStream) -> TokenStream {
    let Lit::Str(string) = parse_macro_input!(input) else {
        return quote!(::core::compile_error!("Expected string literal.")).into();
    };
    match Parser::new().parse(&string.value()) {
        Ok(_) => {
            quote_spanned!(string.span() => ::checkito::regex::Regex::new(#string).unwrap()).into()
        }
        Err(error) => {
            let error = format!("{error}");
            quote_spanned!(string.span() => ::core::compile_error!(#error)).into()
        }
    }
}

#[proc_macro_attribute]
pub fn check(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let content = parse_macro_input!(attribute with Punctuated::<Expr, Comma>::parse_terminated);
    let ItemFn {
        attrs,
        vis,
        sig:
            Signature {
                ident,
                inputs,
                output,
                ..
            },
        block,
        ..
    } = parse_macro_input!(item);
    let inputs = inputs.iter().collect::<Vec<_>>();
    let patterns = inputs
        .iter()
        .flat_map(|argument| match argument {
            FnArg::Typed(PatType { pat, .. }) => Some(pat),
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    let types = inputs
        .iter()
        .flat_map(|argument| match argument {
            FnArg::Typed(PatType { ty, .. }) => Some(ty),
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    let keys = quote!([
        count,
        errors,
        seed,
        size,
        shrinks.accept,
        shrinks.reject,
        shrinks.duration
    ]);
    let assigns = content.iter()
        .filter_map(|expression| match expression {
            Expr::Assign(ExprAssign { left, right, .. }) => {
                let path = utility::path(left);
                Some(if path.is_empty() {
                    utility::error(left, |left| format!("Invalid left expression '{left}'. Must be a key in {keys}."))
                } else if path.iter().eq(["count"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as usize),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
                    };
                    quote_spanned!(left.span() => count = #right)
                } else if path.iter().eq(["errors"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as usize),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
                    };
                    quote_spanned!(left.span() => checker.errors = #right)
                } else if path.iter().eq(["seed"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => ::core::option::Option::Some({ #literal } as u64)),
                        right => quote_spanned!(right.span() => ::core::option::Option::Some({ #right }.try_into().unwrap())),
                    };
                    quote_spanned!(left.span() => checker.seed = #right)
                } else if path.iter().eq(["size"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as f64..{ #literal } as f64),
                        Expr::Lit(ExprLit { lit: Lit::Float(literal), .. }) => quote_spanned!(literal.span() => { #literal } as f64..{ #literal } as f64),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
                    };
                    quote_spanned!(left.span() => checker.size = #right)
                } else if path.iter().eq(["shrinks", "accept"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as usize),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
                    };
                    quote_spanned!(left.span() => checker.shrinks.accept = #right)
                } else if path.iter().eq(["shrinks", "reject"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as usize),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
                    };
                    quote_spanned!(left.span() => checker.shrinks.reject = #right)
                } else if path.iter().eq(["shrinks", "duration"]) {
                    let right = match right.as_ref() {
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => ::core::time::Duration::from_secs({ #literal } as u64)),
                        Expr::Lit(ExprLit { lit: Lit::Float(literal), .. }) => quote_spanned!(literal.span() => ::core::time::Duration::from_secs_f64({ #literal } as f64)),
                        right => quote_spanned!(right.span() => ::core::time::Duration::from_secs_f64({ #right }.try_into().unwrap())),
                    };
                    quote_spanned!(left.span() => checker.shrinks.duration = #right)
                } else {
                    utility::error(left, |left| format!("Unrecognized key '{left}'. Must be one of {keys}."))
                })
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut expressions = content
        .iter()
        .filter(|expression| !matches!(expression, Expr::Assign(_)));
    let generators = types
        .iter()
        .map(|input| match expressions.next() {
            Some(Expr::Lit(ExprLit {
                lit: Lit::Str(literal),
                ..
            })) => quote_spanned!(literal.span() => ::checkito::regex!(#literal)),
            Some(expression) => quote_spanned!(expression.span() => #expression),
            _ => quote_spanned!(input.span() => <#input as ::checkito::FullGenerate>::generator()),
        })
        .collect::<Vec<_>>();
    let excess = expressions
        .map(|expression| utility::error(
    expression,
    |expression| format!("There is no corresponding parameter for the expression '{expression}'. Either add a parameter for it or remove this expression."))
        )
        .collect::<Vec<_>>();
    let output = match output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, output) => quote!(#output),
    };
    let count = if inputs.is_empty() {
        quote!(1)
    } else {
        quote!(::checkito::check::count())
    };
    quote! {
        #(#attrs)*
        #[test]
            #vis fn #ident() {
            fn check((#(#patterns,)*): (#(#types,)*)) -> #output #block

            let generator = (#(#generators,)*);
            #(#excess;)*
            let mut checker = ::checkito::Generate::checker(&generator);
            ::checkito::check::environment(&mut checker);
            let mut count = #count;
            #[allow(clippy::useless_conversion, clippy::unnecessary_cast, clippy::unnecessary_fallible_conversions)]
            { 
                #(#assigns;)* 
                checker.items = false;
                checker.count = count;
            }
            for result in checker.checks(check) {
                if let ::core::result::Result::Err(error) = result {
                    ::core::panic!("{}", error);
                }
            }
        }
    }
    .into()
}
