use std::mem::MaybeUninit;

#[macro_export]
macro_rules! count {
    () => { 0 };
    ($v:ident $(,$vs:ident)*) => {1 + $crate::count!($($vs),*) };
}

#[macro_export]
macro_rules! tuples {
    ($m:ident) => {
        $m!(tuples0, 0);
        $m!(tuples1, 1, p0, T0, 0);
        $m!(tuples2, 2, p0, T0, 0, p1, T1, 1);
        $m!(tuples3, 3, p0, T0, 0, p1, T1, 1, p2, T2, 2);
        $m!(tuples4, 4, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3);
        $m!(tuples5, 5, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4);
        $m!(tuples6, 6, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5);
        $m!(
            tuples7, 7, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6
        );
        $m!(
            tuples8, 8, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7
        );
        $m!(
            tuples9, 9, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8
        );
        $m!(
            tuples10, 10, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9
        );
        $m!(
            tuples11, 11, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10
        );
        $m!(
            tuples12, 12, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11
        );
        $m!(
            tuples13, 13, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12
        );
        $m!(
            tuples14, 14, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13
        );
        $m!(
            tuples15, 15, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14
        );
        $m!(
            tuples16, 16, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15
        );
    };
}

pub trait Nudge {
    fn nudge(self, force: Self) -> Self;
}

impl Nudge for f32 {
    #[inline]
    fn nudge(self, force: Self) -> Self {
        if self == 0.0 {
            1.0 / Self::MAX
        } else if self == -0.0 {
            1.0 / Self::MIN
        } else {
            self * (1.0 + Self::EPSILON * force)
        }
    }
}

impl Nudge for f64 {
    #[inline]
    fn nudge(self, force: Self) -> Self {
        if self == 0.0 {
            1.0 / Self::MAX
        } else if self == -0.0 {
            1.0 / Self::MIN
        } else {
            self * (1.0 + Self::EPSILON * force)
        }
    }
}

pub trait Unzip {
    type Target;
    fn unzip(self) -> Self::Target;
}

macro_rules! unzip {
    ($n:ident, $c:tt $(,$p:ident, $t:ident, $i:tt)*) => {
        impl<$($t,)* const N: usize> Unzip for [($($t,)*); N] {
            type Target = ($([$t; N],)*);

            fn unzip(self) -> Self::Target {
                let mut _uninits = ($(MaybeUninit::<[$t; N]>::uninit(),)*);
                let mut _pointers = ($(_uninits.$i.as_mut_ptr() as *mut $t,)*);
                for (_i, _items) in self.into_iter().enumerate() {
                    $(unsafe { _pointers.$i.add(_i).write(_items.$i); })*
                }
                ($(unsafe { _uninits.$i.assume_init() },)*)
            }
        }
    };
}

tuples!(unzip);
