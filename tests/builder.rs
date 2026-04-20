#![allow(
    clippy::indexing_slicing,
    reason = "integration tests use fixture indexing"
)]

mod common;

use common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{
    PosixFooter, PosixTransitionRule, PosixTransitionTime, TzifBuildError, TzifBuilder, Version,
    VersionPolicy,
};

#[test]
fn fixed_offset_builder_generates_v2_with_posix_footer() -> TestResult {
    let tzif = TzifBuilder::fixed_offset("JST", 9 * 3600)
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some("JST-9"));
    let block = tzif.v2_plus.as_ref().assert_ok()?;
    assert_eq!(block.local_time_types[0].utc_offset, 9 * 3600);
    assert_eq!(block.designations, b"JST\0");
    assert_eq!(tzif.serialize().assert_ok()?, tzif.serialize().assert_ok()?);

    Ok(())
}

#[test]
fn fixed_offset_builder_supports_fractional_hour_offsets() -> TestResult {
    let tzif = TzifBuilder::fixed_offset("NPT", 5 * 3600 + 45 * 60)
        .build()
        .assert_ok()?;

    assert_eq!(tzif.footer.as_deref(), Some("NPT-5:45"));

    let tzif = TzifBuilder::fixed_offset("NST", -(3 * 3600 + 30 * 60))
        .build()
        .assert_ok()?;
    assert_eq!(tzif.footer.as_deref(), Some("NST3:30"));

    Ok(())
}

#[test]
fn fixed_offset_builder_supports_exact_v1_without_footer() -> TestResult {
    let tzif = TzifBuilder::fixed_offset("UTC", 0)
        .version(Version::V1)
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V1);
    assert_eq!(tzif.footer, None);
    assert!(tzif.v2_plus.is_none());

    Ok(())
}

#[test]
fn fixed_offset_builder_supports_exact_v3_and_v4() -> TestResult {
    let v3 = TzifBuilder::fixed_offset("UTC", 0)
        .version(Version::V3)
        .build()
        .assert_ok()?;
    assert_eq!(v3.version, Version::V3);
    assert_eq!(v3.footer.as_deref(), Some("UTC0"));
    v3.serialize().assert_ok()?;

    let v4 = TzifBuilder::fixed_offset("UTC", 0)
        .version_policy(VersionPolicy::Exact(Version::V4))
        .build()
        .assert_ok()?;
    assert_eq!(v4.version, Version::V4);
    assert_eq!(v4.footer.as_deref(), Some("UTC0"));
    v4.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn fixed_offset_builder_validates_designations() -> TestResult {
    assert!(matches!(
        TzifBuilder::fixed_offset("", 0).build().assert_err()?,
        TzifBuildError::EmptyDesignation
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("日本", 9 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::NonAsciiDesignation { .. }
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("J!", 9 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::UnsupportedDesignationCharacter { .. }
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("A B", 9 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::UnsupportedDesignationCharacter { character: ' ', .. }
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("J", 9 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::DesignationTooShort { .. }
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("TOO-LONG", 9 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::DesignationTooLong { .. }
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("UTC", i32::MIN)
            .build()
            .assert_err()?,
        TzifBuildError::InvalidUtcOffset
    ));
    assert!(matches!(
        TzifBuilder::fixed_offset("UTC", 25 * 3600)
            .build()
            .assert_err()?,
        TzifBuildError::PosixOffsetOutOfRange { seconds } if seconds == 25 * 3600
    ));

    Ok(())
}

#[test]
fn builder_allows_numeric_and_signed_designations() -> TestResult {
    let minus_ten = TzifBuilder::fixed_offset("-10", -10 * 3600)
        .build()
        .assert_ok()?;
    assert_eq!(minus_ten.footer.as_deref(), Some("<-10>10"));
    assert_eq!(
        minus_ten.v2_plus.as_ref().assert_ok()?.designations,
        b"-10\0"
    );
    minus_ten.serialize().assert_ok()?;

    let plus_0530 = TzifBuilder::fixed_offset("+0530", 5 * 3600 + 30 * 60)
        .build()
        .assert_ok()?;
    assert_eq!(plus_0530.footer.as_deref(), Some("<+0530>-5:30"));
    assert_eq!(
        plus_0530.v2_plus.as_ref().assert_ok()?.designations,
        b"+0530\0"
    );
    plus_0530.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn fixed_offset_builder_supports_full_posix_offset_range() -> TestResult {
    let tzif = TzifBuilder::fixed_offset("MAX", 24 * 3600 + 59 * 60 + 59)
        .build()
        .assert_ok()?;
    assert_eq!(tzif.footer.as_deref(), Some("MAX-24:59:59"));
    tzif.serialize().assert_ok()?;

    let tzif = TzifBuilder::fixed_offset("MIN", -(24 * 3600 + 59 * 60 + 59))
        .build()
        .assert_ok()?;
    assert_eq!(tzif.footer.as_deref(), Some("MIN24:59:59"));
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_generates_fixed_posix_footer() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("NPT", 5 * 3600 + 45 * 60, false)
        .posix_footer(PosixFooter::fixed("NPT", 5 * 3600 + 45 * 60))
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some("NPT-5:45"));
    tzif.serialize().assert_ok()?;

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
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn explicit_transition_builder_generates_structured_posix_footer() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .designation("EST")
        .designation("EDT")
        .local_time_type("EST", -5 * 3600, false)
        .local_time_type("EDT", -4 * 3600, true)
        .transition(1_710_054_000, "EDT")
        .transition(1_730_613_600, "EST")
        .posix_footer(PosixFooter::daylight_saving(
            "EST",
            -5 * 3600,
            "EDT",
            -4 * 3600,
            PosixTransitionRule::month_weekday(3, 2, 0),
            PosixTransitionRule::month_weekday(11, 1, 0),
        ))
        .build()
        .assert_ok()?;

    assert_eq!(tzif.version, Version::V2);
    assert_eq!(tzif.footer.as_deref(), Some("EST5EDT,M3.2.0,M11.1.0"));
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn structured_posix_footer_supports_non_default_offsets_and_times() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("STD", -5 * 3600, false)
        .local_time_type("DST", -3 * 3600, true)
        .transition(1_710_054_000, "DST")
        .transition(1_730_613_600, "STD")
        .posix_footer(
            PosixFooter::daylight_saving(
                "STD",
                -5 * 3600,
                "DST",
                -3 * 3600,
                PosixTransitionRule::julian_without_leap_day(60),
                PosixTransitionRule::zero_based_day(300),
            )
            .start_time(PosixTransitionTime::hms(1, 30, 0))
            .end_time(PosixTransitionTime::seconds(25 * 3600)),
        )
        .build()
        .assert_ok()?;

    assert_eq!(tzif.footer.as_deref(), Some("STD5DST3,J60/1:30,300/25"));
    assert_eq!(tzif.version, Version::V3);
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn structured_posix_footer_validates_rules() -> TestResult {
    let err = TzifBuilder::transitions()
        .local_time_type("EST", -5 * 3600, false)
        .local_time_type("EDT", -4 * 3600, true)
        .transition(0, "EDT")
        .posix_footer(PosixFooter::daylight_saving(
            "EST",
            -5 * 3600,
            "EDT",
            -4 * 3600,
            PosixTransitionRule::month_weekday(13, 2, 0),
            PosixTransitionRule::month_weekday(11, 1, 0),
        ))
        .build()
        .assert_err()?;

    assert!(matches!(
        err,
        TzifBuildError::InvalidPosixMonth { month: 13 }
    ));

    Ok(())
}

#[test]
fn structured_posix_footer_validates_all_rule_bounds() -> TestResult {
    let cases = [
        (PosixTransitionRule::month_weekday(3, 0, 0), "week"),
        (PosixTransitionRule::month_weekday(3, 2, 7), "weekday"),
        (PosixTransitionRule::julian_without_leap_day(0), "julian"),
        (PosixTransitionRule::zero_based_day(366), "zero-based"),
    ];

    for (rule, expected) in cases {
        let err = TzifBuilder::transitions()
            .local_time_type("EST", -5 * 3600, false)
            .local_time_type("EDT", -4 * 3600, true)
            .transition(0, "EDT")
            .posix_footer(PosixFooter::daylight_saving(
                "EST",
                -5 * 3600,
                "EDT",
                -4 * 3600,
                rule,
                PosixTransitionRule::month_weekday(11, 1, 0),
            ))
            .build()
            .assert_err()?;

        match expected {
            "week" => assert!(matches!(err, TzifBuildError::InvalidPosixWeek { week: 0 })),
            "weekday" => assert!(matches!(
                err,
                TzifBuildError::InvalidPosixWeekday { weekday: 7 }
            )),
            "julian" => assert!(matches!(
                err,
                TzifBuildError::InvalidPosixJulianDay { day: 0 }
            )),
            "zero-based" => assert!(matches!(
                err,
                TzifBuildError::InvalidPosixZeroBasedDay { day: 366 }
            )),
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
fn structured_posix_footer_validates_transition_time_bounds() -> TestResult {
    for transition_time in [
        PosixTransitionTime::seconds(168 * 3600),
        PosixTransitionTime::hms(1, 60, 0),
        PosixTransitionTime::hms(1, 0, 60),
    ] {
        let err = TzifBuilder::transitions()
            .local_time_type("EST", -5 * 3600, false)
            .local_time_type("EDT", -4 * 3600, true)
            .transition(0, "EDT")
            .posix_footer(
                PosixFooter::daylight_saving(
                    "EST",
                    -5 * 3600,
                    "EDT",
                    -4 * 3600,
                    PosixTransitionRule::month_weekday(3, 2, 0),
                    PosixTransitionRule::month_weekday(11, 1, 0),
                )
                .start_time(transition_time),
            )
            .build()
            .assert_err()?;

        assert!(matches!(
            err,
            TzifBuildError::InvalidPosixTransitionTime { .. }
        ));
    }

    Ok(())
}

#[test]
fn structured_posix_footer_supports_transition_time_extension_boundaries() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("STD", 0, false)
        .local_time_type("DST", 3600, true)
        .posix_footer(
            PosixFooter::daylight_saving(
                "STD",
                0,
                "DST",
                3600,
                PosixTransitionRule::month_weekday(3, 2, 0),
                PosixTransitionRule::month_weekday(11, 1, 0),
            )
            .start_time(PosixTransitionTime::seconds(-167 * 3600))
            .end_time(PosixTransitionTime::seconds(167 * 3600)),
        )
        .build()
        .assert_ok()?;

    assert_eq!(
        tzif.footer.as_deref(),
        Some("STD0DST,M3.2.0/-167,M11.1.0/167")
    );
    assert_eq!(tzif.version, Version::V3);
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn structured_posix_footer_validates_offset_bounds_without_panic() -> TestResult {
    let err = TzifBuilder::transitions()
        .local_time_type("UTC", 0, false)
        .posix_footer(PosixFooter::fixed("UTC", 25 * 3600))
        .build()
        .assert_err()?;

    assert!(matches!(
        err,
        TzifBuildError::PosixOffsetOutOfRange { seconds } if seconds == 25 * 3600
    ));

    let err = TzifBuilder::transitions()
        .local_time_type("STD", 0, false)
        .local_time_type("DST", 3600, true)
        .transition(0, "DST")
        .posix_footer(PosixFooter::daylight_saving(
            "STD",
            i32::MAX,
            "DST",
            i32::MIN,
            PosixTransitionRule::month_weekday(3, 2, 0),
            PosixTransitionRule::month_weekday(11, 1, 0),
        ))
        .build()
        .assert_err()?;

    assert!(matches!(
        err,
        TzifBuildError::PosixOffsetOutOfRange { seconds } if seconds == i32::MAX
    ));

    Ok(())
}

#[test]
fn structured_posix_footer_can_omit_daylight_offset_outside_explicit_posix_range() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("STD", 24 * 3600 + 59 * 60 + 59, false)
        .local_time_type("DST", 25 * 3600 + 59 * 60 + 59, true)
        .posix_footer(PosixFooter::daylight_saving(
            "STD",
            24 * 3600 + 59 * 60 + 59,
            "DST",
            25 * 3600 + 59 * 60 + 59,
            PosixTransitionRule::month_weekday(3, 2, 0),
            PosixTransitionRule::month_weekday(11, 1, 0),
        ))
        .build()
        .assert_ok()?;

    assert_eq!(
        tzif.footer.as_deref(),
        Some("STD-24:59:59DST,M3.2.0,M11.1.0")
    );
    tzif.serialize().assert_ok()?;

    Ok(())
}

#[test]
fn structured_posix_footer_supports_negative_transition_times() -> TestResult {
    let tzif = TzifBuilder::transitions()
        .local_time_type("EST", -5 * 3600, false)
        .local_time_type("EDT", -4 * 3600, true)
        .transition(0, "EDT")
        .posix_footer(
            PosixFooter::daylight_saving(
                "EST",
                -5 * 3600,
                "EDT",
                -4 * 3600,
                PosixTransitionRule::month_weekday(3, 2, 0),
                PosixTransitionRule::month_weekday(11, 1, 0),
            )
            .start_time(PosixTransitionTime::hms(-1, 30, 15)),
        )
        .build()
        .assert_ok()?;

    assert_eq!(
        tzif.footer.as_deref(),
        Some("EST5EDT,M3.2.0/-1:30:15,M11.1.0")
    );
    assert_eq!(tzif.version, Version::V3);

    Ok(())
}

#[test]
fn structured_posix_footer_rejects_v2_when_extension_is_required() -> TestResult {
    let err = TzifBuilder::transitions()
        .local_time_type("EST", -5 * 3600, false)
        .local_time_type("EDT", -4 * 3600, true)
        .transition(0, "EDT")
        .posix_footer(
            PosixFooter::daylight_saving(
                "EST",
                -5 * 3600,
                "EDT",
                -4 * 3600,
                PosixTransitionRule::month_weekday(3, 2, 0),
                PosixTransitionRule::month_weekday(11, 1, 0),
            )
            .start_time(PosixTransitionTime::hms(-1, 30, 15)),
        )
        .version(Version::V2)
        .build()
        .assert_err()?;

    assert!(matches!(
        err,
        TzifBuildError::VersionCannotRepresentFooterExtension {
            version: Version::V2
        }
    ));

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
    tzif.serialize().assert_ok()?;

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
    tzif.serialize().assert_ok()?;

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
    tzif.serialize().assert_ok()?;

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
    tzif.serialize().assert_ok()?;

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
