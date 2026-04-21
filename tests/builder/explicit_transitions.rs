use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{PosixFooter, TzifBuildError, TzifBuilder, Version, VersionPolicy};

#[test]
fn explicit_transition_builder_generates_fixed_posix_footer() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("NPT", 5 * 3600 + 45 * 60, false)
        .posix_footer(PosixFooter::fixed("NPT", 5 * 3600 + 45 * 60))
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some("NPT-5:45"));
    tzif.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_hides_designation_indexes() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .designation("EST")
        .designation("EDT")
        .local_time_type("EST", -5 * 3600, false)
        .local_time_type("EDT", -4 * 3600, true)
        .transition(1_710_054_000, "EDT")
        .transition(1_730_613_600, "EST")
        .footer("EST5EDT,M3.2.0,M11.1.0")
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some("EST5EDT,M3.2.0,M11.1.0"));
    let block = tzif.v2_plus.as_ref().assert_ok()?;
    assert_eq!(block.designations, b"EST\0EDT\0");
    assert_eq!(block.local_time_types[0].designation_index, 0);
    assert_eq!(block.local_time_types[1].designation_index, 4);
    assert_eq!(block.transition_types, vec![1, 0]);
    tzif.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_rejects_unknown_and_unsorted_transitions() -> TestResult {
    assert!(matches!(
        TzifBuilder::transitions()
            .local_time_type("EST", -5 * 3600, false)
            .transition(0, "EDT")
            .build()
            .assert_err()?,
        TzifBuildError::UnknownDesignation(designation) if designation == "EDT"
    ));

    assert!(matches!(
        TzifBuilder::transitions()
            .local_time_type("EST", -5 * 3600, false)
            .transition(10, "EST")
            .transition(10, "EST")
            .build()
            .assert_err()?,
        TzifBuildError::UnsortedTransitions
    ));

    assert!(matches!(
        TzifBuilder::transitions()
            .local_time_type("EST", -5 * 3600, false)
            .transition(10, "EST")
            .transition(9, "EST")
            .build()
            .assert_err()?,
        TzifBuildError::UnsortedTransitions
    ));

    Ok(())
}

#[test]
fn explicit_transition_builder_rejects_duplicate_and_missing_local_time_types() -> TestResult {
    assert!(matches!(
        TzifBuilder::transitions()
            .local_time_type("UTC", i32::MIN, false)
            .build()
            .assert_err()?,
        TzifBuildError::InvalidUtcOffset
    ));

    assert!(matches!(
        TzifBuilder::transitions()
            .designation("UTC")
            .designation("UTC")
            .local_time_type("UTC", 0, false)
            .build()
            .assert_err()?,
        TzifBuildError::DuplicateDesignation(designation) if designation == "UTC"
    ));

    assert!(matches!(
        TzifBuilder::transitions()
            .local_time_type("UTC", 0, false)
            .local_time_type("UTC", 3600, true)
            .build()
            .assert_err()?,
        TzifBuildError::DuplicateDesignation(designation) if designation == "UTC"
    ));

    assert!(matches!(
        TzifBuilder::transitions().build().assert_err()?,
        TzifBuildError::UnknownDesignation(message) if message == "local time type"
    ));

    Ok(())
}

#[test]
fn explicit_transition_builder_rejects_v1_when_footer_is_present() -> TestResult {
    let raw_err = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .footer("UTC0")
        .version(Version::V1)
        .build()
        .assert_err()?;
    assert!(matches!(
        raw_err,
        TzifBuildError::VersionCannotIncludeFooter {
            version: Version::V1
        }
    ));

    let posix_err = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .posix_footer(PosixFooter::fixed("UTC", 0))
        .version(Version::V1)
        .build()
        .assert_err()?;
    assert!(matches!(
        posix_err,
        TzifBuildError::VersionCannotIncludeFooter {
            version: Version::V1
        }
    ));

    Ok(())
}

#[test]
fn explicit_transition_builder_auto_selects_v2_even_when_v1_could_represent_the_data() -> TestResult
{
    let tzif = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .transition(0, "UTC")
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some(""));
    tzif.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_supports_explicit_legacy_v1_override() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .transition(0, "UTC")
        .version(Version::V1)
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V1);
    assert!(tzif.footer.is_none());
    assert!(tzif.v2_plus.is_none());
    tzif.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_auto_selects_v2_for_64_bit_transitions() -> TestResult {
    let timestamp = i64::from(i32::MAX) + 1;
    let tzif = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .transition(timestamp, "UTC")
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.v1.transition_times, Vec::<i64>::new());
    assert_eq!(
        tzif.v2_plus.as_ref().assert_ok()?.transition_times,
        vec![timestamp]
    );
    tzif.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_supports_version_override() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .transition(0, "UTC")
        .version_policy(VersionPolicy::Exact(Version::V2))
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some(""));
    tzif.to_bytes().assert_ok()?;

    let err = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .transition(i64::from(i32::MAX) + 1, "UTC")
        .version(Version::V1)
        .build()
        .assert_err()?;
    assert!(matches!(
        err,
        TzifBuildError::TransitionOutOfRangeForVersion {
            version: Version::V1,
            ..
        }
    ));

    Ok(())
}
