use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use std::any::Any;

pub struct Generator<I> {
    inner: Box<dyn Any>,
    generate: fn(&dyn Any, &mut State) -> Shrinker<I>,
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
        (self.generate)(&*self.inner, state)
    }
}

impl Generator<()> {
    pub(crate) fn new<G: Generate + 'static>(generate: G) -> Generator<G::Item>
    where
        G::Shrink: 'static,
    {
        Generator {
            inner: Box::new(generate),
            generate: |inner, state| inner.downcast_ref::<G>().unwrap().generate(state).boxed(),
        }
    }
}

impl<I> Generator<I> {
    pub fn cast<G: Generate + 'static>(self) -> Result<G, Self> {
        match self.inner.downcast::<G>() {
            Ok(inner) => Ok(*inner),
            Err(inner) => Err(Self {
                inner,
                generate: self.generate,
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

    pub fn cast<G: Shrink + 'static>(self) -> Result<G, Self> {
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
            inner: (self.clone)(&*self.inner),
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        }
    }
}

impl<I> Shrink for Shrinker<I> {
    type Item = I;

    fn item(&self) -> Self::Item {
        (self.item)(&*self.inner)
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            inner: (self.shrink)(&mut *self.inner)?,
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        })
    }
}
