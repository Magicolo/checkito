use crate::{
    collect,
    generate::{Generate, State},
    primitive::{self, character},
    shrink::Shrink,
};
use regex_syntax::{
    hir::{
        Class, ClassBytesRange, ClassUnicodeRange, Group, Hir, HirKind, Literal, Repetition,
        RepetitionKind, RepetitionRange,
    },
    Error, Parser,
};
use std::str::FromStr;

pub struct Regex {
    tree: Hir,
    limit: u32,
}

#[derive(Clone)]
pub enum Shrinker {
    Empty,
    Literal(char),
    Range(character::Shrinker),
    All(collect::Shrinker<Shrinker, String>),
}

impl Regex {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }
}

impl FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Regex {
            tree: Parser::new().parse(s)?,
            limit: 64,
        })
    }
}

impl Generate for Regex {
    type Item = String;
    type Shrink = Shrinker;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        fn next(kind: &HirKind, buffer: &mut String, state: &mut State, limit: u32) -> Shrinker {
            match kind {
                HirKind::Empty | HirKind::Anchor(_) | HirKind::WordBoundary(_) => Shrinker::Empty,
                HirKind::Literal(Literal::Unicode(symbol)) => {
                    buffer.push(*symbol);
                    Shrinker::Literal(*symbol)
                }
                HirKind::Literal(Literal::Byte(symbol)) => {
                    buffer.push(*symbol as char);
                    Shrinker::Literal(*symbol as char)
                }
                HirKind::Class(Class::Unicode(class)) => match class.ranges().any().generate(state)
                {
                    (Some(item), Some(shrink)) => {
                        buffer.push(item);
                        Shrinker::Range(shrink)
                    }
                    _ => Shrinker::Empty,
                },
                HirKind::Class(Class::Bytes(class)) => match class.ranges().any().generate(state) {
                    (Some(item), Some(shrink)) => {
                        buffer.push(item as char);
                        Shrinker::Range(shrink.into())
                    }
                    _ => Shrinker::Empty,
                },
                HirKind::Repetition(Repetition { hir, kind, .. }) => {
                    let (low, high) = match kind {
                        RepetitionKind::ZeroOrOne => (0, 1),
                        RepetitionKind::ZeroOrMore => (0, limit),
                        RepetitionKind::OneOrMore => (1, limit),
                        RepetitionKind::Range(range) => match range {
                            RepetitionRange::Exactly(low) => (*low, *low),
                            RepetitionRange::AtLeast(low) => (*low, low.saturating_add(limit)),
                            RepetitionRange::Bounded(low, high) => (*low, *high),
                        },
                    };
                    if low > high || high == 0 {
                        Shrinker::Empty
                    } else {
                        let (count, _) = (low..=high).size(|size| size.powf(2.0)).generate(state);
                        let limit = limit / (32 - high.leading_zeros());
                        let shrinks =
                            Iterator::map(0..count, |_| next(hir.kind(), buffer, state, limit))
                                .collect();
                        Shrinker::All(collect::Shrinker::new(shrinks, low as _))
                    }
                }
                HirKind::Group(Group { hir, .. }) => next(hir.kind(), buffer, state, limit),
                HirKind::Concat(hirs) => Shrinker::All(collect::Shrinker::new(
                    hirs.iter()
                        .map(|hir| next(hir.kind(), buffer, state, limit))
                        .collect(),
                    hirs.len(),
                )),
                HirKind::Alternation(hirs) => next(
                    hirs[state.random().usize(..hirs.len())].kind(),
                    buffer,
                    state,
                    limit,
                ),
            }
        }

        let mut buffer = String::new();
        let shrink = next(self.tree.kind(), &mut buffer, state, self.limit);
        (buffer, shrink)
    }
}

impl Generate for ClassUnicodeRange {
    type Item = char;
    type Shrink = character::Shrinker;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        (self.start()..=self.end()).generate(state)
    }
}

impl Generate for ClassBytesRange {
    type Item = u8;
    type Shrink = primitive::Shrinker<u8>;

    fn generate(&self, state: &mut State) -> (Self::Item, Self::Shrink) {
        (self.start()..=self.end()).generate(state)
    }
}

impl Shrink for Shrinker {
    type Item = String;

    fn generate(&self) -> Self::Item {
        fn next(shrink: &Shrinker, buffer: &mut String) {
            match shrink {
                Shrinker::Empty => {}
                Shrinker::Literal(symbol) => buffer.push(*symbol),
                Shrinker::Range(shrink) => buffer.push(shrink.generate()),
                Shrinker::All(shrink) => {
                    for shrink in shrink.shrinks() {
                        next(shrink, buffer);
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
            Self::Empty => None,
            Self::Literal(_) => None,
            Self::Range(shrink) => Some(Self::Range(shrink.shrink()?)),
            Self::All(shrink) => Some(Self::All(shrink.shrink()?)),
        }
    }
}
