use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, TzifError, TzifFile, Version};

use super::support::ltt;

#[test]
fn writer_emits_version_one_file_without_later_sections() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![-1],
        transition_types: vec![0],
        local_time_types: vec![ltt(-3600, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_ok()?;

    assert_eq!(&bytes[..4], b"TZif");
    assert_eq!(bytes[4], 0);
    assert_eq!(bytes.len(), 44 + 4 + 1 + 6 + 4);
    assert_eq!(&bytes[44..48], &(-1_i32).to_be_bytes());
    assert_eq!(&bytes[49..53], &(-3600_i32).to_be_bytes());
    assert_eq!(
        TzifFile::parse(&bytes)
            .assert_ok()?
            .to_bytes()
            .assert_ok()?,
        bytes
    );

    Ok(())
}

#[test]
fn writer_emits_version_two_plus_sections_and_big_endian_64_bit_times() -> TestResult {
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
    .to_bytes()
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
        TzifFile::parse(&bytes)
            .assert_ok()?
            .to_bytes()
            .assert_ok()?,
        bytes
    );

    Ok(())
}

#[test]
fn writer_enforces_version_header_layout_requirements() -> TestResult {
    let unexpected_v2_plus = TzifFile {
        version: Version::V1,
        v1: DataBlock::placeholder(),
        v2_plus: Some(DataBlock::placeholder()),
        footer: Some(String::new()),
    }
    .to_bytes()
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
    .to_bytes()
    .assert_err()?;
    assert!(matches!(
        missing_v2_plus,
        TzifError::MissingV2PlusData(Version::V2)
    ));

    Ok(())
}
