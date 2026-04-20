#![allow(
    clippy::indexing_slicing,
    reason = "integration tests mutate fixed fixtures"
)]

mod common;

use common::{AssertErr, AssertOk, TestResult};
use jiff::{civil, tz::TimeZone, Timestamp};
use tzif_codec::{DataBlock, LeapSecond, LocalTimeType, TzifFile};

#[test]
fn appendix_b1_version_1_utc_with_leap_seconds() -> TestResult {
    let tzif = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: leap_seconds_1972_2016(),
        standard_wall_indicators: vec![false],
        ut_local_indicators: vec![false],
    });

    assert_tzif_bytes_roundtrip(&tzif, B1_UTC_LEAP)?;

    Ok(())
}

#[test]
fn appendix_b2_version_2_pacific_honolulu() -> TestResult {
    let tzif = TzifFile::v2(honolulu_v1(), honolulu_v2(), "HST10");
    let bytes = tzif.serialize().assert_ok()?;

    assert_tzif_bytes_roundtrip(&tzif, B2_HONOLULU)?;

    let zone = TimeZone::tzif("Pacific/Honolulu", &bytes).assert_ok()?;
    let timestamp = Timestamp::from_second(-1_156_939_200).assert_ok()?;
    let local = zone.to_datetime(timestamp);
    assert_eq!(local, civil::date(1933, 5, 4).at(2, 30, 0, 0));

    Ok(())
}

#[test]
fn appendix_b3_truncated_version_2_pacific_johnston() -> TestResult {
    let tzif = TzifFile::v2(DataBlock::placeholder(), johnston_v2(), "");

    assert_tzif_bytes_roundtrip(&tzif, B3_JOHNSTON)?;

    Ok(())
}

#[test]
fn appendix_b4_truncated_version_3_asia_jerusalem() -> TestResult {
    let tzif = TzifFile::v3(
        DataBlock::placeholder(),
        jerusalem_v3(),
        "IST-2IDT,M3.4.4/26,M10.5.0",
    );
    let bytes = tzif.serialize().assert_ok()?;

    assert_tzif_bytes_roundtrip(&tzif, B4_JERUSALEM)?;

    let zone = TimeZone::tzif("Asia/Jerusalem", &bytes).assert_ok()?;
    let timestamp = Timestamp::from_second(2_145_916_800).assert_ok()?;
    let local = zone.to_datetime(timestamp);
    assert_eq!(local, civil::date(2038, 1, 1).at(2, 0, 0, 0));

    Ok(())
}

#[test]
fn appendix_b5_truncated_version_4_europe_london() -> TestResult {
    let tzif = TzifFile::v4(
        DataBlock::placeholder(),
        london_v4(),
        "GMT0BST,M3.5.0/1,M10.5.0",
    );

    assert_tzif_bytes_roundtrip(&tzif, B5_LONDON)?;

    Ok(())
}

#[test]
fn rejects_mismatched_transition_counts() -> TestResult {
    let mut block = DataBlock::placeholder();
    block.transition_times.push(0);

    let err = TzifFile::v1(block).serialize().assert_err()?;
    assert_eq!(
        err.to_string(),
        "transition_types has 0 entries, but expected 1"
    );

    Ok(())
}

#[test]
fn parse_and_to_bytes_remain_aliases() -> TestResult {
    let tzif = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), "UTC0");
    let serialized = tzif.serialize().assert_ok()?;

    assert_eq!(tzif.to_bytes().assert_ok()?, serialized);
    assert_eq!(
        TzifFile::parse(&serialized).assert_ok()?,
        TzifFile::deserialize(&serialized).assert_ok()?
    );

    Ok(())
}

const fn ltt(utc_offset: i32, is_dst: bool, designation_index: u8) -> LocalTimeType {
    LocalTimeType {
        utc_offset,
        is_dst,
        designation_index,
    }
}

fn assert_tzif_bytes_roundtrip(tzif: &TzifFile, expected: &str) -> TestResult {
    let expected = hex(expected)?;
    let encoded = tzif.serialize().assert_ok()?;
    assert_eq!(encoded, expected);

    let deserialized = TzifFile::deserialize(&expected).assert_ok()?;
    assert_eq!(deserialized.serialize().assert_ok()?, expected);
    assert_eq!(deserialized, *tzif);
    Ok(())
}

fn leap_seconds_1972_2016() -> Vec<LeapSecond> {
    [
        78_796_800,
        94_694_401,
        126_230_402,
        157_766_403,
        189_302_404,
        220_924_805,
        252_460_806,
        283_996_807,
        315_532_808,
        362_793_609,
        394_329_610,
        425_865_611,
        489_024_012,
        567_993_613,
        631_152_014,
        662_688_015,
        709_948_816,
        741_484_817,
        773_020_818,
        820_454_419,
        867_715_220,
        915_148_821,
        1_136_073_622,
        1_230_768_023,
        1_341_100_824,
        1_435_708_825,
        1_483_228_826,
    ]
    .into_iter()
    .zip(1_i32..)
    .map(|(occurrence, correction)| LeapSecond {
        occurrence,
        correction,
    })
    .collect()
}

fn honolulu_v1() -> DataBlock {
    DataBlock {
        transition_times: vec![
            -2_147_483_648,
            -1_157_283_000,
            -1_155_436_200,
            -880_198_200,
            -769_395_600,
            -765_376_200,
            -712_150_200,
        ],
        transition_types: vec![1, 2, 1, 3, 4, 1, 5],
        local_time_types: vec![
            ltt(-37_886, false, 0),
            ltt(-37_800, false, 4),
            ltt(-34_200, true, 8),
            ltt(-34_200, true, 12),
            ltt(-34_200, true, 16),
            ltt(-36_000, false, 4),
        ],
        designations: b"LMT\0HST\0HDT\0HWT\0HPT\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![false, false, false, false, true, false],
        ut_local_indicators: vec![false, false, false, false, true, false],
    }
}

fn honolulu_v2() -> DataBlock {
    let mut block = honolulu_v1();
    block.transition_times[0] = -2_334_101_314;
    block
}

fn johnston_v2() -> DataBlock {
    DataBlock {
        transition_times: vec![
            -2_334_101_314,
            -1_157_283_000,
            -1_155_436_200,
            -880_198_200,
            -769_395_600,
            -765_376_200,
            -712_150_200,
            1_087_344_000,
        ],
        transition_types: vec![2, 3, 2, 4, 5, 2, 6, 1],
        local_time_types: vec![
            ltt(-37_886, false, 4),
            ltt(0, false, 0),
            ltt(-37_800, false, 8),
            ltt(-34_200, true, 12),
            ltt(-34_200, true, 16),
            ltt(-34_200, true, 20),
            ltt(-36_000, false, 8),
        ],
        designations: b"-00\0LMT\0HST\0HDT\0HWT\0HPT\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    }
}

fn jerusalem_v3() -> DataBlock {
    DataBlock {
        transition_times: vec![2_145_916_800],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0), ltt(7_200, false, 4)],
        designations: b"-00\0IST\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    }
}

fn london_v4() -> DataBlock {
    DataBlock {
        transition_times: vec![1_640_995_227],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0), ltt(0, false, 4)],
        designations: b"-00\0GMT\0".to_vec(),
        leap_seconds: vec![
            LeapSecond {
                occurrence: 1_483_228_826,
                correction: 27,
            },
            LeapSecond {
                occurrence: 1_719_532_827,
                correction: 27,
            },
        ],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    }
}

fn hex(input: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    input
        .split_ascii_whitespace()
        .map(|part| u8::from_str_radix(part, 16).map_err(Box::<dyn std::error::Error>::from))
        .collect()
}

const B1_UTC_LEAP: &str = "
54 5a 69 66 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 01 00 00 00 01 00 00 00 1b 00 00 00 00 00 00 00 01
00 00 00 04 00 00 00 00 00 00 55 54 43 00
04 b2 58 00 00 00 00 01 05 a4 ec 01 00 00 00 02
07 86 1f 82 00 00 00 03 09 67 53 03 00 00 00 04
0b 48 86 84 00 00 00 05 0d 2b 0b 85 00 00 00 06
0f 0c 3f 06 00 00 00 07 10 ed 72 87 00 00 00 08
12 ce a6 08 00 00 00 09 15 9f ca 89 00 00 00 0a
17 80 fe 0a 00 00 00 0b 19 62 31 8b 00 00 00 0c
1d 25 ea 0c 00 00 00 0d 21 da e5 0d 00 00 00 0e
25 9e 9d 8e 00 00 00 0f 27 7f d1 0f 00 00 00 10
2a 50 f5 90 00 00 00 11 2c 32 29 11 00 00 00 12
2e 13 5c 92 00 00 00 13 30 e7 24 13 00 00 00 14
33 b8 48 94 00 00 00 15 36 8c 10 15 00 00 00 16
43 b7 1b 96 00 00 00 17 49 5c 07 97 00 00 00 18
4f ef 93 18 00 00 00 19 55 93 2d 99 00 00 00 1a
58 68 46 9a 00 00 00 1b 00 00
";

const B2_HONOLULU: &str = "
54 5a 69 66 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 06 00 00 00 06 00 00 00 00 00 00 00 07 00 00 00 06
00 00 00 14 80 00 00 00 bb 05 43 48 bb 21 71 58 cb 89 3d c8
d2 23 f4 70 d2 61 49 38 d5 8d 73 48 01 02 01 03 04 01 05
ff ff 6c 02 00 00 ff ff 6c 58 00 04 ff ff 7a 68 01 08
ff ff 7a 68 01 0c ff ff 7a 68 01 10 ff ff 73 60 00 04
4c 4d 54 00 48 53 54 00 48 44 54 00 48 57 54 00 48 50 54 00
00 00 00 00 01 00 00 00 00 00 01 00
54 5a 69 66 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 06 00 00 00 06 00 00 00 00 00 00 00 07 00 00 00 06
00 00 00 14 ff ff ff ff 74 e0 70 be ff ff ff ff bb 05 43 48
ff ff ff ff bb 21 71 58 ff ff ff ff cb 89 3d c8
ff ff ff ff d2 23 f4 70 ff ff ff ff d2 61 49 38
ff ff ff ff d5 8d 73 48 01 02 01 03 04 01 05
ff ff 6c 02 00 00 ff ff 6c 58 00 04 ff ff 7a 68 01 08
ff ff 7a 68 01 0c ff ff 7a 68 01 10 ff ff 73 60 00 04
4c 4d 54 00 48 53 54 00 48 44 54 00 48 57 54 00 48 50 54 00
00 00 00 00 01 00 00 00 00 00 01 00
0a 48 53 54 31 30 0a
";

const B3_JOHNSTON: &str = "
54 5a 69 66 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
00 00 00 01 00 00 00 00 00 00 00
54 5a 69 66 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 08 00 00 00 07
00 00 00 18 ff ff ff ff 74 e0 70 be ff ff ff ff bb 05 43 48
ff ff ff ff bb 21 71 58 ff ff ff ff cb 89 3d c8
ff ff ff ff d2 23 f4 70 ff ff ff ff d2 61 49 38
ff ff ff ff d5 8d 73 48 00 00 00 00 40 cf 8d 80
02 03 02 04 05 02 06 01
ff ff 6c 02 00 04 00 00 00 00 00 00 ff ff 6c 58 00 08
ff ff 7a 68 01 0c ff ff 7a 68 01 10 ff ff 7a 68 01 14
ff ff 73 60 00 08
2d 30 30 00 4c 4d 54 00 48 53 54 00 48 44 54 00 48 57 54 00 48 50 54 00
0a 0a
";

const B4_JERUSALEM: &str = "
54 5a 69 66 33 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
00 00 00 01 00 00 00 00 00 00 00
54 5a 69 66 33 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 02
00 00 00 08 00 00 00 00 7f e8 17 80 01
00 00 00 00 00 00 00 00 1c 20 00 04
2d 30 30 00 49 53 54 00
0a 49 53 54 2d 32 49 44 54 2c 4d 33 2e 34 2e 34 2f 32 36 2c 4d 31 30 2e 35 2e 30 0a
";

const B5_LONDON: &str = "
54 5a 69 66 34 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
00 00 00 01 00 00 00 00 00 00 00
54 5a 69 66 34 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 01 00 00 00 02
00 00 00 08 00 00 00 00 61 cf 99 9b 01
00 00 00 00 00 00 00 00 00 00 00 04
2d 30 30 00 47 4d 54 00
00 00 00 00 58 68 46 9a 00 00 00 1b
00 00 00 00 66 7d fd 1b 00 00 00 1b
0a 47 4d 54 30 42 53 54 2c 4d 33 2e 35 2e 30 2f 31 2c 4d 31 30 2e 35 2e 30 0a
";
