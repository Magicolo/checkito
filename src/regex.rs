#![cfg(feature = "regex")]

use crate::{
    collect,
    generate::{Generator, State},
    primitive::character,
    shrink::Shrinker,
};
use core::fmt;
use regex_syntax::{
    Parser,
    hir::{Capture, Class, ClassBytesRange, ClassUnicodeRange, Hir, HirKind, Repetition},
};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Regex {
    pattern: Cow<'static, str>,
    tree: Hir,
    repeats: u32,
}

#[derive(Clone)]
pub enum Shrink {
    Empty,
    Text(String),
    Range(character::Shrink),
    All(collect::Shrink<Shrink, String>),
}

#[derive(Clone)]
pub struct Error(Box<regex_syntax::Error>);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error").field(&self.0).finish()
    }
}

impl Regex {
    pub fn new(pattern: impl Into<Cow<'static, str>>) -> Result<Self, Error> {
        let pattern = pattern.into();
        Ok(Regex {
            tree: Parser::new()
                .parse(&pattern)
                .map_err(|error| Error(Box::new(error)))?,
            pattern,
            repeats: 64,
        })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn repeats(mut self, repeats: u32) -> Self {
        self.repeats = repeats;
        self
    }
}

impl Generator for Regex {
    type Item = String;
    type Shrink = Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        fn next(kind: &HirKind, state: &mut State, repeats: u32) -> Shrink {
            match kind {
                HirKind::Empty | HirKind::Look(_) => Shrink::Empty,
                HirKind::Literal(literal) => {
                    Shrink::Text(String::from_utf8(literal.0.to_vec()).unwrap())
                }
                HirKind::Class(Class::Unicode(class)) if class.ranges().is_empty() => Shrink::Empty,
                HirKind::Class(Class::Bytes(class)) if class.ranges().is_empty() => Shrink::Empty,
                HirKind::Class(Class::Unicode(class)) => {
                    Shrink::Range(class.ranges().any().generate(state).unwrap())
                }
                HirKind::Class(Class::Bytes(class)) => {
                    Shrink::Range(class.ranges().any().generate(state).unwrap())
                }
                HirKind::Capture(Capture { sub, .. }) => next(sub.kind(), state, repeats),
                HirKind::Concat(hirs) | HirKind::Alternation(hirs) if hirs.is_empty() => {
                    Shrink::Empty
                }
                HirKind::Concat(hirs) | HirKind::Alternation(hirs) if hirs.len() == 1 => {
                    next(hirs[0].kind(), state, repeats)
                }
                HirKind::Concat(hirs) => Shrink::All(collect::Shrink::new(
                    hirs.iter().map(|hir| next(hir.kind(), state, repeats)),
                    Some(hirs.len()),
                )),
                HirKind::Alternation(hirs) => next(
                    hirs[state.random().usize(..hirs.len())].kind(),
                    state,
                    repeats,
                ),
                HirKind::Repetition(Repetition { min, max, sub, .. }) => {
                    let (low, high) = range(*min, *max, repeats);
                    let count = (low..=high).generate(state).item();
                    let limit = repeats / (u32::BITS - high.leading_zeros());
                    Shrink::All(collect::Shrink::new(
                        Iterator::map(0..count, |_| next(sub.kind(), state, limit)),
                        Some(low as _),
                    ))
                }
            }
        }
        next(self.tree.kind(), state, self.repeats)
    }

    fn constant(&self) -> bool {
        fn next(kind: &HirKind, repeats: u32) -> bool {
            match kind {
                HirKind::Empty | HirKind::Literal(_) | HirKind::Look(_) => true,
                HirKind::Class(Class::Unicode(class)) => {
                    class.ranges().iter().all(Generator::constant)
                }
                HirKind::Class(Class::Bytes(class)) => {
                    class.ranges().iter().all(Generator::constant)
                }
                HirKind::Capture(Capture { sub, .. }) => next(sub.kind(), repeats),
                HirKind::Concat(hirs) => hirs.iter().all(|hir| next(hir.kind(), repeats)),
                HirKind::Alternation(hirs) => hirs.iter().all(|hir| next(hir.kind(), repeats)),
                HirKind::Repetition(Repetition { min, max, sub, .. }) => {
                    let (low, high) = range(*min, *max, repeats);
                    if low == 0 && high == 0 {
                        true
                    } else {
                        (low..=high).constant() && next(sub.kind(), repeats)
                    }
                }
            }
        }
        next(self.tree.kind(), self.repeats)
    }
}

fn range(min: u32, max: Option<u32>, repeats: u32) -> (u32, u32) {
    let low = min;
    let high = max.unwrap_or(repeats.max(low));
    (low, high)
}

impl Generator for ClassUnicodeRange {
    type Item = char;
    type Shrink = character::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        character::Range::char(self.start()..=self.end())
            .unwrap()
            .generate(state)
    }

    fn constant(&self) -> bool {
        character::Range::char(self.start()..=self.end())
            .unwrap()
            .constant()
    }
}

impl Generator for ClassBytesRange {
    type Item = char;
    type Shrink = character::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        character::Range::char(self.start() as char..=self.end() as char)
            .unwrap()
            .generate(state)
    }

    fn constant(&self) -> bool {
        character::Range::char(self.start() as char..=self.end() as char)
            .unwrap()
            .constant()
    }
}

impl Shrinker for Shrink {
    type Item = String;

    fn item(&self) -> Self::Item {
        fn next(shrinker: &Shrink, buffer: &mut String) {
            match shrinker {
                Shrink::Empty => {}
                Shrink::Text(text) => buffer.push_str(text),
                Shrink::Range(shrinker) => buffer.push(shrinker.item()),
                Shrink::All(shrinker) => {
                    for shrinker in shrinker.shrinkers() {
                        next(shrinker, buffer);
                    }
                }
            }
        }

        let mut buffer = String::new();
        next(self, &mut buffer);
        buffer
    }

    fn shrink(&mut self) -> Option<Self> {
        match self {
            Self::Empty | Self::Text(_) => None,
            Self::Range(shrinker) => Some(Self::Range(shrinker.shrink()?)),
            Self::All(shrinker) => Some(Self::All(shrinker.shrink()?)),
        }
    }
}
