use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy, Debug, Default)]
pub struct Size<T: ?Sized>(pub T);

impl<T: ?Sized> Deref for Size<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for Size<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
