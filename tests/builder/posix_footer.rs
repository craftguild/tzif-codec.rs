use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{
    PosixFooter, PosixTransitionRule, PosixTransitionTime, TzifBuildError, TzifBuilder, Version,
};

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
    tzif.to_bytes().assert_ok()?;

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
    tzif.to_bytes().assert_ok()?;

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
    tzif.to_bytes().assert_ok()?;

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
    tzif.to_bytes().assert_ok()?;

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
