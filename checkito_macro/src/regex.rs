use regex_syntax::Parser;
use syn::{
    Error, Expr, ExprLit, Lit, LitStr,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

pub struct Regex(pub LitStr, pub Option<Expr>);

impl Parse for Regex {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let span = input.span();
        let expressions = Punctuated::<Expr, Comma>::parse_terminated(input)?;
        let mut expressions = expressions.into_iter();
        let pattern = match expressions.next() {
            Some(Expr::Lit(ExprLit {
                lit: Lit::Str(string),
                ..
            })) => string,
            Some(expression) => {
                return Err(Error::new(expression.span(), "expected a string literal"));
            }
            None => return Err(Error::new(span, "expected a string literal")),
        };
        let repeats = expressions.next();
        if let Some(expression) = expressions.next() {
            return Err(Error::new(expression.span(), "unexpected expression"));
        }
        match Parser::new().parse(&pattern.value()) {
            Ok(_) => Ok(Regex(pattern, repeats)),
            Err(error) => Err(Error::new(pattern.span(), format!("{error}"))),
        }
    }
}
