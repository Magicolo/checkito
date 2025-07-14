use quote::quote_spanned;
use syn::{
    __private::{Span, TokenStream2},
    Block, Expr, ExprBinary, ExprBlock, ExprCast, ExprConst, ExprGroup, ExprLit, ExprRange,
    ExprUnary, Ident, Lit, RangeLimits, Stmt, Type, TypeGroup, TypeParen, TypePath,
    spanned::Spanned,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    None,
    Default,
    Character(char),
}

#[derive(Clone, PartialEq, Eq)]
struct Pack {
    module: Ident,
    constant: Ident,
    kind: Kind,
}

impl Pack {
    pub fn new(module: &str, constant: &str, span: Span) -> Self {
        Self {
            module: Ident::new(module, span),
            constant: Ident::new(constant, span),
            kind: Kind::None,
        }
    }

    pub fn default(span: Span) -> Self {
        Self {
            module: Ident::new("i32", span),
            constant: Ident::new("I32", span),
            kind: Kind::Default,
        }
    }

    pub fn character(value: char, span: Span) -> Self {
        Self {
            module: Ident::new("char", span),
            constant: Ident::new("Char", span),
            kind: Kind::Character(value),
        }
    }

    pub fn is_default(&self) -> bool {
        matches!(self.kind, Kind::Default)
    }

    pub fn limit(&self, expression: &Expr, limits: &RangeLimits) -> Option<TokenStream2> {
        match limits {
            RangeLimits::HalfOpen(_) => match self.kind {
                Kind::Character(value) => {
                    let value = char::from_u32(u32::checked_sub(value as u32, 1)?)?;
                    Some(quote_spanned!(expression.span() => #value))
                }
                _ => Some(quote_spanned!(expression.span() => #expression - 1)),
            },
            RangeLimits::Closed(_) => Some(quote_spanned!(expression.span() => #expression)),
        }
    }

    pub fn merge(left: Option<Self>, right: Option<Self>) -> Option<Self> {
        match (left, right) {
            (None, None) => None,
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (Some(left), Some(right)) => {
                if right.is_default() || left == right {
                    Some(left)
                } else if left.is_default() {
                    Some(right)
                } else {
                    None
                }
            }
        }
    }
}

pub fn convert(expression: &Expr) -> Option<TokenStream2> {
    if let Some(Pack {
        module, constant, ..
    }) = unpack_expression(expression)
    {
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
            let (left, right, pack) = match (start, end) {
                (None, None) => return None,
                (None, Some(end)) => {
                    let pack = unpack_expression(end)?;
                    let module = &pack.module;
                    (
                        quote_spanned!(expression.span() => #module::MIN),
                        pack.limit(end, limits)?,
                        pack,
                    )
                }
                (Some(start), None) => {
                    let pack = unpack_expression(start)?;
                    let module = &pack.module;
                    (
                        quote_spanned!(start.span() => #start),
                        quote_spanned!(expression.span() => #module::MAX),
                        pack,
                    )
                }
                (Some(start), Some(end)) => {
                    let pack = Pack::merge(unpack_expression(start), unpack_expression(end))?;
                    (
                        quote_spanned!(start.span() => #start),
                        pack.limit(end, limits)?,
                        pack,
                    )
                }
            };
            let module = &pack.module;
            let constant = &pack.constant;
            Some(quote_spanned!(expression.span() => {
                #[allow(unused_braces)]
                #[allow(clippy::unnecessary_cast)]
                <::checkito::state::Range::<::checkito::primitive::#module::#constant::<{ #left }>, ::checkito::primitive::#module::#constant::<{ #right }>> as ::checkito::primitive::Constant>::VALUE
            }))
        }
        _ => None,
    }
}

fn unpack_expression(expression: &Expr) -> Option<Pack> {
    match expression {
        Expr::Group(ExprGroup { expr, .. }) => unpack_expression(expr),
        Expr::Const(ExprConst { block, .. }) => unpack_block(block),
        Expr::Block(ExprBlock { block, .. }) => unpack_block(block),
        Expr::Cast(ExprCast { expr, ty, .. }) => {
            let pack = unpack_expression(expr)?;
            let value = match pack.kind {
                Kind::Character(value) => Some(value),
                _ => None,
            };
            unpack_type(ty, value)
        }
        Expr::Lit(ExprLit { lit, .. }) => unpack_literal(lit),
        Expr::Unary(ExprUnary { expr, .. }) => unpack_expression(expr),
        Expr::Binary(ExprBinary { left, right, .. }) => {
            Pack::merge(unpack_expression(left), unpack_expression(right))
        }
        _ => None,
    }
}

fn unpack_block(block: &Block) -> Option<Pack> {
    match block.stmts.last()? {
        Stmt::Expr(expr, None) => unpack_expression(expr),
        _ => None,
    }
}

fn unpack_literal(literal: &Lit) -> Option<Pack> {
    let span = literal.span();
    match literal {
        Lit::Bool(_) => Some(Pack::new("bool", "Bool", span)),
        Lit::Char(value) => Some(Pack::character(value.value(), span)),
        Lit::Byte(_) => Some(Pack::new("u8", "U8", span)),
        Lit::Int(value) if value.suffix().is_empty() => Some(Pack::default(span)),
        Lit::Int(value) => unpack_name(value.suffix(), None, span),
        _ => None,
    }
}

fn unpack_type(type_: &Type, value: Option<char>) -> Option<Pack> {
    match type_ {
        Type::Group(TypeGroup { elem, .. }) => unpack_type(elem, value),
        Type::Paren(TypeParen { elem, .. }) => unpack_type(elem, value),
        Type::Path(TypePath { qself: None, path }) => {
            unpack_name(path.get_ident()?.to_string().as_str(), value, path.span())
        }
        _ => None,
    }
}

fn unpack_name(name: &str, value: Option<char>, span: Span) -> Option<Pack> {
    match name {
        "bool" => Some(Pack::new("bool", "Bool", span)),
        "char" => Some(Pack::character(value?, span)),
        "u8" => Some(Pack::new("u8", "U8", span)),
        "u16" => Some(Pack::new("u16", "U16", span)),
        "u32" => Some(Pack::new("u32", "U32", span)),
        "u64" => Some(Pack::new("u64", "U64", span)),
        "u128" => Some(Pack::new("u128", "U128", span)),
        "usize" => Some(Pack::new("usize", "Usize", span)),
        "i8" => Some(Pack::new("i8", "I8", span)),
        "i16" => Some(Pack::new("i16", "I16", span)),
        "i32" => Some(Pack::new("i32", "I32", span)),
        "i64" => Some(Pack::new("i64", "I64", span)),
        "i128" => Some(Pack::new("i128", "I128", span)),
        "isize" => Some(Pack::new("isize", "Isize", span)),
        _ => None,
    }
}
