use crate::utility;
use core::fmt;
use quote::{format_ident, quote_spanned};
use std::{collections::HashSet, ops::Deref};
use syn::{
    __private::{Span, TokenStream2},
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Error, Expr, ExprAssign, ExprLit, ExprPath, ExprRange, FnArg, Ident, ItemFn, Lit, LitBool,
    Meta, PatType, RangeLimits,
};

pub struct Check {
    pub span: Span,
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

impl Check {
    pub fn new(span: Span) -> Self {
        Self {
            span,
            settings: Vec::new(),
            generators: Vec::new(),
            rest: false,
            debug: None,
        }
    }

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
            arguments.push(format_ident!("_{}", arguments.len()));
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
                quote_spanned!(self.span => ::checkito::check::run::debug(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
            Some(false) => {
                quote_spanned!(self.span => ::checkito::check::run::minimal(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
            None => {
                quote_spanned!(self.span => ::checkito::check::run::default(
                    (#(#generators,)*),
                    |_checker| { #(#updates)* },
                    |(#(#arguments,)*)| #name(#(#arguments,)*)
                ))
            }
        })
    }
}

impl Parse for Check {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut check = Check::new(input.span());
        let mut keys = Key::KEYS.into_iter().collect::<HashSet<_>>();
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
                                    check.debug = Some(*value);
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
                        check.settings.push((key, *left, right));
                    } else {
                        return Err(utility::error(left, |left| {
                            format!("duplicate key '{left}'")
                        }));
                    }
                }
                expression if check.rest => {
                    return Err(utility::error(expression, |expression| {
                        format!("excess expression '{expression}' after '..' operator\nonly configuration assignment expressions are allowed in this position")
                    }))
                }
                Expr::Range(ExprRange {
                    start: None,
                    end: None,
                    limits: RangeLimits::HalfOpen(_),
                    ..
                }) => check.rest = true,
                expression => check.generators.push(expression),
            }
        }
        Ok(check)
    }
}

impl TryFrom<&syn::Attribute> for Check {
    type Error = Error;

    fn try_from(value: &syn::Attribute) -> Result<Self, Self::Error> {
        const PATHS: [&[&str]; 2] = [&["checkito", "check"], &["check"]];

        let path = value.path();
        if PATHS.into_iter().any(|legal| utility::is(path, legal)) {
            if matches!(value.meta, Meta::Path(_)) {
                Ok(Check::new(value.span()))
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
