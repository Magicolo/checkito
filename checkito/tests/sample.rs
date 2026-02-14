pub mod common;
use common::*;

#[test]
fn sampler_with_same_seed_is_reproducible() {
    let mut left = (0u8..=100).sampler();
    left.seed = 7;
    left.count = 64;

    let mut right = (0u8..=100).sampler();
    right.seed = 7;
    right.count = 64;

    assert_eq!(
        left.samples().collect::<Vec<_>>(),
        right.samples().collect::<Vec<_>>()
    );
}

#[test]
fn sampler_sample_respects_size_for_collections() {
    let sampler = Generate::collect::<Vec<_>>(0u8..=u8::MAX).sampler();

    assert_eq!(sampler.sample(0.0), Vec::<u8>::new());
    assert!(sampler.sample(1.0).len() >= sampler.sample(0.0).len());
}

#[test]
fn samples_iterator_supports_exact_size_and_double_ended_iteration() {
    let mut samples = (0u8..=10).samples(6);

    assert_eq!(samples.size_hint(), (6, Some(6)));
    assert_eq!(samples.len(), 6);

    let _ = samples.next().unwrap();
    let _ = samples.next_back().unwrap();
    assert_eq!(samples.len(), 4);

    let _ = samples.nth(1).unwrap();
    assert_eq!(samples.len(), 2);

    let _ = samples.nth_back(0).unwrap();
    assert_eq!(samples.len(), 1);

    let _ = samples.last().unwrap();
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(0u64..=u64::MAX, 1usize..=64)]
    fn sampler_reproducibility_holds_for_many_seeds(seed: u64, count: usize) {
        let mut left = (0u8..=100).sampler();
        left.seed = seed;
        left.count = count;

        let mut right = (0u8..=100).sampler();
        right.seed = seed;
        right.count = count;

        assert_eq!(
            left.samples().collect::<Vec<_>>(),
            right.samples().collect::<Vec<_>>()
        );
    }

    #[check(0u64..=u64::MAX)]
    fn sampler_single_sample_is_reproducible(seed: u64) {
        let mut left = (0u8..=100).sampler();
        left.seed = seed;
        let mut right = (0u8..=100).sampler();
        right.seed = seed;

        assert_eq!(left.sample(0.0), right.sample(0.0));
        assert_eq!(left.sample(0.5), right.sample(0.5));
        assert_eq!(left.sample(1.0), right.sample(1.0));
    }
}
