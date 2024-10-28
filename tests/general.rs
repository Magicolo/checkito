pub mod common;
use common::*;
use generate::State;

pub fn generate_is_object_safe(
    generator: &dyn Generate<Item = u8, Shrink = u8>,
    state: &mut State,
) {
    let mut shrinker = generator.generate(state);
    let _ = shrinker.item();
    let _ = shrinker.shrink();

    let checker = generator.checker();
    let _ = checker.checks(|_| true);

    let sampler = generator.sampler();
    let _ = sampler.sample(1.0);
    let _ = sampler.samples();
}
