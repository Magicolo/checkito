use crate::{
    generate::{Generate, State},
    shrink::{All, Shrink},
    FullGenerate, FullShrink, IntoGenerate, IntoShrink,
};

#[derive(Clone, Debug, Default)]
pub struct Array<T: ?Sized, const N: usize>(pub T);

impl<G: FullGenerate, const N: usize> FullGenerate for Array<G, N> {
    type Item = [G::Item; N];
    type Generate = Array<G::Generate, N>;

    fn generator() -> Self::Generate {
        Array(G::generator())
    }
}

impl<G: IntoGenerate, const N: usize> IntoGenerate for Array<G, N> {
    type Item = [G::Item; N];
    type Generate = Array<G::Generate, N>;

    fn generator(self) -> Self::Generate {
        Array(self.0.generator())
    }
}

impl<G: Generate + ?Sized, const N: usize> Generate for Array<G, N> {
    type Item = [G::Item; N];
    type Shrink = All<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        All::new([(); N].map(|_| self.0.generate(state)))
    }
}

impl<S: FullShrink, const N: usize> FullShrink for Array<S, N> {
    type Item = [S::Item; N];
    type Shrink = All<[S::Shrink; N]>;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        let mut shrinks = [(); N].map(|_| None);
        for (i, item) in item.into_iter().enumerate() {
            shrinks[i] = Some(S::shrinker(item)?);
        }
        Some(All::new(shrinks.map(Option::unwrap)))
    }
}

impl<S: IntoShrink, const N: usize> IntoShrink for Array<S, N> {
    type Item = [S::Item; N];
    type Shrink = All<[S::Shrink; N]>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        let mut shrinks = [(); N].map(|_| None);
        for (i, item) in item.into_iter().enumerate() {
            shrinks[i] = Some(self.0.shrinker(item)?);
        }
        Some(All::new(shrinks.map(Option::unwrap)))
    }
}

impl<G: FullGenerate, const N: usize> FullGenerate for [G; N] {
    type Item = [G::Item; N];
    type Generate = [G::Generate; N];

    fn generator() -> Self::Generate {
        [(); N].map(|_| G::generator())
    }
}

impl<G: IntoGenerate, const N: usize> IntoGenerate for [G; N] {
    type Item = [G::Item; N];
    type Generate = [G::Generate; N];

    fn generator(self) -> Self::Generate {
        self.map(|generate| generate.generator())
    }
}

impl<G: Generate, const N: usize> Generate for [G; N] {
    type Item = [G::Item; N];
    type Shrink = All<[G::Shrink; N]>;

    fn generate(&self, state: &mut State) -> Self::Shrink {
        let mut index = 0;
        All::new([(); N].map(|_| {
            let shrink = self[index].generate(state);
            index += 1;
            shrink
        }))
    }
}

impl<S: FullShrink, const N: usize> FullShrink for [S; N] {
    type Item = [S::Item; N];
    type Shrink = All<[S::Shrink; N]>;

    fn shrinker(item: Self::Item) -> Option<Self::Shrink> {
        let mut shrinks = [(); N].map(|_| None);
        for (i, item) in item.into_iter().enumerate() {
            shrinks[i] = Some(S::shrinker(item)?);
        }
        Some(All::new(shrinks.map(Option::unwrap)))
    }
}

impl<S: IntoShrink, const N: usize> IntoShrink for [S; N] {
    type Item = [S::Item; N];
    type Shrink = All<[S::Shrink; N]>;

    fn shrinker(&self, item: Self::Item) -> Option<Self::Shrink> {
        let mut shrinks = [(); N].map(|_| None);
        for (i, item) in item.into_iter().enumerate() {
            shrinks[i] = Some(self[i].shrinker(item)?);
        }
        Some(All::new(shrinks.map(Option::unwrap)))
    }
}

impl<S: Shrink, const N: usize> Shrink for All<[S; N]> {
    type Item = [S::Item; N];

    fn item(&self) -> Self::Item {
        let mut index = 0;
        [(); N].map(|_| {
            let shrink = self.inner[index].item();
            index += 1;
            shrink
        })
    }

    fn shrink(&mut self) -> Option<Self> {
        let start = self.index;
        self.index += 1;
        for i in 0..N {
            let index = (start + i) % N;
            if let Some(shrink) = self.inner[index].shrink() {
                let mut shrinks = self.inner.clone();
                shrinks[index] = shrink;
                return Some(Self {
                    inner: shrinks,
                    index: self.index,
                });
            }
        }
        None
    }
}
