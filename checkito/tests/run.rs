pub mod common;
use checkito::check::Result as CheckResult;
use common::*;

#[test]
fn result_accessors_cover_pass_and_fail_paths() {
    let pass = (0u8..=0).checks(Ok::<_, ()>).next().unwrap();
    assert_eq!(pass.generates(), 1);
    assert_eq!(pass.shrinks(), 0);
    assert_eq!(pass.state().index(), 0);
    let pass_item = pass.clone().into_pass(false).unwrap();
    assert_eq!(pass_item.item, 0);
    assert_eq!(pass_item.proof, 0);
    assert!(pass.clone().fail(false).is_none());
    assert!(pass.clone().into_result().is_ok());

    let fail = (0u8..=0).checks(|_| Err::<(), _>("x")).next().unwrap();
    assert!(fail.clone().pass(false).is_none());
    let fail_item = fail.clone().into_fail(false).unwrap();
    assert_eq!(fail_item.cause, Cause::Disprove("x"));
    assert!(fail.clone().into_result().is_err());
    assert_eq!(fail.into_item(), 0);
}

#[test]
fn fail_message_reports_disprove_and_panic_causes() {
    let disprove = (0u8..=0).check(|_| Err::<(), _>("disproved")).unwrap();
    assert_eq!(disprove.message(), "\"disproved\"");

    let panic = (0u8..=0).check::<(), _>(|_| panic!("boom")).unwrap();
    assert_eq!(panic.message(), "boom");
    assert_eq!(panic.cause, Cause::Panic(Some("boom".into())));
}

#[test]
fn checks_respect_yield_flags_and_still_report_final_failure() {
    let mut checker = (0u8..=3).checker();
    checker.generate.exhaustive = Some(true);
    checker.generate.count = 4;
    checker.generate.items = false;
    checker.shrink.items = false;
    checker.shrink.errors = false;

    let steps = checker
        .checks(|value| value < 2)
        .collect::<Vec<CheckResult<u8, bool>>>();

    assert_eq!(steps.len(), 1);
    assert!(matches!(steps[0], CheckResult::Fail(_)));
}

#[cfg(feature = "check")]
mod check {
    use super::*;

    #[check(0u8..=u8::MAX)]
    fn result_item_round_trip_matches_generated_value(value: u8) {
        let step = same(value).checks(Ok::<_, ()>).next().unwrap();
        assert_eq!(step.into_item(), value);
    }

    #[check(0u8..=u8::MAX)]
    fn result_accessors_match_generated_value(value: u8) {
        let step = same(value).checks(Ok::<_, ()>).next().unwrap();
        assert_eq!(step.generates(), 1);
        assert_eq!(step.shrinks(), 0);
        let pass = step.into_pass(false).unwrap();
        assert_eq!(pass.item, value);
        assert_eq!(pass.proof, value);
    }

    #[check(0u8..=u8::MAX)]
    fn fail_result_reports_disprove_for_arbitrary_value(value: u8) {
        let fail = same(value).check(|_| Err::<(), _>("error")).unwrap();
        assert_eq!(fail.cause, Cause::Disprove("error"));
        assert_eq!(fail.item, value);
    }

    #[check(0u8..=u8::MAX)]
    fn pass_result_has_no_failure_for_arbitrary_value(value: u8) {
        let pass = same(value).checks(Ok::<_, ()>).next().unwrap();
        assert!(pass.clone().fail(false).is_none());
        assert!(pass.into_result().is_ok());
    }
}
