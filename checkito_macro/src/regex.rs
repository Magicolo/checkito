use regex_syntax::Parser;
use syn::{
    parse::{Parse, ParseStream},
    Error, Lit, LitStr,
};

use crate::utility;

pub struct Regex(pub LitStr);

impl Parse for Regex {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let string = match Lit::parse(input)? {
            Lit::Str(string) => string,
            literal => {
                return Err(utility::error(literal, |literal| {
                    format!("expected '{}' to be a string literal", literal)
                }))
            }
        };
        match Parser::new().parse(&string.value()) {
            Ok(_) => Ok(Regex(string)),
            Err(error) => Err(Error::new_spanned(string, format!("{error}"))),
        }
    }
}
