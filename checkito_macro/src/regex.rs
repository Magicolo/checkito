use regex_syntax::Parser;
use syn::{
    Error, Lit, LitStr,
    parse::{Parse, ParseStream},
};

pub struct Regex(pub LitStr);

impl Parse for Regex {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let string = match Lit::parse(input)? {
            Lit::Str(string) => string,
            literal => {
                return Err(Error::new(
                    literal.span(),
                    format!("expected a string literal"),
                ));
            }
        };
        match Parser::new().parse(&string.value()) {
            Ok(_) => Ok(Regex(string)),
            Err(error) => Err(Error::new(string.span(), format!("{error}"))),
        }
    }
}
