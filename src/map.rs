use crate::{
    generate::{Generator, State},
    shrink::Shrinker,
};

#[derive(Debug, Default, Clone)]
pub struct Map<I: ?Sized, F> {
    map: F,
    generator: I,
}

#[derive(Debug, Clone)]
pub struct Shrink<I: ?Sized, F> {
    map: F,
    shrinker: I,
}

impl<G: Generator, T, F: Fn(G::Item) -> T> Map<G, F> {
    pub const fn new(generator: G, map: F) -> Self {
        Self { generator, map }
    }
}

impl<S: Shrinker, T, F: Fn(S::Item) -> T> Shrink<S, F> {
    pub const fn new(shrinker: S, map: F) -> Self {
        Self { shrinker, map }
    }
}

impl<G: Generator + ?Sized, T, F: Fn(G::Item) -> T + Clone> Generator for Map<G, F> {
    type Item = T;
    type Shrink = Shrink<G::Shrink, F>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Shrink::new(self.generator.generate(state), self.map.clone())
    }

    fn constant(&self) -> bool {
        self.generator.constant()
    }
}

impl<S: Shrinker, T, F: Fn(S::Item) -> T + Clone> Shrinker for Shrink<S, F> {
    type Item = T;

    fn item(&self) -> Self::Item {
        (self.map)(self.shrinker.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self::new(self.shrinker.shrink()?, self.map.clone()))
    }
}
