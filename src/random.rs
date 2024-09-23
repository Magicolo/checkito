use core::ops::RangeBounds;
use fastrand::Rng;

#[derive(Debug, Clone)]
pub struct Random(Rng);

impl Random {
    pub fn new(seed: Option<u64>) -> Self {
        Self(seed.map_or_else(Rng::new, Rng::with_seed))
    }

    pub fn seed(&self) -> u64 {
        self.0.get_seed()
    }
}

macro_rules! bridge {
    ($type:ident) => {
        impl Random {
            pub fn $type(&mut self) -> $type {
                self.0.$type()
            }
        }
    };
    ($($type:ident),*) => {$(bridge!($type);)*}
}

macro_rules! range {
    ($type:ident) => {
        impl Random {
            pub fn $type<R: RangeBounds<$type>>(&mut self, range: R) -> $type {
                self.0.$type(range)
            }
        }
    };
    ($($type:ident),*) => {$(range!($type);)*}
}

bridge!(f32, f64, bool);
range!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, char);
