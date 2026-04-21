use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{TzifBuildError, TzifBuilder, Version, VersionPolicy};

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
    assert_eq!(tzif.to_bytes().assert_ok()?, tzif.to_bytes().assert_ok()?);

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
    v3.to_bytes().assert_ok()?;

    let v4 = TzifBuilder::fixed_offset("UTC", 0)
        .version_policy(VersionPolicy::Exact(Version::V4))
        .build()
        .assert_ok()?;
    assert_eq!(v4.version, Version::V4);
    assert_eq!(v4.footer.as_deref(), Some("UTC0"));
    v4.to_bytes().assert_ok()?;

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
    minus_ten.to_bytes().assert_ok()?;

    let plus_0530 = TzifBuilder::fixed_offset("+0530", 5 * 3600 + 30 * 60)
        .build()
        .assert_ok()?;
    assert_eq!(plus_0530.footer.as_deref(), Some("<+0530>-5:30"));
    assert_eq!(
        plus_0530.v2_plus.as_ref().assert_ok()?.designations,
        b"+0530\0"
    );
    plus_0530.to_bytes().assert_ok()?;

    Ok(())
}

#[test]
fn fixed_offset_builder_supports_full_posix_offset_range() -> TestResult {
    let tzif = TzifBuilder::fixed_offset("MAX", 24 * 3600 + 59 * 60 + 59)
        .build()
        .assert_ok()?;
    assert_eq!(tzif.footer.as_deref(), Some("MAX-24:59:59"));
    tzif.to_bytes().assert_ok()?;

    let tzif = TzifBuilder::fixed_offset("MIN", -(24 * 3600 + 59 * 60 + 59))
        .build()
        .assert_ok()?;
    assert_eq!(tzif.footer.as_deref(), Some("MIN24:59:59"));
    tzif.to_bytes().assert_ok()?;

    Ok(())
}
