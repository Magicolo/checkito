use crate::{
    collect,
    generate::{Generate, State},
    primitive::{character, Range},
    shrink::Shrink,
};
use regex_syntax::{
    hir::{Class, Group, HirKind, Literal, Repetition, RepetitionKind, RepetitionRange},
    Error, Parser,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Regex {
    tree: Tree,
    repeats: usize,
}

#[derive(Debug, Clone)]
pub enum Tree {
    Symbol(char),
    Range(Range<char>),
    Repeat(Box<Tree>, usize, Option<usize>),
    Any(Vec<Tree>),
    All(Vec<Tree>),
}

#[derive(Clone)]
pub enum Shrinker {
    Symbol(char),
    Range(character::Shrinker),
    All(collect::Shrinker<Shrinker, String>),
}

impl Regex {
    pub fn new(tree: Tree, repeats: usize) -> Self {
        Self { tree, repeats }
    }

    pub fn repeats(mut self, repeats: usize) -> Self {
        self.repeats = repeats;
        self
    }

    pub const fn tree(&self) -> &Tree {
        &self.tree
    }
}

impl FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn tree(kind: &HirKind) -> Tree {
            match kind {
                HirKind::Empty | HirKind::Anchor(_) | HirKind::WordBoundary(_) => {
                    Tree::All(Vec::new())
                }
                HirKind::Literal(Literal::Unicode(symbol)) => Tree::Symbol(*symbol),
                HirKind::Literal(Literal::Byte(symbol)) => Tree::Symbol(*symbol as char),
                HirKind::Class(Class::Unicode(class)) => Tree::Any(
                    class
                        .ranges()
                        .iter()
                        .map(|class| {
                            if class.start() == class.end() {
                                Tree::Symbol(class.start())
                            } else {
                                Tree::Range(Range::char(class.start()..=class.end()).unwrap())
                            }
                        })
                        .collect(),
                ),
                HirKind::Class(Class::Bytes(class)) => Tree::Any(
                    class
                        .ranges()
                        .iter()
                        .map(|class| {
                            if class.start() == class.end() {
                                Tree::Symbol(class.start() as char)
                            } else {
                                Tree::Range(Range::u8(class.start()..=class.end()).unwrap().into())
                            }
                        })
                        .collect(),
                ),
                HirKind::Repetition(Repetition { hir, kind, .. }) => {
                    let (low, high) = match kind {
                        RepetitionKind::ZeroOrOne => (0, Some(1)),
                        RepetitionKind::ZeroOrMore => (0, None),
                        RepetitionKind::OneOrMore => (1, None),
                        RepetitionKind::Range(range) => match range {
                            RepetitionRange::Exactly(low) => (*low as _, Some(*low as _)),
                            RepetitionRange::AtLeast(low) => (*low as _, None),
                            RepetitionRange::Bounded(low, high) => (*low as _, Some(*high as _)),
                        },
                    };
                    match high {
                        Some(high) if low > high || high == 0 => Tree::All(Vec::new()),
                        Some(1) => tree(hir.kind()),
                        high => Tree::Repeat(Box::new(tree(hir.kind())), low, high),
                    }
                }
                HirKind::Group(Group { hir, .. }) => tree(hir.kind()),
                HirKind::Concat(hirs) => Tree::All(
                    hirs.iter()
                        .flat_map(|hir| match tree(hir.kind()) {
                            Tree::All(trees) => trees,
                            tree => vec![tree],
                        })
                        .collect(),
                ),
                HirKind::Alternation(hirs) if hirs.len() == 0 => Tree::All(vec![]),
                HirKind::Alternation(hirs) => Tree::Any(
                    hirs.iter()
                        .flat_map(|hir| match tree(hir.kind()) {
                            Tree::Any(trees) => trees,
                            tree => vec![tree],
                        })
                        .collect(),
                ),
            }
        }

        Ok(Regex {
            tree: tree(Parser::new().parse(s)?.kind()),
            repeats: 64,
        })
    }
}

impl Generate for Regex {
    type Item = String;
    type Shrink = Shrinker;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        fn next(tree: &Tree, state: &mut State, repeats: usize) -> Shrinker {
            match tree {
                Tree::Symbol(symbol) => Shrinker::Symbol(*symbol),
                Tree::Range(range) => Shrinker::Range(range.generate(state)),
                Tree::Repeat(tree, low, high) => {
                    let low = *low;
                    let high = high.unwrap_or(repeats.max(low));
                    let count = (low..=high).generate(state).item();
                    let limit = repeats / (usize::BITS - high.leading_zeros()) as usize;
                    let shrinks = Iterator::map(0..count, |_| next(tree, state, limit)).collect();
                    Shrinker::All(collect::Shrinker::new(shrinks, low as _))
                }
                Tree::Any(trees) => {
                    next(&trees[state.random().usize(..trees.len())], state, repeats)
                }
                Tree::All(trees) => Shrinker::All(collect::Shrinker::new(
                    trees
                        .iter()
                        .map(|tree| next(tree, state, repeats))
                        .collect(),
                    trees.len(),
                )),
            }
        }

        next(self.tree(), state, self.repeats)
    }
}

impl Shrink for Shrinker {
    type Item = String;

    fn item(&self) -> Self::Item {
        fn next(shrink: &Shrinker, buffer: &mut String) {
            match shrink {
                Shrinker::Symbol(symbol) => buffer.push(*symbol),
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
            Self::Symbol(_) => None,
            Self::Range(shrink) => Some(Self::Range(shrink.shrink()?)),
            Self::All(shrink) => Some(Self::All(shrink.shrink()?)),
        }
    }
}
