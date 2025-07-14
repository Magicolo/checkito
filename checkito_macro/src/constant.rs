use quote::quote_spanned;
use syn::{
    __private::{Span, TokenStream2},
    Block, Expr, ExprBinary, ExprBlock, ExprCast, ExprConst, ExprGroup, ExprLit, ExprRange,
    ExprUnary, Ident, Lit, RangeLimits, Stmt, Type, TypeGroup, TypeParen, TypePath,
    spanned::Spanned,
};

pub fn convert(expression: &Expr) -> Option<TokenStream2> {
    if let Some((module, constant)) = unpack_expression(expression) {
        return Some(quote_spanned!(expression.span() => {
            #[allow(unused_braces)]
            #[allow(clippy::unnecessary_cast)]
            <::checkito::primitive::#module::#constant::<{ #expression }> as ::checkito::primitive::Constant>::VALUE
        }));
    }

    match expression {
        Expr::Group(ExprGroup { expr, .. }) => convert(expr),
        Expr::Range(ExprRange {
            start, limits, end, ..
        }) => {
            let (left, right, module, constant) = match (start, end) {
                (None, None) => return None,
                (None, Some(end)) => {
                    let (module, constant) = unpack_expression(end)?;
                    (
                        quote_spanned!(expression.span() => #module::MIN),
                        match limits {
                            RangeLimits::HalfOpen(_) => quote_spanned!(end.span() => #end - 1),
                            RangeLimits::Closed(_) => quote_spanned!(end.span() => #end),
                        },
                        module,
                        constant,
                    )
                }
                (Some(start), None) => {
                    let (module, constant) = unpack_expression(start)?;
                    (
                        quote_spanned!(start.span() => #start),
                        quote_spanned!(expression.span() => #module::MAX),
                        module,
                        constant,
                    )
                }
                (Some(start), Some(end)) => {
                    let (module, constant) =
                        match (unpack_expression(start), unpack_expression(end)) {
                            (None, None) => return None,
                            (Some(left), None) => left,
                            (None, Some(right)) => right,
                            (Some(left), Some(right)) if left == right => left,
                            (Some(_), Some(_)) => return None,
                        };
                    (
                        quote_spanned!(start.span() => #start),
                        match limits {
                            RangeLimits::HalfOpen(_) => quote_spanned!(end.span() => #end - 1),
                            RangeLimits::Closed(_) => quote_spanned!(end.span() => #end),
                        },
                        module,
                        constant,
                    )
                }
            };
            Some(quote_spanned!(expression.span() => {
                #[allow(unused_braces)]
                #[allow(clippy::unnecessary_cast)]
                <::checkito::state::Range::<::checkito::primitive::#module::#constant::<{ #left }>, ::checkito::primitive::#module::#constant::<{ #right }>> as ::checkito::primitive::Constant>::VALUE
            }))
        }
        _ => None,
    }
}

fn unpack_expression(expression: &Expr) -> Option<(Ident, Ident)> {
    match expression {
        Expr::Group(ExprGroup { expr, .. }) => unpack_expression(expr),
        Expr::Const(ExprConst { block, .. }) => unpack_block(block),
        Expr::Block(ExprBlock { block, .. }) => unpack_block(block),
        Expr::Cast(ExprCast { expr, ty, .. }) if unpack_expression(expr).is_some() => {
            unpack_type(ty)
        }
        Expr::Lit(ExprLit { lit, .. }) => unpack_literal(lit),
        Expr::Unary(ExprUnary { expr, .. }) => unpack_expression(expr),
        Expr::Binary(ExprBinary { left, right, .. }) => {
            match (unpack_expression(left), unpack_expression(right)) {
                (None, None) => None,
                (None, Some(pair)) => Some(pair),
                (Some(pair), None) => Some(pair),
                (Some(left), Some(right)) if left == right => Some(left),
                (Some(_), Some(_)) => None,
            }
        }
        _ => None,
    }
}

fn unpack_block(block: &Block) -> Option<(Ident, Ident)> {
    match block.stmts.last()? {
        Stmt::Expr(expr, None) => unpack_expression(expr),
        _ => None,
    }
}

fn unpack_type(type_: &Type) -> Option<(Ident, Ident)> {
    match type_ {
        Type::Group(TypeGroup { elem, .. }) => unpack_type(elem),
        Type::Paren(TypeParen { elem, .. }) => unpack_type(elem),
        Type::Path(TypePath { qself: None, path }) => {
            unpack_name(path.get_ident()?.to_string().as_str(), path.span())
        }
        _ => None,
    }
}

fn unpack_literal(literal: &Lit) -> Option<(Ident, Ident)> {
    match literal {
        Lit::Bool(value) => Some((
            Ident::new("bool", value.span()),
            Ident::new("Bool", value.span()),
        )),
        Lit::Char(value) => Some((
            Ident::new("char", value.span()),
            Ident::new("Char", value.span()),
        )),
        Lit::Byte(value) => Some((
            Ident::new("u8", value.span()),
            Ident::new("U8", value.span()),
        )),
        Lit::Int(value) if value.suffix().is_empty() => Some((
            Ident::new("i32", value.span()),
            Ident::new("I32", value.span()),
        )),
        Lit::Int(value) => unpack_name(value.suffix(), value.span()),
        _ => None,
    }
}

fn unpack_name(name: &str, span: Span) -> Option<(Ident, Ident)> {
    match name {
        "bool" => Some((Ident::new("bool", span), Ident::new("Bool", span))),
        "char" => Some((Ident::new("char", span), Ident::new("Char", span))),
        "u8" => Some((Ident::new("u8", span), Ident::new("U8", span))),
        "u16" => Some((Ident::new("u16", span), Ident::new("U16", span))),
        "u32" => Some((Ident::new("u32", span), Ident::new("U32", span))),
        "u64" => Some((Ident::new("u64", span), Ident::new("U64", span))),
        "u128" => Some((Ident::new("u128", span), Ident::new("U128", span))),
        "usize" => Some((Ident::new("usize", span), Ident::new("Usize", span))),
        "i8" => Some((Ident::new("i8", span), Ident::new("I8", span))),
        "i16" => Some((Ident::new("i16", span), Ident::new("I16", span))),
        "i32" => Some((Ident::new("i32", span), Ident::new("I32", span))),
        "i64" => Some((Ident::new("i64", span), Ident::new("I64", span))),
        "i128" => Some((Ident::new("i128", span), Ident::new("I128", span))),
        "isize" => Some((Ident::new("isize", span), Ident::new("Isize", span))),
        _ => None,
    }
}
