#![cfg(feature = "regex")]

use crate::{
    REPEATS, all,
    any::{self, Any},
    collect::{self},
    generate::{Generate, State},
    prelude::collect,
    primitive::char,
    shrink::Shrink,
};
use core::{fmt, ops::RangeInclusive};
use regex_syntax::{
    Parser,
    hir::{Capture, Class, ClassBytesRange, ClassUnicodeRange, Hir, HirKind, Repetition},
};

#[derive(Debug, Clone)]
pub enum Regex {
    Empty,
    Text(String),
    Range(RangeInclusive<char>),
    Collect(collect::Collect<Box<Regex>, RangeInclusive<usize>, String>),
    Any(any::Any<Box<[Regex]>>),
    All(Box<[Regex]>),
}

#[derive(Debug, Clone)]
pub enum Shrinker {
    Empty,
    Text(String),
    Range(char::Shrinker),
    All(all::Shrinker<Box<[Shrinker]>>),
    Collect(collect::Shrinker<Shrinker, String>),
}

#[derive(Clone)]
pub struct Error(Box<regex_syntax::Error>);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error").field(&self.0).finish()
    }
}

impl Regex {
    pub(crate) fn new(pattern: &str, repeats: Option<u32>) -> Result<Self, Error> {
        let hir = Parser::new().parse(pattern)?;
        Ok(Regex::from_hir(hir, repeats.unwrap_or(REPEATS)))
    }
}

impl From<regex_syntax::Error> for Error {
    fn from(value: regex_syntax::Error) -> Self {
        Error(Box::new(value))
    }
}

impl From<&ClassUnicodeRange> for Regex {
    fn from(value: &ClassUnicodeRange) -> Self {
        Regex::Range(value.start()..=value.end())
    }
}

impl From<&ClassBytesRange> for Regex {
    fn from(value: &ClassBytesRange) -> Self {
        Regex::Range(value.start() as char..=value.end() as char)
    }
}

impl Regex {
    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    fn from_iter(
        trees: impl IntoIterator<Item = Regex>,
        merge: impl FnOnce(Box<[Regex]>) -> Regex,
    ) -> Regex {
        let mut buffer = Vec::new();
        let mut last = None;
        for tree in trees {
            if !tree.is_empty() {
                buffer.extend(last.replace(tree));
            }
        }
        match last {
            Some(tree) if buffer.is_empty() => tree,
            Some(tree) => {
                buffer.push(tree);
                merge(buffer.into_boxed_slice())
            }
            None => Self::Empty,
        }
    }

    fn from_hir(hir: Hir, repeats: u32) -> Self {
        match hir.into_kind() {
            HirKind::Empty | HirKind::Look(_) => Self::Empty,
            HirKind::Literal(literal) => {
                String::from_utf8(literal.0.to_vec()).map_or(Self::Empty, Self::Text)
            }
            HirKind::Capture(Capture { sub, .. }) => Self::from_hir(*sub, repeats),
            HirKind::Repetition(Repetition { min, max, sub, .. }) => {
                let tree = Self::from_hir(*sub, repeats / 2);
                if tree.is_empty() {
                    return Self::Empty;
                }
                let low = min;
                let high = max.unwrap_or(repeats.max(low));
                if low == 1 && high == 1 {
                    return tree;
                }
                Self::Collect(collect(
                    Box::new(tree),
                    low as usize..=high as usize,
                    Some(low as _),
                ))
            }
            HirKind::Class(Class::Unicode(class)) => {
                Self::from_iter(class.ranges().iter().map(Self::from), |trees| {
                    Self::Any(Any(trees))
                })
            }
            HirKind::Class(Class::Bytes(class)) => {
                Self::from_iter(class.ranges().iter().map(Self::from), |trees| {
                    Self::Any(Any(trees))
                })
            }
            HirKind::Concat(hirs) => Self::from_iter(
                hirs.into_iter().map(|hir| Self::from_hir(hir, repeats)),
                Self::All,
            ),
            HirKind::Alternation(hirs) => Self::from_iter(
                hirs.into_iter().map(|hir| Self::from_hir(hir, repeats)),
                |trees| Self::Any(Any(trees)),
            ),
        }
    }
}

impl Generate for Regex {
    type Item = String;
    type Shrink = Shrinker;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        match self {
            Regex::Empty => Shrinker::Empty,
            Regex::Text(text) => Shrinker::Text(text.clone()),
            Regex::Range(range) => Shrinker::Range(range.generate(state)),
            Regex::Collect(collect) => Shrinker::Collect(collect.generate(state)),
            Regex::Any(any) => any.generate(state).0.unwrap_or(Shrinker::Empty),
            Regex::All(all) => Shrinker::All(all.generate(state)),
        }
    }

    fn constant(&self) -> bool {
        match self {
            Regex::Empty | Regex::Text(_) => true,
            Regex::Range(range) => range.constant(),
            Regex::Collect(collect) => collect.constant(),
            Regex::Any(any) => any.constant(),
            Regex::All(all) => all.constant(),
        }
    }
}

impl Shrink for Shrinker {
    type Item = String;

    fn item(&self) -> Self::Item {
        fn descend(shrinker: &Shrinker, buffer: &mut String) {
            match shrinker {
                Shrinker::Empty => {}
                Shrinker::Text(text) => buffer.push_str(text),
                Shrinker::Range(shrinker) => buffer.push(shrinker.item()),
                Shrinker::All(shrinker) => {
                    for shrinker in shrinker.shrinkers.iter() {
                        descend(shrinker, buffer);
                    }
                }
                Shrinker::Collect(shrinker) => {
                    for shrinker in shrinker.shrinkers.iter() {
                        descend(shrinker, buffer);
                    }
                }
            }
        }

        let mut buffer = String::new();
        descend(self, &mut buffer);
        buffer
    }

    fn shrink(&mut self) -> Option<Self> {
        match self {
            Self::Empty | Self::Text(_) => None,
            Self::Range(shrinker) => Some(Self::Range(shrinker.shrink()?)),
            Self::All(shrinker) => Some(Self::All(shrinker.shrink()?)),
            Self::Collect(shrinker) => Some(Self::Collect(shrinker.shrink()?)),
        }
    }
}
