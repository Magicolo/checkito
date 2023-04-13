use crate::{
    generate::{Generate, State},
    shrink::Shrink,
};
use std::marker::PhantomData;

pub struct Function<T, F>(F, PhantomData<T>);

impl<T, F> Function<T, F> {
    pub const fn new(generate: F) -> Self {
        Self(generate, PhantomData)
    }
}

impl<T, F: Clone> Clone for Function<T, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, F: Fn() -> T + Clone> Generate for Function<T, F> {
    type Item = T;
    type Shrink = Self;

    fn generate(&self, _: &mut State) -> Self::Shrink {
        self.clone()
    }
}

impl<T, F: Fn() -> T + Clone> Shrink for Function<T, F> {
    type Item = T;

    fn item(&self) -> Self::Item {
        self.0()
    }

    fn shrink(&mut self) -> Option<Self> {
        None
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
