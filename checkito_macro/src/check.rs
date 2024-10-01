use crate::utility;
use core::fmt;
use quote::{ToTokens, format_ident, quote_spanned};
use std::{collections::HashSet, ops::Deref};
use syn::{
    __private::{Span, TokenStream2},
    Error, Expr, ExprAssign, ExprField, ExprPath, ExprRange, FnArg, Ident, Member, Meta, PatType,
    RangeLimits, Signature,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

pub struct Check {
    pub span: Span,
    pub settings: Vec<(Key, Expr, TokenStream2)>,
    pub generators: Vec<Expr>,
    pub rest: bool,
    pub debug: Option<bool>,
    pub color: Option<bool>,
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Color,
    Debug,
    Verbose,
    GenerateCount,
    GenerateSeed,
    GenerateSize,
    GenerateItems,
    GenerateError,
    ShrinkCount,
    ShrinkItems,
    ShrinkErrors,
}

impl Key {
    const KEYS: [Key; 11] = [
        Key::Color,
        Key::Debug,
        Key::Verbose,
        Key::GenerateCount,
        Key::GenerateSeed,
        Key::GenerateSize,
        Key::GenerateItems,
        Key::GenerateError,
        Key::ShrinkCount,
        Key::ShrinkItems,
        Key::ShrinkErrors,
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
            Key::Color => "color",
            Key::Debug => "debug",
            Key::Verbose => "verbose",
            Key::GenerateCount => "generate.count",
            Key::GenerateSeed => "generate.seed",
            Key::GenerateSize => "generate.size",
            Key::GenerateItems => "generate.items",
            Key::GenerateError => "generate.error",
            Key::ShrinkCount => "shrink.count",
            Key::ShrinkItems => "shrink.items",
            Key::ShrinkErrors => "shrink.errors",
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
        let unrecognized = || {
            utility::error(value, |key| {
                format!(
                    "unrecognized key '{key}'\nmust be one of [{}]",
                    utility::join(", ", Self::KEYS)
                )
            })
        };
        let invalid = || {
            utility::error(value, |key| {
                format!(
                    "invalid expression '{key}'\nmust be a key in [{}].",
                    utility::join(", ", Self::KEYS)
                )
            })
        };
        match value {
            Expr::Path(ExprPath { path, .. }) => {
                let ident = path.require_ident()?;
                for key in Self::KEYS {
                    if ident == &key {
                        return Ok(key);
                    }
                }
                Err(unrecognized())
            }
            Expr::Field(ExprField { base, member, .. }) => {
                if let Member::Named(name) = member {
                    if let Expr::Path(ExprPath { path, .. }) = base.as_ref() {
                        for key in Self::KEYS {
                            if [path.require_ident()?, name].into_iter().eq(key.split('.')) {
                                return Ok(key);
                            }
                        }
                    }
                }
                Err(unrecognized())
            }
            _ => Err(invalid()),
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
            color: None,
            verbose: None,
        }
    }

    pub fn run(&self, signature: &Signature) -> Result<TokenStream2, Error> {
        let mut expressions = self.generators.iter();
        let mut generators = Vec::new();
        let mut arguments = Vec::new();
        for parameter in signature.inputs.iter() {
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
                        format!(
                            "missing generator for parameter '{parameter}'\neither add a \
                             generator in the '#[check]' macro, use '_' to fill in a single \
                             parameter or use '..' operator to fill in all remaining parameters"
                        )
                    }));
                }
            };
            generators.push(generator);
            arguments.push(format_ident!("_{}", arguments.len()));
        }

        for expression in expressions {
            return Err(utility::error(expression, |expression| {
                format!(
                    "missing parameter for generator '{expression}'\neither add a parameter in \
                     the function's signature or remove the generator"
                )
            }));
        }

        let mut updates = Vec::new();
        for (key, left, right) in self.settings.iter() {
            updates.push(match key {
                Key::GenerateCount => {
                    quote_spanned!(left.span() => _checker.generate.count = #right;)
                }
                Key::GenerateSeed => {
                    quote_spanned!(left.span() => _checker.generate.seed = #right;)
                }
                Key::GenerateSize => {
                    quote_spanned!(left.span() => _checker.generate.size = #right;)
                }
                Key::GenerateItems => {
                    quote_spanned!(left.span() => _checker.generate.items = #right;)
                }
                Key::GenerateError => {
                    quote_spanned!(left.span() => _checker.generate.error = #right;)
                }
                Key::ShrinkCount => {
                    quote_spanned!(left.span() => _checker.shrink.count = #right;)
                }
                Key::ShrinkItems => {
                    quote_spanned!(left.span() => _checker.shrink.items = #right;)
                }
                Key::ShrinkErrors => {
                    quote_spanned!(left.span() => _checker.shrink.errors = #right;)
                }
                Key::Debug | Key::Color | Key::Verbose => continue,
            });
        }

        let name = &signature.ident;
        let color = self.color.unwrap_or(true);
        let verbose = self.verbose.unwrap_or(false);
        Ok(match self.debug {
            Some(true) => quote_spanned!(self.span => ::checkito::check::help::debug(
                (#(#generators,)*),
                |_checker| { #(#updates)* },
                |(#(#arguments,)*)| #name(#(#arguments,)*),
                #color,
                #verbose,
            )),
            Some(false) => quote_spanned!(self.span => ::checkito::check::help::minimal(
                (#(#generators,)*),
                |_checker| { #(#updates)* },
                |(#(#arguments,)*)| #name(#(#arguments,)*),
                #color,
                #verbose,
            )),
            None => quote_spanned!(self.span => ::checkito::check::help::default(
                (#(#generators,)*),
                |_checker| { #(#updates)* },
                |(#(#arguments,)*)| #name(#(#arguments,)*),
                #color,
                #verbose,
            )),
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
                            Key::Debug => {
                                check.debug = Some(utility::as_bool(&right)?);
                                continue;
                            }
                            Key::Color => {
                                check.color = Some(utility::as_bool(&right)?);
                                continue;
                            }
                            Key::Verbose => {
                                check.verbose = Some(utility::as_bool(&right)?);
                                continue;
                            }
                            Key::GenerateSize => {
                                quote_spanned!(right.span() => ::checkito::check::help::IntoRange::<f64>::range(#right))
                            }
                            _ => right.to_token_stream(),
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
                        format!(
                            "excess expression '{expression}' after '..' operator\nonly \
                             configuration assignment expressions are allowed in this position"
                        )
                    }));
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
        if PATHS
            .into_iter()
            .any(|legal| utility::idents(path).eq(legal))
        {
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
