use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use core::{any::Any, fmt};

pub struct Boxed<I> {
    generator: Box<dyn Any>,
    generate: fn(&dyn Any, &mut State) -> Shrinker<I>,
    constant: fn(&dyn Any) -> bool,
}

pub struct Shrinker<I> {
    shrinker: Box<dyn Any>,
    clone: fn(&dyn Any) -> Box<dyn Any>,
    item: fn(&dyn Any) -> I,
    shrink: fn(&mut dyn Any) -> Option<Box<dyn Any>>,
}

impl<I> fmt::Debug for Boxed<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Boxed").field(&self.generator).finish()
    }
}

impl<I> fmt::Debug for Shrinker<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Shrinker").field(&self.shrinker).finish()
    }
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
    pub(crate) fn new<G: Generate<Item = I> + 'static>(generator: Box<G>) -> Self
    where
        G::Shrink: 'static,
    {
        Self {
            generator,
            generate: generate::<G>,
            constant: constant::<G>,
        }
    }

    pub fn downcast<G: Generate + 'static>(self) -> Result<Box<G>, Self> {
        match self.generator.downcast::<G>() {
            Ok(generator) => Ok(generator),
            Err(generator) => Err(Self {
                generator,
                generate: self.generate,
                constant: self.constant,
            }),
        }
    }
}

impl<I> Shrinker<I> {
    pub(crate) fn new<S: Shrink<Item = I> + 'static>(shrinker: Box<S>) -> Self {
        Self {
            shrinker,
            clone: clone::<S>,
            item: item::<S>,
            shrink: shrink::<S>,
        }
    }

    pub fn downcast<S: Shrink + 'static>(self) -> Result<Box<S>, Self> {
        match self.shrinker.downcast::<S>() {
            Ok(shrinker) => Ok(shrinker),
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

fn generate<G: Generate + 'static>(generator: &dyn Any, state: &mut State) -> Shrinker<G::Item>
where
    G::Shrink: 'static,
{
    Shrinker::new(Box::new(
        generator.downcast_ref::<G>().unwrap().generate(state),
    ))
}

fn constant<G: Generate + 'static>(generator: &dyn Any) -> bool {
    generator.downcast_ref::<G>().unwrap().constant()
}

fn clone<S: Shrink + 'static>(shrinker: &dyn Any) -> Box<dyn Any> {
    Box::new(shrinker.downcast_ref::<S>().unwrap().clone())
}

fn item<S: Shrink + 'static>(shrinker: &dyn Any) -> S::Item {
    shrinker.downcast_ref::<S>().unwrap().item()
}

fn shrink<S: Shrink + 'static>(shrinker: &mut dyn Any) -> Option<Box<dyn Any>> {
    Some(Box::new(shrinker.downcast_mut::<S>().unwrap().shrink()?))
}
