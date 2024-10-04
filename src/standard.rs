use crate::{
    FullGenerator, IntoGenerator,
    any::Any,
    generate::{Generator, State},
    map::Map,
    shrink::Shrinker,
};
use orn::Or2;
use std::{rc::Rc, sync::Arc};

impl<G: FullGenerator> FullGenerator for Option<G> {
    type FullGen =
        Map<Any<(G::FullGen, ())>, fn(<Any<(G::FullGen, ())> as Generator>::Item) -> Self::Item>;
    type Item = Option<G::Item>;

    fn full_gen() -> Self::FullGen {
        Any((G::full_gen(), ())).map(|item| match item {
            Or2::T0(item) => Some(item),
            Or2::T1(_) => None,
        })
    }
}

impl<G: Generator> Generator for Option<G> {
    type Item = Option<G::Item>;
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Some(self.as_ref()?.generate(state))
    }

    fn constant(&self) -> bool {
        self.as_ref().map_or(true, G::constant)
    }
}

impl<S: Shrinker> Shrinker for Option<S> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        Some(self.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Some(self.as_mut()?.shrink()?))
    }
}

impl<G: FullGenerator, E: FullGenerator> FullGenerator for Result<G, E> {
    type FullGen = Map<
        Any<(G::FullGen, E::FullGen)>,
        fn(<Any<(G::FullGen, E::FullGen)> as Generator>::Item) -> Self::Item,
    >;
    type Item = Result<G::Item, E::Item>;

    fn full_gen() -> Self::FullGen {
        Any((G::full_gen(), E::full_gen())).map(|item| match item {
            Or2::T0(item) => Result::Ok(item),
            Or2::T1(item) => Result::Err(item),
        })
    }
}

impl<G: Generator, E: Generator> Generator for Result<G, E> {
    type Item = Result<G::Item, E::Item>;
    type Shrink = Result<G::Shrink, E::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        match self {
            Ok(generator) => Ok(generator.generate(state)),
            Err(generator) => Err(generator.generate(state)),
        }
    }

    fn constant(&self) -> bool {
        match self {
            Ok(generator) => generator.constant(),
            Err(generator) => generator.constant(),
        }
    }
}

impl<S: Shrinker, E: Shrinker> Shrinker for Result<S, E> {
    type Item = Result<S::Item, E::Item>;

    fn item(&self) -> Self::Item {
        match self {
            Ok(shrinker) => Ok(shrinker.item()),
            Err(shrinker) => Err(shrinker.item()),
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        match self {
            Ok(shrinker) => Some(Ok(shrinker.shrink()?)),
            Err(shrinker) => Some(Err(shrinker.shrink()?)),
        }
    }
}

impl<G: FullGenerator> FullGenerator for Box<G> {
    type FullGen = G::FullGen;
    type Item = G::Item;

    fn full_gen() -> Self::FullGen {
        G::full_gen()
    }
}

impl<G: IntoGenerator> IntoGenerator for Box<G> {
    type IntoGen = G::IntoGen;
    type Item = G::Item;

    fn into_gen(self) -> Self::IntoGen {
        G::into_gen(*self)
    }
}

impl<G: Generator> Generator for Box<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: FullGenerator> FullGenerator for Rc<G> {
    type FullGen = G::FullGen;
    type Item = G::Item;

    fn full_gen() -> Self::FullGen {
        G::full_gen()
    }
}

impl<G: Generator> Generator for Rc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}

impl<G: FullGenerator> FullGenerator for Arc<G> {
    type FullGen = G::FullGen;
    type Item = G::Item;

    fn full_gen() -> Self::FullGen {
        G::full_gen()
    }
}

impl<G: Generator> Generator for Arc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }

    fn constant(&self) -> bool {
        G::constant(self)
    }
}
