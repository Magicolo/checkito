use quote::ToTokens;
use std::mem::replace;
use syn::{Error, Expr, ExprLit, Ident, Lit, LitBool, Path, PathSegment};

pub fn string<T: ToTokens>(tokens: &T) -> String {
    tokens.to_token_stream().to_string()
}

pub fn error<T: ToTokens>(tokens: T, format: impl FnOnce(String) -> String) -> Error {
    let message = format(string(&tokens));
    Error::new_spanned(tokens, message)
}

pub fn join<S: AsRef<str>, I: AsRef<str>>(
    separator: S,
    items: impl IntoIterator<Item = I>,
) -> String {
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

pub fn idents(path: &Path) -> impl Iterator<Item = &Ident> {
    path.segments.iter().map(|PathSegment { ident, .. }| ident)
}

pub fn as_bool(expression: &Expr) -> Result<bool, Error> {
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
