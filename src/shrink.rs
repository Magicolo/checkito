use crate::map::Map;

pub trait Shrink: Clone {
    type Item;

    fn generate(&self) -> Self::Item;
    fn shrink(&mut self) -> Option<Self>;

    fn map<T, F: Fn(Self::Item) -> T>(self, map: F) -> Map<Self, T, F>
    where
        Map<Self, T, F>: Shrink,
    {
        Map::shrink(self, map)
    }
}
