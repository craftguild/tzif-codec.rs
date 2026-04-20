#![allow(
    clippy::indexing_slicing,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "integration tests use generated fixtures and boundary mutation"
)]

mod common;

use common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, LeapSecond, LocalTimeType, TzifError, TzifFile, Version};

#[test]
fn parser_rejects_truncated_counted_arrays_before_using_them() -> TestResult {
    let bytes = v1_header(0, 0, 0, 2, 1, 4);
    let err = TzifFile::deserialize(&bytes).assert_err()?;

    assert!(matches!(
        err,
        TzifError::UnexpectedEof {
            offset: 44,
            context: "data block"
        }
    ));

    Ok(())
}

#[test]
fn serializer_emits_version_one_file_without_later_sections() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![-1],
        transition_types: vec![0],
        local_time_types: vec![ltt(-3600, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_ok()?;

    assert_eq!(&bytes[..4], b"TZif");
    assert_eq!(bytes[4], 0);
    assert_eq!(bytes.len(), 44 + 4 + 1 + 6 + 4);
    assert_eq!(&bytes[44..48], &(-1_i32).to_be_bytes());
    assert_eq!(&bytes[49..53], &(-3600_i32).to_be_bytes());
    assert_eq!(
        TzifFile::deserialize(&bytes)
            .assert_ok()?
            .serialize()
            .assert_ok()?,
        bytes
    );

    Ok(())
}

#[test]
fn serializer_emits_version_two_plus_sections_and_big_endian_64_bit_times() -> TestResult {
    let timestamp = i64::from(i32::MAX) + 1;
    let bytes = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![timestamp],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    )
    .serialize()
    .assert_ok()?;

    let second_header_offset = 44 + 6 + 1;
    let v2_data_offset = second_header_offset + 44;
    let footer_offset = v2_data_offset + 8 + 1 + 6 + 4;

    assert_eq!(&bytes[..4], b"TZif");
    assert_eq!(bytes[4], b'2');
    assert_eq!(
        &bytes[second_header_offset..second_header_offset + 4],
        b"TZif"
    );
    assert_eq!(bytes[second_header_offset + 4], b'2');
    assert_eq!(
        &bytes[v2_data_offset..v2_data_offset + 8],
        &timestamp.to_be_bytes()
    );
    assert_eq!(&bytes[footer_offset..footer_offset + 2], b"\n\n");
    assert_eq!(
        TzifFile::deserialize(&bytes)
            .assert_ok()?
            .serialize()
            .assert_ok()?,
        bytes
    );

    Ok(())
}

#[test]
fn parser_rejects_invalid_header_magic_and_version() -> TestResult {
    let mut invalid_magic = v1_header(0, 0, 0, 0, 1, 4);
    invalid_magic[0] = b'X';
    let err = TzifFile::deserialize(&invalid_magic).assert_err()?;
    assert!(matches!(err, TzifError::InvalidMagic { offset: 0 }));

    let mut invalid_version = v1_header(0, 0, 0, 0, 1, 4);
    invalid_version[4] = b'5';
    let err = TzifFile::deserialize(&invalid_version).assert_err()?;
    assert!(matches!(err, TzifError::InvalidVersion(b'5')));

    Ok(())
}

#[test]
fn parser_rejects_version_two_plus_header_version_mismatch() -> TestResult {
    let mut bytes = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), "")
        .serialize()
        .assert_ok()?;
    let second_header = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(index, value)| (value == b"TZif").then_some(index))
        .nth(1)
        .assert_ok()?;
    bytes[second_header + 4] = b'3';

    let err = TzifFile::deserialize(&bytes).assert_err()?;
    assert!(matches!(
        err,
        TzifError::VersionMismatch {
            first: Version::V2,
            second: Version::V3
        }
    ));

    Ok(())
}

#[test]
fn serializer_enforces_version_header_layout_requirements() -> TestResult {
    let unexpected_v2_plus = TzifFile {
        version: Version::V1,
        v1: DataBlock::placeholder(),
        v2_plus: Some(DataBlock::placeholder()),
        footer: Some(String::new()),
    }
    .serialize()
    .assert_err()?;
    assert!(matches!(
        unexpected_v2_plus,
        TzifError::UnexpectedV2PlusData
    ));

    let missing_v2_plus = TzifFile {
        version: Version::V2,
        v1: DataBlock::placeholder(),
        v2_plus: None,
        footer: None,
    }
    .serialize()
    .assert_err()?;
    assert!(matches!(
        missing_v2_plus,
        TzifError::MissingV2PlusData(Version::V2)
    ));

    Ok(())
}

#[test]
fn public_validate_checks_tzif_without_serializing() -> TestResult {
    let valid = TzifFile::v1(DataBlock::placeholder());
    assert!(valid.validate().is_ok());

    let invalid = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let err = invalid.validate().assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidTransitionType {
            index: 0,
            transition_type: 1
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_zero_typecnt_and_charcnt() -> TestResult {
    let missing_type = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;
    assert!(matches!(missing_type, TzifError::EmptyLocalTimeTypes));

    let missing_designation = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: vec![],
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;
    assert!(matches!(missing_designation, TzifError::EmptyDesignations));

    Ok(())
}

#[test]
fn serializer_rejects_indicator_counts_that_are_neither_zero_nor_typecnt() -> TestResult {
    let standard_wall_mismatch = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0), ltt(3600, false, 4)],
        designations: b"UTC\0ONE\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![true],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;
    assert!(matches!(
        standard_wall_mismatch,
        TzifError::CountMismatch {
            field: "standard_wall_indicators",
            expected: 2,
            actual: 1
        }
    ));

    let ut_local_mismatch = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0), ltt(3600, false, 4)],
        designations: b"UTC\0ONE\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![true],
    })
    .serialize()
    .assert_err()?;
    assert!(matches!(
        ut_local_mismatch,
        TzifError::CountMismatch {
            field: "ut_local_indicators",
            expected: 2,
            actual: 1
        }
    ));

    Ok(())
}

#[test]
fn parser_rejects_transition_type_indexes_outside_local_time_types() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![0],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_ok()?;
    let transition_type_offset = 44 + 4;
    let mut invalid = bytes;
    invalid[transition_type_offset] = 1;

    let err = TzifFile::deserialize(&invalid).assert_err()?;
    assert!(matches!(
        err,
        TzifError::InvalidTransitionType {
            index: 0,
            transition_type: 1
        }
    ));

    Ok(())
}

#[test]
fn parser_rejects_designation_indexes_outside_designation_table() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_ok()?;
    let designation_index_offset = 44 + 4 + 1;
    let mut invalid = bytes;
    invalid[designation_index_offset] = 4;

    let err = TzifFile::deserialize(&invalid).assert_err()?;
    assert!(matches!(
        err,
        TzifError::InvalidDesignationIndex {
            index: 0,
            designation_index: 4
        }
    ));

    Ok(())
}

#[test]
fn parser_rejects_invalid_boolean_indicators() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![false],
        ut_local_indicators: vec![false],
    })
    .serialize()
    .assert_ok()?;
    let standard_wall_offset = 44 + 6 + 4;
    let mut invalid = bytes;
    invalid[standard_wall_offset] = 2;

    let err = TzifFile::deserialize(&invalid).assert_err()?;
    assert!(matches!(
        err,
        TzifError::InvalidBooleanIndicator {
            field: "standard_wall_indicators",
            index: 0,
            value: 2
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_transition_type_indexes_outside_local_time_types() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidTransitionType {
            index: 0,
            transition_type: 1
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_designation_indexes_outside_designation_table() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 4)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidDesignationIndex {
            index: 0,
            designation_index: 4
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_transition_times_that_are_not_strictly_ascending() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![0, 0],
        transition_types: vec![0, 0],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::TransitionTimesNotAscending { index: 1 }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_unterminated_designation_indexes() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::UnterminatedDesignation {
            index: 0,
            designation_index: 0
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_invalid_utc_offsets_and_designations() -> TestResult {
    let invalid_offset = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(i32::MIN, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_err()?;
    assert!(matches!(
        invalid_offset,
        TzifError::InvalidUtcOffset { index: 0 }
    ));

    for designations in [
        b"AB\0".to_vec(),
        b"TOO-LONG\0".to_vec(),
        b"JST!\0".to_vec(),
        vec![0xe6, 0x97, 0xa5, 0],
    ] {
        let err = TzifFile::v1(DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations,
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        })
        .serialize()
        .assert_err()?;
        assert!(matches!(
            err,
            TzifError::InvalidDesignation { index: 0, .. }
        ));
    }

    Ok(())
}

#[test]
fn serializer_allows_placeholder_empty_designation_and_unspecified_minus_zero() -> TestResult {
    TzifFile::v1(DataBlock::placeholder())
        .serialize()
        .assert_ok()?;

    TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"-00\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_ok()?;

    Ok(())
}

#[test]
fn serializer_rejects_ut_local_without_standard_wall() -> TestResult {
    for standard_wall_indicators in [vec![false], vec![]] {
        let err = TzifFile::v1(DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators,
            ut_local_indicators: vec![true],
        })
        .serialize()
        .assert_err()?;

        assert!(matches!(
            err,
            TzifError::InvalidUtLocalIndicatorCombination { index: 0 }
        ));
    }

    Ok(())
}

#[test]
fn serializer_rejects_invalid_leap_second_tables() -> TestResult {
    let cases = [
        (
            vec![LeapSecond {
                occurrence: -1,
                correction: 1,
            }],
            "negative-first",
            Version::V4,
        ),
        (
            vec![
                LeapSecond {
                    occurrence: 78_796_800,
                    correction: 1,
                },
                LeapSecond {
                    occurrence: 78_796_800,
                    correction: 2,
                },
            ],
            "not-ascending",
            Version::V4,
        ),
        (
            vec![LeapSecond {
                occurrence: 94_694_401,
                correction: 2,
            }],
            "truncated-v3",
            Version::V3,
        ),
        (
            vec![
                LeapSecond {
                    occurrence: 78_796_800,
                    correction: 1,
                },
                LeapSecond {
                    occurrence: 94_694_401,
                    correction: 1,
                },
            ],
            "expiration-v3",
            Version::V3,
        ),
        (
            vec![
                LeapSecond {
                    occurrence: 78_796_800,
                    correction: 1,
                },
                LeapSecond {
                    occurrence: 94_694_401,
                    correction: 3,
                },
            ],
            "bad-delta",
            Version::V4,
        ),
    ];

    for (leap_seconds, expected, version) in cases {
        let block = DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds,
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        };
        let file = match version {
            Version::V3 => TzifFile::v3(DataBlock::placeholder(), block, ""),
            Version::V4 => TzifFile::v4(DataBlock::placeholder(), block, ""),
            _ => unreachable!(),
        };
        let err = file.serialize().assert_err()?;
        match expected {
            "negative-first" => assert!(matches!(
                err,
                TzifError::FirstLeapSecondOccurrenceNegative { value: -1 }
            )),
            "not-ascending" => assert!(matches!(
                err,
                TzifError::LeapSecondOccurrencesNotAscending { index: 1 }
            )),
            "truncated-v3" => assert!(matches!(
                err,
                TzifError::LeapSecondTruncationRequiresVersion4 {
                    version: Version::V3
                }
            )),
            "expiration-v3" => assert!(matches!(
                err,
                TzifError::LeapSecondExpirationRequiresVersion4 {
                    version: Version::V3
                }
            )),
            "bad-delta" => assert!(matches!(
                err,
                TzifError::InvalidLeapSecondCorrection { index: 1 }
            )),
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
fn serializer_rejects_leap_second_occurrences_outside_utc_month_end() -> TestResult {
    let err = TzifFile::v4(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![LeapSecond {
                occurrence: 10,
                correction: 1,
            }],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    )
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::LeapSecondOccurrenceNotAtMonthEnd { index: 0 }
    ));

    Ok(())
}

#[test]
fn serializer_accepts_negative_leap_second_at_utc_month_end() -> TestResult {
    TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![LeapSecond {
            occurrence: 78_796_800,
            correction: -1,
        }],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .serialize()
    .assert_ok()?;

    Ok(())
}

#[test]
fn serializer_allows_version_four_truncated_and_expiring_leap_second_tables() -> TestResult {
    let block = DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![
            LeapSecond {
                occurrence: 126_230_402,
                correction: 3,
            },
            LeapSecond {
                occurrence: 1_719_532_827,
                correction: 3,
            },
        ],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    };

    TzifFile::v4(DataBlock::placeholder(), block, "")
        .serialize()
        .assert_ok()?;

    Ok(())
}

#[test]
fn serializer_rejects_invalid_raw_footers() -> TestResult {
    for (footer, expected) in [
        ("UTC\0", "control"),
        ("UTC\n0", "control"),
        ("日本0", "ascii"),
        ("EST5EDT,M3.2.0/-1,M11.1.0", "extension"),
        ("not a tz", "syntax"),
    ] {
        let err = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), footer)
            .serialize()
            .assert_err()?;
        match expected {
            "control" => assert!(matches!(err, TzifError::InvalidFooterControlByte)),
            "ascii" => assert!(matches!(err, TzifError::InvalidFooterAscii)),
            "extension" => assert!(matches!(
                err,
                TzifError::FooterExtensionRequiresVersion3 {
                    version: Version::V2
                }
            )),
            "syntax" => assert!(matches!(err, TzifError::InvalidFooterSyntax)),
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
fn serializer_accepts_raw_footer_time_boundaries() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/24:59:59,M11.1.0/0",
    )
    .serialize()
    .assert_ok()?;

    TzifFile::v3(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/-167,M11.1.0/167",
    )
    .serialize()
    .assert_ok()?;

    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/-167,M11.1.0/167",
    )
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterExtensionRequiresVersion3 {
            version: Version::V2
        }
    ));

    Ok(())
}

#[test]
fn serializer_accepts_posix_footer_with_dst_rules_omitted() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_710_054_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5EDT",
    )
    .serialize()
    .assert_ok()?;

    Ok(())
}

#[test]
fn serializer_accepts_rfc_footer_all_year_daylight_saving_example() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_719_792_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"XXX\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "XXX3EDT4,0/0,J365/23",
    )
    .serialize()
    .assert_ok()?;

    Ok(())
}

#[test]
fn serializer_accepts_rfc_footer_version_three_extension_example() -> TestResult {
    TzifFile::v3(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_717_200_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-2 * 3600, true, 4)],
            designations: b"-03\0-02\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "<-03>3<-02>,M3.5.0/-2,M10.5.0/-1",
    )
    .serialize()
    .assert_ok()?;

    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_717_200_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-2 * 3600, true, 4)],
            designations: b"-03\0-02\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "<-03>3<-02>,M3.5.0/-2,M10.5.0/-1",
    )
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterExtensionRequiresVersion3 {
            version: Version::V2
        }
    ));

    Ok(())
}

#[test]
fn serializer_rejects_footer_inconsistent_with_last_transition() -> TestResult {
    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_710_054_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5",
    )
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterInconsistentWithLastTransition
    ));

    Ok(())
}

#[test]
fn serializer_rejects_footer_rule_that_does_not_match_last_transition_time() -> TestResult {
    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_704_067_200],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5EDT,M3.2.0,M11.1.0",
    )
    .serialize()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterInconsistentWithLastTransition
    ));

    Ok(())
}

const fn ltt(utc_offset: i32, is_dst: bool, designation_index: u8) -> LocalTimeType {
    LocalTimeType {
        utc_offset,
        is_dst,
        designation_index,
    }
}

fn v1_header(
    isutcnt: u32,
    isstdcnt: u32,
    leapcnt: u32,
    timecnt: u32,
    typecnt: u32,
    charcnt: u32,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"TZif");
    bytes.push(0);
    bytes.extend_from_slice(&[0; 15]);
    for count in [isutcnt, isstdcnt, leapcnt, timecnt, typecnt, charcnt] {
        bytes.extend_from_slice(&count.to_be_bytes());
    }
    bytes
}
