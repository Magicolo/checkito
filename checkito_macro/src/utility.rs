use quote::{quote_spanned, ToTokens};
use std::mem::replace;
use syn::{spanned::Spanned, Error, Ident, Path};

pub fn string<T: ToTokens>(tokens: &T) -> String {
    quote_spanned!(tokens.span() => #tokens).to_string()
}

pub fn error<T: ToTokens>(tokens: T, format: impl FnOnce(String) -> String) -> Error {
    let message = format(string(&tokens));
    Error::new_spanned(tokens, message)
}

pub fn join<'a, S: AsRef<str>, I: AsRef<str>>(
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
    path.segments.iter().map(|segment| &segment.ident)
}

pub fn is<'a, T: AsRef<str> + ?Sized + 'a>(
    left: &Path,
    right: impl IntoIterator<Item = &'a T>,
) -> bool {
    idents(left).eq(right)
}
