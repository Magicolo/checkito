use crate::{
    collect,
    generate::{Generate, State},
    primitive::character,
    shrink::Shrink,
};
use regex_syntax::{
    hir::{Capture, Class, ClassBytesRange, ClassUnicodeRange, Hir, HirKind, Repetition},
    Parser,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Regex {
    tree: Hir,
    repeats: u32,
}

#[derive(Clone)]
pub enum Shrinker {
    Empty,
    Text(String),
    Range(character::Shrinker),
    All(collect::Shrinker<Shrinker, String>),
}

#[derive(Debug)]
pub struct Error(regex_syntax::Error);

impl Regex {
    pub fn repeats(mut self, repeats: u32) -> Self {
        self.repeats = repeats;
        self
    }
}

impl FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Regex {
            tree: Parser::new().parse(s).map_err(Error)?,
            repeats: 64,
        })
    }
}

impl Generate for Regex {
    type Item = String;
    type Shrink = Shrinker;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        fn next(kind: &HirKind, state: &mut State, repeats: u32) -> Shrinker {
            match kind {
                HirKind::Empty | HirKind::Look(_) => Shrinker::Empty,
                HirKind::Literal(literal) => {
                    Shrinker::Text(String::from_utf8(literal.0.to_vec()).unwrap())
                }
                HirKind::Class(Class::Unicode(class)) if class.ranges().is_empty() => {
                    Shrinker::Empty
                }
                HirKind::Class(Class::Bytes(class)) if class.ranges().is_empty() => Shrinker::Empty,
                HirKind::Class(Class::Unicode(class)) => {
                    Shrinker::Range(class.ranges().any().generate(state).unwrap())
                }
                HirKind::Class(Class::Bytes(class)) => {
                    Shrinker::Range(class.ranges().any().generate(state).unwrap())
                }
                HirKind::Capture(Capture { sub, .. }) => next(sub.kind(), state, repeats),
                HirKind::Concat(hirs) | HirKind::Alternation(hirs) if hirs.is_empty() => {
                    Shrinker::Empty
                }
                HirKind::Concat(hirs) | HirKind::Alternation(hirs) if hirs.len() == 1 => {
                    next(hirs[0].kind(), state, repeats)
                }
                HirKind::Concat(hirs) => Shrinker::All(collect::Shrinker::new(
                    hirs.iter()
                        .map(|hir| next(hir.kind(), state, repeats))
                        .collect(),
                    hirs.len(),
                )),
                HirKind::Alternation(hirs) => next(
                    hirs[state.random().usize(..hirs.len())].kind(),
                    state,
                    repeats,
                ),
                HirKind::Repetition(Repetition { min, max, sub, .. }) => {
                    let low = *min;
                    let high = (*max).unwrap_or(repeats.max(low));
                    let count = (low..=high).generate(state).item();
                    let limit = repeats / (u32::BITS - high.leading_zeros());
                    Shrinker::All(collect::Shrinker::new(
                        Iterator::map(0..count, |_| next(sub.kind(), state, limit)).collect(),
                        low as _,
                    ))
                }
            }
        }
        next(self.tree.kind(), state, self.repeats)
    }
}

impl Generate for ClassUnicodeRange {
    type Item = char;
    type Shrink = character::Shrinker;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        character::Range::char(self.start()..=self.end())
            .unwrap()
            .generate(state)
    }
}

impl Generate for ClassBytesRange {
    type Item = char;
    type Shrink = character::Shrinker;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        character::Range::char(self.start() as char..=self.end() as char)
            .unwrap()
            .generate(state)
    }
}

impl Shrink for Shrinker {
    type Item = String;

    fn item(&self) -> Self::Item {
        fn next(shrink: &Shrinker, buffer: &mut String) {
            match shrink {
                Shrinker::Empty => {}
                Shrinker::Text(text) => buffer.push_str(text),
                Shrinker::Range(shrink) => buffer.push(shrink.item()),
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
            Self::Empty | Self::Text(_) => None,
            Self::Range(shrink) => Some(Self::Range(shrink.shrink()?)),
            Self::All(shrink) => Some(Self::All(shrink.shrink()?)),
        }
    }
}
