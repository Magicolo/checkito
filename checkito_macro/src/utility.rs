use quote::{quote_spanned, ToTokens};
use syn::{spanned::Spanned, Expr, ExprField, ExprPath, Ident, LitStr, Member, Path};

pub fn string<T: ToTokens>(tokens: T) -> String {
    quote_spanned!(tokens.span() => #tokens).to_string()
}

pub fn error<T: ToTokens>(
    tokens: T,
    format: impl FnOnce(String) -> String,
) -> syn::__private::TokenStream2 {
    let span = tokens.span();
    let error = LitStr::new(&format(string(tokens)), span);
    quote_spanned!(span => compile_error!(#error))
}

pub fn path(expression: &Expr) -> Vec<Ident> {
    fn descend(expression: &Expr, path: &mut Vec<Ident>) {
        match expression {
            Expr::Path(ExprPath {
                path:
                    Path {
                        segments,
                        leading_colon: None,
                        ..
                    },
                ..
            }) if segments.len() == 1 => {
                for segment in segments {
                    path.push(segment.ident.clone());
                }
            }
            Expr::Field(ExprField {
                base,
                member: Member::Named(name),
                ..
            }) => {
                descend(base, path);
                path.push(name.clone());
            }
            Expr::Field(ExprField {
                base,
                member: Member::Unnamed(index),
                ..
            }) => {
                descend(base, path);
                path.push(Ident::new(&string(index), index.span()));
            }
            _ => {}
        }
    }

    let mut path = Vec::new();
    descend(expression, &mut path);
    path
}
