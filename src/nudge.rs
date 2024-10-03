pub trait Nudge {
    fn nudge(self, force: Self) -> Self;
}

macro_rules! floating {
    ($t:ty) => {
        impl Nudge for $t {
            #[inline]
            fn nudge(self, force: Self) -> Self {
                if self == 0.0 {
                    force / Self::MAX
                } else if self == -0.0 {
                    force / Self::MIN
                } else {
                    self * (1.0 + Self::EPSILON * force)
                }
            }
        }
    };
    ($($t:ty),*) => { $(floating!($t);)* }
}

floating!(f32, f64);
