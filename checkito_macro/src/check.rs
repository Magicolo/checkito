use crate::utility;
use core::fmt;
use quote::{quote, quote_spanned};
use std::{collections::HashSet, ops::Deref};
use syn::{
    __private::TokenStream2,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Error, Expr, ExprAssign, ExprLit, ExprPath, ExprRange, FnArg, Ident, ItemFn, Lit, LitBool,
    Meta, PatType, RangeLimits,
};

#[derive(Default)]
pub struct Check {
    pub settings: Vec<(Key, Expr, TokenStream2)>,
    pub generators: Vec<Expr>,
    pub rest: bool,
    pub debug: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Count,
    Seed,
    Size,
    Accept,
    Reject,
    Duration,
    Debug,
}

impl Key {
    const KEYS: [Key; 7] = [
        Key::Count,
        Key::Seed,
        Key::Size,
        Key::Accept,
        Key::Reject,
        Key::Duration,
        Key::Debug,
    ];
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        Key::into(*self)
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        Key::into(*self)
    }
}

impl From<Key> for &'static str {
    fn from(value: Key) -> Self {
        match value {
            Key::Count => "count",
            Key::Seed => "seed",
            Key::Size => "size",
            Key::Accept => "accept",
            Key::Reject => "reject",
            Key::Duration => "duration",
            Key::Debug => "debug",
        }
    }
}

impl TryFrom<&Ident> for Key {
    type Error = Error;

    fn try_from(value: &Ident) -> Result<Self, Self::Error> {
        for key in Self::KEYS {
            if value == &key {
                return Ok(key);
            }
        }
        Err(utility::error(value, |key| {
            format!(
                "unrecognized key '{key}'\nmust be one of [{}]",
                utility::join(", ", Self::KEYS)
            )
        }))
    }
}

impl TryFrom<&Expr> for Key {
    type Error = Error;

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        if let Expr::Path(ExprPath { path, .. }) = value {
            Key::try_from(path.require_ident()?)
        } else {
            Err(utility::error(value, |key| {
                format!(
                    "invalid expression '{key}'\nmust be a key in [{}].",
                    utility::join(", ", Self::KEYS)
                )
            }))
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Parse for Check {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut keys = Key::KEYS.into_iter().collect::<HashSet<_>>();
        let mut rest = false;
        let mut debug = None;
        let mut settings = Vec::new();
        let mut generators = Vec::new();
        for expression in Punctuated::<Expr, Comma>::parse_terminated(input)? {
            match expression {
                Expr::Assign(ExprAssign { left, right, .. }) => {
                    let key = Key::try_from(left.as_ref())?;
                    if keys.remove(&key) {
                        let right = match key {
                            Key::Debug => match right.as_ref() {
                                Expr::Lit(ExprLit {
                                    lit: Lit::Bool(LitBool { value, .. }),
                                    ..
                                }) => {
                                    debug = Some(*value);
                                    continue;
                                }
                                right => {
                                    return Err(utility::error(right, |right| {
                                        format!("invalid expression '{right}' for '{}'\nmust be a boolean literal", key)
                                    }))
                                }
                            },
                            Key::Count => quote_spanned!(right.span() => { #right } as usize),
                            Key::Seed => quote_spanned!(right.span() => { #right } as u64),
                            Key::Size => match right.as_ref() {
                                Expr::Lit(_) => {
                                    quote_spanned!(right.span() => { #right } as f64..{ #right } as f64)
                                }
                                right => {
                                    quote_spanned!(right.span() => { #right }.try_into().unwrap())
                                }
                            },
                            Key::Accept => quote_spanned!(right.span() => { #right } as usize),
                            Key::Reject => quote_spanned!(right.span() => { #right } as usize),
                            Key::Duration => {
                                quote_spanned!(right.span() => ::core::time::Duration::from_secs_f64({ #right } as f64))
                            }
                        };
                        settings.push((key, *left, right));
                    } else {
                        return Err(utility::error(left, |left| {
                            format!("duplicate key '{left}'")
                        }));
                    }
                }
                expression if rest => {
                    return Err(utility::error(expression, |expression| {
                        format!("excess expression '{expression}' after '..' operator\nonly configuration assignment expressions are allowed in this position")
                    }))
                }
                Expr::Range(ExprRange {
                    start: None,
                    end: None,
                    limits: RangeLimits::HalfOpen(_),
                    ..
                }) => rest = true,
                expression => generators.push(expression),
            }
        }
        Ok(Check {
            settings,
            generators,
            rest,
            debug,
        })
    }
}

impl TryFrom<&syn::Attribute> for Check {
    type Error = Error;

    fn try_from(value: &syn::Attribute) -> Result<Self, Self::Error> {
        const PATHS: [&[&str]; 2] = [&["checkito", "check"], &["check"]];

        let path = value.path();
        if PATHS.into_iter().any(|legal| utility::is(path, legal)) {
            if matches!(value.meta, Meta::Path(_)) {
                Ok(Check::default())
            } else {
                value.meta.require_list()?.parse_args()
            }
        } else {
            Err(utility::error(path, |path| {
                let paths = PATHS.into_iter().map(|path| utility::join("::", path));
                format!(
                    "invalid attribute path '{path}'\nmust be one of [{}]",
                    utility::join(", ", paths)
                )
            }))
        }
    }
}

impl Check {
    pub fn run(&self, function: &ItemFn) -> Result<TokenStream2, Error> {
        let mut expressions = self.generators.iter();
        let mut generators = Vec::new();
        let mut arguments = Vec::new();
        for parameter in function.sig.inputs.iter() {
            let FnArg::Typed(PatType { ty, .. }) = parameter else {
                return Err(utility::error(parameter, |parameter| {
                    format!("invalid parameter '{parameter}'")
                }));
            };

            let generator = match expressions.next() {
                Some(Expr::Lit(ExprLit {
                    lit: Lit::Str(literal),
                    ..
                })) => quote_spanned!(literal.span() => ::checkito::regex!(#literal)),
                Some(Expr::Lit(ExprLit {
                    lit:
                        literal @ (Lit::Byte(_)
                        | Lit::Char(_)
                        | Lit::Int(_)
                        | Lit::Float(_)
                        | Lit::Bool(_)),
                    ..
                })) => quote_spanned!(literal.span() => ::checkito::same::Same(#literal)),
                // TODO: Handle ranges with 'IntoGenerate'.
                Some(Expr::Infer(infer)) => {
                    quote_spanned!(infer.span() => <#ty as ::checkito::FullGenerate>::generator())
                }
                Some(expression) => quote_spanned!(expression.span() => #expression),
                None if self.rest => {
                    quote_spanned!(parameter.span() => <#ty as ::checkito::FullGenerate>::generator())
                }
                None => {
                    return Err(utility::error(parameter, |parameter| {
                        format!("missing generator for parameter '{parameter}'\neither add a generator in the '#[check]' macro, use '_' to fill in a single parameter or use '..' operator to fill in all remaining parameters")
                    }))
                }
            };
            generators.push(generator);
            let argument = Ident::new(&format!("_{}", arguments.len()), parameter.span());
            arguments.push(argument);
        }

        let mut updates = Vec::new();
        for (key, left, right) in self.settings.iter() {
            let update = match key {
                Key::Count => quote_spanned!(left.span() => _checker.count = #right;),
                Key::Seed => quote_spanned!(left.span() => _checker.seed = #right;),
                Key::Size => quote_spanned!(left.span() => _checker.size = #right;),
                Key::Accept => quote_spanned!(left.span() => _checker.shrinks.accept = #right;),
                Key::Reject => quote_spanned!(left.span() => _checker.shrinks.reject = #right;),
                Key::Duration => quote_spanned!(left.span() => _checker.shrinks.duration = #right;),
                Key::Debug => continue,
            };
            updates.push(update);
        }

        let name = &function.sig.ident;
        Ok(match self.debug {
            Some(true) => {
                quote!(::checkito::check::run::debug(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
            Some(false) => {
                quote!(::checkito::check::run::minimal(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
            None => {
                quote!(::checkito::check::run::default(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
        })
    }
}

// pub fn parse(attribute: TokenStream, item: TokenStream) -> TokenStream {
//     let content = parse_macro_input!(attribute with Punctuated::<Expr, Comma>::parse_terminated);
//     let function: ItemFn = parse_macro_input!(item);
//     let ItemFn {
//         attrs,
//         vis,
//         sig: Signature { ident, inputs, .. },
//         ..
//     } = &function;
//     let inputs = inputs.iter().collect::<Vec<_>>();
//     let pairs = inputs
//         .iter()
//         .flat_map(|argument| match argument {
//             FnArg::Typed(pattern) => Some(pattern),
//             FnArg::Receiver(_) => None,
//         })
//         .collect::<Vec<_>>();
//     let mut keys = [
//         Key::Accept,
//         Key::Count,
//         Key::Debug,
//         Key::Duration,
//         Key::Reject,
//         Key::Seed,
//         Key::Size,
//     ]
//     .map(|key| (Ident::new(&key, Span::call_site().into()), Some(key)))
//     .into_iter()
//     .collect();

//     let mut debug = None;
//     let assigns = content.iter()
//         .filter_map(|expression| match expression {
//             Expr::Assign(ExprAssign { left, right, .. }) => {
//                 Some(match utility::take(left, &mut keys) {
//                     Ok(Key::Debug) => {
//                         match right.as_ref() {
//                             Expr::Lit(ExprLit { lit: Lit::Bool(LitBool { value, .. }), .. }) => { debug = Some(*value); quote!() },
//                             right => utility::error(right, |right| format!("Invalid right expression '{right}'. Must be a boolean literal.")),
//                         }
//                     }
//                     Ok(Key::Count) => quote_spanned!(left.span() => checker.count = { #right } as usize),
//                     Ok(Key::Seed) => quote_spanned!(left.span() => checker.seed = { #right } as u64),
//                     Ok(Key::Size) => {
//                         let right = match right.as_ref() {
//                             Expr::Lit(_) => quote_spanned!(right.span() => { #right } as f64..{ #right } as f64),
//                             right => quote_spanned!(right.span() => { #right }.try_into().unwrap()),
//                         };
//                         quote_spanned!(left.span() => checker.size = #right)
//                     }
//                     Ok(Key::Accept) => quote_spanned!(left.span() => checker.shrinks.accept = { #right } as usize),
//                     Ok(Key::Reject) => quote_spanned!(left.span() => checker.shrinks.reject = { #right } as usize),
//                     Ok(Key::Duration) => quote_spanned!(left.span() => checker.shrinks.duration = ::core::time::Duration::from_secs_f64({ #right } as f64)),
//                     Err(error) => error,
//                 })
//             },
//             _ => None,
//         })
//         .collect::<Vec<_>>();
//     let rest = Cell::new(false);
//     let mut expressions = content
//         .iter()
//         .filter_map(|expression| match expression {
//             Expr::Assign(_) => None,
//             expression if rest.get() => Some(Err(utility::error(expression, |expression| format!("Excess expression '{expression}' after '..' operator. Only assignment expression are allowed in this position to configure the generation.")))),
//             Expr::Range(ExprRange { start: None, end: None, limits: RangeLimits::HalfOpen(_), .. }) => {
//                 rest.set(true);
//                 None
//             }
//             expression => Some(Ok(expression)),
//         });
//     let (generators, errors) = pairs
//         .iter()
//         .map(|pattern @ PatType { ty, .. }| match expressions.next() {
//             Some(Err(error)) => Err(error),
//             Some(Ok(expression @ Expr::Lit(ExprLit { lit: Lit::Str(literal), .. }))) => {
//                 Ok(quote_spanned!(expression.span() => ::checkito::regex!(#literal)))
//             },
//             Some(Ok(expression @ Expr::Lit(ExprLit { lit: literal @ (Lit::Byte(_) | Lit::Char(_) | Lit::Int(_) | Lit::Float(_) | Lit::Bool(_)), .. }))) =>
//                 Ok(quote_spanned!(expression.span() => ::checkito::same::Same(#literal))),
//             // TODO: Handle ranges with 'IntoGenerate'.
//             Some(Ok(expression @ Expr::Infer(_))) => Ok(quote_spanned!(expression.span() => <#ty as ::checkito::FullGenerate>::generator())),
//             Some(Ok(expression)) => Ok(quote_spanned!(expression.span() => #expression)),
//             None if rest.get() => Ok(quote_spanned!(pattern.span() => <#ty as ::checkito::FullGenerate>::generator())),
//             None => Err(utility::error(pattern, |pattern| format!("Missing generator for parameter '{pattern}'. Either add a generator in the '#[check]' macro, use '_' to fill in a single parameter or use '..' operator to fill in all remaining parameters."))),
//         })
//         .partition::<Vec<_>, _>(|result| result.is_ok());
//     let generators = generators
//         .into_iter()
//         .filter_map(Result::ok)
//         .collect::<Vec<_>>();
//     let names = (0..generators.len())
//         .map(|i| Ident::new(&format!("_{i}"), Span::call_site().into()))
//         .collect::<Vec<_>>();
//     let errors = expressions
//         .filter_map(|expression| match expression {
//             Ok(expression) => Some(utility::error(expression, |expression| format!("Excess expression '{expression}' with no corresponding parameter. Either add a parameter or remove this expression."))),
//             Err(error) => Some(error),
//         })
//         .chain(errors.into_iter().filter_map(Result::err))
//         .collect::<Vec<_>>();
//     let check = match debug {
//         Some(true) => quote! {
//             match result {
//                 ::core::result::Result::Ok(item) => ::std::println!("\x1b[32mCHECK({})\x1b[0m: {:?}", _i + 1, item),
//                 ::core::result::Result::Err(error) => {
//                     ::std::eprintln!("\x1b[31mCHECK({})\x1b[0m: {error:?}", _i + 1);
//                     ::core::panic!();
//                 }
//             }
//         },
//         Some(false) => quote! {
//             if let ::core::result::Result::Err(error) = result {
//                 ::std::eprintln!();
//                 ::std::eprintln!("\x1b[31mCHECK({})\x1b[0m: {{ type: {:?}, seed: {} }}", error.index() + 1, ::core::any::type_name_of_val(error.item()), error.seed());
//                 ::core::panic!();
//             }
//         },
//         None => quote! {
//             if let ::core::result::Result::Err(error) = result {
//                 ::std::eprintln!();
//                 ::std::eprintln!("\x1b[31mCHECK({})\x1b[0m: {{ item: {:?}, seed: {}, message: \"{}\" }}", error.index() + 1, error.item(), error.seed(), error.message());
//                 ::core::panic!();
//             }
//         },
//     };
//     let debug = debug.unwrap_or_default();
//     let panic = if debug {
//         quote!(::std::panic::set_hook(::std::boxed::Box::new(|_| {}));)
//     } else {
//         quote!()
//     };
//     quote! {
//         #(#attrs)*
//         #[test]
//         #vis fn #ident() {
//             #(#errors;)*
//             #function

//             #[allow(
//                 clippy::useless_conversion,
//                 clippy::unnecessary_cast,
//                 clippy::unnecessary_fallible_conversions,
//                 clippy::unused_enumerate_index)]
//             {
//                 let generator = (#(#generators,)*);
//                 let mut checker = ::checkito::Check::checker(&generator);
//                 ::checkito::check::environment::update(&mut checker);
//                 #(#assigns;)*
//                 checker.items = #debug;
//                 #panic
//                 for (_i, result) in checker.checks(|(#(#names,)*)| #ident(#(#names,)*)).enumerate() {
//                     #check
//                 }
//             }
//         }
//     }
//     .into()
// }
