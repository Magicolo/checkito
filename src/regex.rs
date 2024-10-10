#![cfg(feature = "regex")]

use crate::{
    all,
    any::{self, Any},
    collect::{self},
    generate::{Generate, State},
    primitive::char,
    shrink::Shrink,
};
use core::{fmt, ops::RangeInclusive};
use regex_syntax::{
    Parser,
    hir::{Capture, Class, ClassBytesRange, ClassUnicodeRange, Hir, HirKind, Repetition},
};
use std::string::FromUtf8Error;

#[derive(Debug, Clone)]
pub enum Regex {
    Empty,
    Text(String),
    Range(RangeInclusive<char>),
    Collect(Box<collect::Collect<Regex, RangeInclusive<usize>, String>>),
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
pub struct Error(Inner);

#[derive(Clone)]
enum Inner {
    Regex(Box<regex_syntax::Error>),
    Utf8(FromUtf8Error),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error").field(&self.0).finish()
    }
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inner::Regex(error) => write!(f, "Regex({error})"),
            Inner::Utf8(error) => write!(f, "Utf8({error})"),
        }
    }
}

impl Regex {
    pub fn new(pattern: &str, repeats: Option<u32>) -> Result<Self, Error> {
        let hir = Parser::new().parse(pattern)?;
        Regex::try_from_hir(hir, repeats.unwrap_or(64))
    }
}

impl From<regex_syntax::Error> for Error {
    fn from(value: regex_syntax::Error) -> Self {
        Error(Inner::Regex(Box::new(value)))
    }
}

impl From<FromUtf8Error> for Error {
    fn from(value: FromUtf8Error) -> Self {
        Error(Inner::Utf8(value))
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

    fn try_from_iter(
        results: impl IntoIterator<Item = Result<Regex, Error>>,
        merge: impl FnOnce(Box<[Regex]>) -> Regex,
    ) -> Result<Regex, Error> {
        let mut trees = Vec::new();
        let mut last = None;
        for result in results {
            match result? {
                Self::Empty => {}
                tree => trees.extend(last.replace(tree)),
            }
        }
        Ok(match last {
            Some(tree) if trees.is_empty() => tree,
            Some(tree) => {
                trees.push(tree);
                merge(trees.into_boxed_slice())
            }
            None => Self::Empty,
        })
    }

    fn try_from_hir(hir: Hir, repeats: u32) -> Result<Self, Error> {
        match hir.into_kind() {
            HirKind::Empty | HirKind::Look(_) => Ok(Self::Empty),
            HirKind::Literal(literal) => Ok(Self::Text(String::from_utf8(literal.0.to_vec())?)),
            HirKind::Capture(Capture { sub, .. }) => Self::try_from_hir(*sub, repeats),
            HirKind::Repetition(Repetition { min, max, sub, .. }) => {
                let tree = Self::try_from_hir(*sub, repeats / 2)?;
                if tree.is_empty() {
                    return Ok(Self::Empty);
                }
                let low = min;
                let high = max.unwrap_or(repeats.max(low));
                if low == 1 && high == 1 {
                    return Ok(tree);
                }
                Ok(Self::Collect(Box::new(collect::Collect::new_with(
                    tree,
                    low as usize..=high as usize,
                    Some(low as _),
                ))))
            }
            HirKind::Class(Class::Unicode(class)) => Self::try_from_iter(
                class.ranges().iter().map(|range| Ok(Self::from(range))),
                |trees| Self::Any(Any(trees)),
            ),
            HirKind::Class(Class::Bytes(class)) => Self::try_from_iter(
                class.ranges().iter().map(|range| Ok(Self::from(range))),
                |trees| Self::Any(Any(trees)),
            ),
            HirKind::Concat(hirs) => Self::try_from_iter(
                hirs.into_iter().map(|hir| Self::try_from_hir(hir, repeats)),
                Self::All,
            ),
            HirKind::Alternation(hirs) => Self::try_from_iter(
                hirs.into_iter().map(|hir| Self::try_from_hir(hir, repeats)),
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
            Regex::Collect(collect) => Shrinker::Collect(collect.as_ref().generate(state)),
            Regex::Any(any) => any.generate(state).0.unwrap_or(Shrinker::Empty),
            Regex::All(all) => Shrinker::All(all.as_ref().generate(state)),
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
