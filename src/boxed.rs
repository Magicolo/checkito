use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use core::any::Any;

pub struct Generator<I> {
    inner: Box<dyn Any>,
    generate: fn(&dyn Any, &mut State) -> Shrinker<I>,
    constant: fn(&dyn Any) -> bool,
}

pub struct Shrinker<I> {
    inner: Box<dyn Any>,
    clone: fn(&dyn Any) -> Box<dyn Any>,
    item: fn(&dyn Any) -> I,
    shrink: fn(&mut dyn Any) -> Option<Box<dyn Any>>,
}

impl<I> Generate for Generator<I> {
    type Item = I;
    type Shrink = Shrinker<I>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        (self.generate)(self.inner.as_ref(), state)
    }

    fn constant(&self) -> bool {
        (self.constant)(self.inner.as_ref())
    }
}

impl<I> Generator<I> {
    pub(crate) fn new<G: Generate<Item = I> + 'static>(generate: G) -> Self
    where
        G::Shrink: 'static,
    {
        Self {
            inner: Box::new(generate),
            generate: |inner, state| inner.downcast_ref::<G>().unwrap().generate(state).boxed(),
            constant: |inner| inner.downcast_ref::<G>().unwrap().constant(),
        }
    }
}

impl<I> Generator<I> {
    pub fn downcast<G: Generate + 'static>(self) -> Result<G, Self> {
        match self.inner.downcast::<G>() {
            Ok(inner) => Ok(*inner),
            Err(inner) => Err(Self {
                inner,
                generate: self.generate,
                constant: self.constant,
            }),
        }
    }
}

impl Shrinker<()> {
    pub(crate) fn new<S: Shrink + 'static>(shrink: S) -> Shrinker<S::Item> {
        Shrinker {
            inner: Box::new(shrink),
            clone: |inner| Box::new(inner.downcast_ref::<S>().unwrap().clone()),
            item: |inner| inner.downcast_ref::<S>().unwrap().item(),
            shrink: |inner| Some(Box::new(inner.downcast_mut::<S>().unwrap().shrink()?)),
        }
    }

    pub fn downcast<G: Shrink + 'static>(self) -> Result<G, Self> {
        match self.inner.downcast::<G>() {
            Ok(inner) => Ok(*inner),
            Err(inner) => Err(Self {
                inner,
                clone: self.clone,
                item: self.item,
                shrink: self.shrink,
            }),
        }
    }
}

impl<I> Clone for Shrinker<I> {
    fn clone(&self) -> Self {
        Self {
            inner: (self.clone)(self.inner.as_ref()),
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        }
    }
}

impl<I> Shrink for Shrinker<I> {
    type Item = I;

    fn item(&self) -> Self::Item {
        (self.item)(self.inner.as_ref())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: (self.shrink)(self.inner.as_mut())?,
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        })
    }
}
