mod utility;

use std::cell::Cell;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use regex_syntax::Parser;
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, Expr, ExprRange, ExprAssign, ExprLit, FnArg, ItemFn, Lit, PatType, RangeLimits, ReturnType, Signature
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
    let pairs = inputs
        .iter()
        .flat_map(|argument| match argument {
            FnArg::Typed(pattern) => Some(pattern),
            FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    let patterns = pairs.iter().map(|PatType { pat, .. }| pat).collect::<Vec<_>>();
    let types = pairs.iter().map(|PatType { ty, .. }| ty).collect::<Vec<_>>();
    let keys = quote!([count, errors, seed, size, shrinks.accept, shrinks.reject, shrinks.duration]);
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
                        Expr::Lit(ExprLit { lit: Lit::Int(literal), .. }) => quote_spanned!(literal.span() => { #literal } as u64),
                        right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
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
    let rest = Cell::new(false);
    let mut expressions = content
        .iter()
        .filter_map(|expression| match expression {
            Expr::Assign(_) => None,
            expression if rest.get() => Some(Err(utility::error(expression, |expression| format!("Excess expression '{expression}' after '..' operator. Only assignment expression are allowed in this position to configure the generation.")))),
            Expr::Range(ExprRange { start: None, end: None, limits: RangeLimits::HalfOpen(_), .. }) => {
                rest.set(true);
                None
            }
            expression => Some(Ok(expression)),
        });
    let (generators, errors) = pairs
        .iter()
        .map(|pattern @ PatType { ty, .. }| match expressions.next() {
            Some(Err(error)) => Err(error),
            Some(Ok(expression @ Expr::Lit(ExprLit { lit: Lit::Str(literal), .. }))) => Ok(quote_spanned!(expression.span() => ::checkito::regex!(#literal))),
            Some(Ok(expression @ Expr::Infer(_))) => Ok(quote_spanned!(expression.span() => <#ty as ::checkito::FullGenerate>::generator())),
            Some(Ok(expression)) => Ok(quote_spanned!(expression.span() => #expression)),
            None if rest.get() => Ok(quote_spanned!(pattern.span() => <#ty as ::checkito::FullGenerate>::generator())),
            None => Err(utility::error(pattern, |pattern| format!("Missing generator for parameter '{pattern}'. Either add a generator in the '#[check]' macro, use '_' to fill in a single parameter or use '..' operator to fill in all remaining parameters."))),
        })
        .partition::<Vec<_>, _>(|result| result.is_ok());
    let generators = generators.into_iter().filter_map(Result::ok).collect::<Vec<_>>();
    let errors = expressions
        .filter_map(|expression| match expression {
            Ok(expression) => Some(utility::error(expression, |expression| format!("Excess expression '{expression}' with no corresponding parameter. Either add a parameter or remove this expression."))),
            Err(error) => Some(error),
        })
        .chain(errors.into_iter().filter_map(Result::err))
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
            #(#errors;)*
            fn check((#(#patterns,)*): (#(#types,)*)) -> #output #block

            let generator = (#(#generators,)*);
            let mut checker = ::checkito::Check::checker(&generator);
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
