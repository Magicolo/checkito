use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use core::any::Any;

#[derive(Debug)]
pub struct Boxed<I> {
    generator: Box<dyn Any>,
    generate: fn(&dyn Any, &mut State) -> Shrinker<I>,
    constant: fn(&dyn Any) -> bool,
}

#[derive(Debug)]
pub struct Shrinker<I> {
    shrinker: Box<dyn Any>,
    clone: fn(&dyn Any) -> Box<dyn Any>,
    item: fn(&dyn Any) -> I,
    shrink: fn(&mut dyn Any) -> Option<Box<dyn Any>>,
}

impl<I> Generate for Boxed<I> {
    type Item = I;
    type Shrink = Shrinker<I>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        (self.generate)(self.generator.as_ref(), state)
    }

    fn constant(&self) -> bool {
        (self.constant)(self.generator.as_ref())
    }
}

impl<I> Boxed<I> {
    pub(crate) const fn new<G: Generate<Item = I> + 'static>(generator: Box<G>) -> Self
    where
        G::Shrink: 'static,
    {
        Self {
            generator,
            generate: |generator, state| {
                generator
                    .downcast_ref::<G>()
                    .unwrap()
                    .generate(state)
                    .boxed()
            },
            constant: |generator| generator.downcast_ref::<G>().unwrap().constant(),
        }
    }
}

impl<I> Boxed<I> {
    pub fn downcast<G: Generate + 'static>(self) -> Result<G, Self> {
        match self.generator.downcast::<G>() {
            Ok(generator) => Ok(*generator),
            Err(generator) => Err(Self {
                generator,
                generate: self.generate,
                constant: self.constant,
            }),
        }
    }
}

impl Shrinker<()> {
    pub(crate) fn new<S: Shrink + 'static>(shrinker: S) -> Shrinker<S::Item> {
        Shrinker {
            shrinker: Box::new(shrinker),
            clone: |inner| Box::new(inner.downcast_ref::<S>().unwrap().clone()),
            item: |inner| inner.downcast_ref::<S>().unwrap().item(),
            shrink: |inner| Some(Box::new(inner.downcast_mut::<S>().unwrap().shrink()?)),
        }
    }

    pub fn downcast<S: Shrink + 'static>(self) -> Result<S, Self> {
        match self.shrinker.downcast::<S>() {
            Ok(shrinker) => Ok(*shrinker),
            Err(shrinker) => Err(Self {
                shrinker,
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
            shrinker: (self.clone)(self.shrinker.as_ref()),
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        }
    }
}

impl<I> Shrink for Shrinker<I> {
    type Item = I;

    fn item(&self) -> Self::Item {
        (self.item)(self.shrinker.as_ref())
    }

    fn shrink(&mut self) -> Option<Self> {
        Some(Self {
            shrinker: (self.shrink)(self.shrinker.as_mut())?,
            clone: self.clone,
            item: self.item,
            shrink: self.shrink,
        })
    }
}
