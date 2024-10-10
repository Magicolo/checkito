use core::{fmt, mem::replace, ops::Deref};
use quote::{ToTokens, format_ident, quote_spanned};
use std::collections::HashSet;
use syn::{
    __private::{Span, TokenStream2},
    Error, Expr, ExprAssign, ExprField, ExprLit, ExprPath, ExprRange, FnArg, Ident, Lit, LitBool,
    Member, Meta, PatType, Path, PathSegment, RangeLimits, Signature,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

pub struct Check {
    pub span: Span,
    pub settings: Vec<(Key, Expr, TokenStream2)>,
    pub generators: Vec<Expr>,
    pub rest: Option<(usize, Span)>,
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
        Err(error(value, |key| {
            format!(
                "unrecognized key '{key}'\nmust be one of [{}]",
                join(", ", Self::KEYS)
            )
        }))
    }
}

impl TryFrom<&Expr> for Key {
    type Error = Error;

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        let unrecognized = || {
            error(value, |key| {
                format!(
                    "unrecognized key '{key}'\nmust be one of [{}]",
                    join(", ", Self::KEYS)
                )
            })
        };
        let invalid = || {
            error(value, |key| {
                format!(
                    "invalid expression '{key}'\nmust be a key in [{}].",
                    join(", ", Self::KEYS)
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
    pub const fn new(span: Span) -> Self {
        Self {
            span,
            settings: Vec::new(),
            generators: Vec::new(),
            rest: None,
            debug: None,
            color: None,
            verbose: None,
        }
    }

    pub fn run(&self, signature: &Signature) -> Result<TokenStream2, Error> {
        let rest = match self.rest {
            Some((rest, span)) => (
                rest,
                rest + signature.inputs.len().saturating_sub(self.generators.len()),
                span,
            ),
            None => (usize::MAX, usize::MAX, Span::call_site()),
        };
        let mut expressions = self.generators.iter();
        let mut generators = Vec::new();
        let mut arguments = Vec::new();
        for (index, parameter) in signature.inputs.iter().enumerate() {
            let FnArg::Typed(PatType { ty, .. }) = parameter else {
                return Err(error(parameter, |parameter| {
                    format!("invalid parameter '{parameter}'")
                }));
            };

            let generator = if index >= rest.0 && index < rest.1 {
                quote_spanned!(rest.2 => <#ty as ::checkito::generate::FullGenerator>::full_gen())
            } else {
                match expressions.next() {
                    Some(Expr::Infer(infer)) => {
                        quote_spanned!(infer.span() => <#ty as ::checkito::generate::FullGenerator>::full_gen())
                    }
                    Some(expression) => quote_spanned!(expression.span() => #expression),
                    None => {
                        return Err(error(parameter, |parameter| {
                            format!(
                                "missing generator for parameter '{parameter}'\neither add a \
                                 generator in the '#[check]' macro, use '_' to fill in a single \
                                 parameter or use '..' operator to fill in all remaining \
                                 parameters"
                            )
                        }));
                    }
                }
            };
            generators.push(generator);
            arguments.push(format_ident!("_{}", arguments.len()));
        }

        if let Some(expression) = expressions.next() {
            return Err(error(expression, |expression| {
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
                                check.debug = Some(as_bool(&right)?);
                                continue;
                            }
                            Key::Color => {
                                check.color = Some(as_bool(&right)?);
                                continue;
                            }
                            Key::Verbose => {
                                check.verbose = Some(as_bool(&right)?);
                                continue;
                            }
                            Key::GenerateSize => {
                                quote_spanned!(right.span() => ::checkito::check::help::IntoRange::<f64>::range(#right))
                            }
                            _ => right.to_token_stream(),
                        };
                        check.settings.push((key, *left, right));
                    } else {
                        return Err(error(left, |left| format!("duplicate key '{left}'")));
                    }
                }
                Expr::Range(ExprRange {
                    start: None,
                    end: None,
                    limits: RangeLimits::HalfOpen(_),
                    ..
                }) => {
                    if check.rest.is_some() {
                        return Err(Error::new_spanned(expression, "duplicate '..' operator"));
                    } else {
                        check.rest = Some((check.generators.len(), expression.span()));
                    }
                }
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
        if PATHS.into_iter().any(|legal| idents(path).eq(legal)) {
            if matches!(value.meta, Meta::Path(_)) {
                Ok(Check::new(value.span()))
            } else {
                value.meta.require_list()?.parse_args()
            }
        } else {
            Err(error(path, |path| {
                let paths = PATHS.into_iter().map(|path| join("::", path));
                format!(
                    "invalid attribute path '{path}'\nmust be one of [{}]",
                    join(", ", paths)
                )
            }))
        }
    }
}

fn string<T: ToTokens>(tokens: &T) -> String {
    tokens.to_token_stream().to_string()
}

fn error<T: ToTokens>(tokens: T, format: impl FnOnce(String) -> String) -> Error {
    let message = format(string(&tokens));
    Error::new_spanned(tokens, message)
}

fn join<S: AsRef<str>, I: AsRef<str>>(separator: S, items: impl IntoIterator<Item = I>) -> String {
    let mut buffer = String::new();
    let mut join = false;
    let separator = separator.as_ref();
    for item in items {
        if replace(&mut join, true) {
            buffer.push_str(separator);
        }
        buffer.push_str(item.as_ref());
    }
    buffer
}

fn idents(path: &Path) -> impl Iterator<Item = &Ident> {
    path.segments.iter().map(|PathSegment { ident, .. }| ident)
}

fn as_bool(expression: &Expr) -> Result<bool, Error> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Bool(LitBool { value, .. }),
            ..
        }) => Ok(*value),
        expression => Err(error(expression, |expression| {
            format!("expression '{expression}' must be a boolean literal",)
        })),
    }
}
