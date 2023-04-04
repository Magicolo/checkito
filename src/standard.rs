use std::{rc::Rc, sync::Arc};

use crate::{
    any::{tuples2::One, Any},
    generate::{Generate, State},
    map::Map,
    shrink::Shrink,
    FullGenerate,
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

impl<G: FullGenerate> FullGenerate for Rc<G> {
    type Item = Rc<G::Item>;
    type Generate = Map<G::Generate, Self::Item>;

    fn generator() -> Self::Generate {
        let new: fn(G::Item) -> Self::Item = Rc::new;
        G::generator().map(new)
    }
}

impl<G: FullGenerate> FullGenerate for Arc<G> {
    type Item = Arc<G::Item>;
    type Generate = Map<G::Generate, Self::Item>;

    fn generator() -> Self::Generate {
        let new: fn(G::Item) -> Self::Item = Arc::new;
        G::generator().map(new)
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
