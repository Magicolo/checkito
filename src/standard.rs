use std::{rc::Rc, sync::Arc};

use crate::{
    any::{tuples2::One, Any},
    generate::{Generate, State},
    map::Map,
    shrink::{FullShrink, IntoShrink, Shrink},
    FullGenerate, IntoGenerate,
};

impl<G: FullGenerate> FullGenerate for Option<G> {
    type Item = Option<G::Item>;
    type Generate = Map<Any<(G::Generate, ())>, Self::Item>;

    fn generator() -> Self::Generate {
        Any((G::generator(), ())).map(|item| match item {
            One::T0(item) => Some(item),
            One::T1(_) => None,
        })
    }
}

impl<G: Generate> Generate for Option<G> {
    type Item = Option<G::Item>;
    type Shrink = Option<G::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        Some(self.as_ref()?.generate(state))
    }
}

impl<S: FullShrink> FullShrink for Option<S> {
    type Item = Option<S::Item>;
    type Shrink = Option<S::Shrink>;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        Some(S::shrinker(item?))
    }
}

impl<S: IntoShrink> IntoShrink for Option<S> {
    type Item = Option<S::Item>;
    type Shrink = Option<S::Shrink>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        Some(self.as_ref()?.shrinker(item?))
    }
}

impl<S: Shrink> Shrink for Option<S> {
    type Item = Option<S::Item>;

    fn item(&self) -> Self::Item {
        Some(self.as_ref()?.item())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Some(self.as_mut()?.shrink()?))
    }
}

impl<G: FullGenerate, E: FullGenerate> FullGenerate for Result<G, E> {
    type Item = Result<G::Item, E::Item>;
    type Generate = Map<Any<(G::Generate, E::Generate)>, Self::Item>;

    fn generator() -> Self::Generate {
        Any((G::generator(), E::generator())).map(|item| match item {
            One::T0(item) => Result::Ok(item),
            One::T1(item) => Result::Err(item),
        })
    }
}

impl<G: Generate, E: Generate> Generate for Result<G, E> {
    type Item = Result<G::Item, E::Item>;
    type Shrink = Result<G::Shrink, E::Shrink>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        match self {
            Ok(generate) => Ok(generate.generate(state)),
            Err(generate) => Err(generate.generate(state)),
        }
    }
}

impl<S: FullShrink, E: FullShrink> FullShrink for Result<S, E> {
    type Item = Result<S::Item, E::Item>;
    type Shrink = Result<S::Shrink, E::Shrink>;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        match item {
            Ok(item) => Some(Ok(S::shrinker(item)?)),
            Err(item) => Some(Err(E::shrinker(item)?)),
        }
    }
}

impl<S: IntoShrink, E: IntoShrink> IntoShrink for Result<S, E> {
    type Item = Result<S::Item, E::Item>;
    type Shrink = Result<S::Shrink, E::Shrink>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        match (self, item) {
            (Ok(shrink), Ok(item)) => Some(Ok(shrink.shrinker(item)?)),
            (Err(shrink), Err(item)) => Some(Err(shrink.shrinker(item)?)),
            _ => None,
        }
    }
}

impl<S: Shrink, E: Shrink> Shrink for Result<S, E> {
    type Item = Result<S::Item, E::Item>;

    fn item(&self) -> Self::Item {
        match self {
            Ok(shrink) => Ok(shrink.item()),
            Err(shrink) => Err(shrink.item()),
        }
    }

    fn shrink(&mut self) -> Option<Self> {
        match self {
            Ok(shrink) => Some(Ok(shrink.shrink()?)),
            Err(shrink) => Some(Err(shrink.shrink()?)),
        }
    }
}

impl<G: FullGenerate> FullGenerate for Box<G> {
    type Item = G::Item;
    type Generate = G::Generate;

    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: IntoGenerate> IntoGenerate for Box<G> {
    type Item = G::Item;
    type Generate = G::Generate;

    fn generator(self) -> Self::Generate {
        G::generator(*self)
    }
}

impl<G: Generate> Generate for Box<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }
}

impl<G: FullGenerate> FullGenerate for Rc<G> {
    type Item = G::Item;
    type Generate = G::Generate;

    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: Generate> Generate for Rc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }
}

impl<G: FullGenerate> FullGenerate for Arc<G> {
    type Item = G::Item;
    type Generate = G::Generate;

    fn generator() -> Self::Generate {
        G::generator()
    }
}

impl<G: Generate> Generate for Arc<G> {
    type Item = G::Item;
    type Shrink = G::Shrink;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        G::generate(self, state)
    }
}

impl<T> Generate for fn() -> T {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> Self::Shrink {
        *self
    }
}

impl<T> Shrink for fn() -> T {
    type Item = T;

    fn item(&self) -> Self::Item {
        self()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
    }
}
