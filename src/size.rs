use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Default)]
pub struct Size<T: ?Sized>(pub T);

impl<T: ?Sized> Deref for Size<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for Size<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
