use crate::{
    common::TimeSize, footer::validate_footer, leap::validate_leap_seconds, DataBlock, TzifError,
    TzifFile, Version,
};

pub fn validate_file(file: &TzifFile) -> Result<(), TzifError> {
    validate_block(&file.v1, TimeSize::ThirtyTwo, file.version)?;
    if file.version == Version::V1 {
        if file.v2_plus.is_some() || file.footer.is_some() {
            return Err(TzifError::UnexpectedV2PlusData);
        }
    } else if let (Some(block), Some(footer)) = (&file.v2_plus, &file.footer) {
        validate_block(block, TimeSize::SixtyFour, file.version)?;
        validate_footer(file.version, block, footer)?;
    } else {
        return Err(TzifError::MissingV2PlusData(file.version));
    }
    Ok(())
}

fn validate_block(
    block: &DataBlock,
    time_size: TimeSize,
    version: Version,
) -> Result<(), TzifError> {
    if block.local_time_types.is_empty() {
        return Err(TzifError::EmptyLocalTimeTypes);
    }
    if block.local_time_types.len() > 256 {
        return Err(TzifError::TooManyLocalTimeTypes(
            block.local_time_types.len(),
        ));
    }
    if block.designations.is_empty() {
        return Err(TzifError::EmptyDesignations);
    }
    if block.transition_times.len() != block.transition_types.len() {
        return Err(TzifError::CountMismatch {
            field: "transition_types",
            expected: block.transition_times.len(),
            actual: block.transition_types.len(),
        });
    }
    validate_optional_indicator_count(
        "standard_wall_indicators",
        block.local_time_types.len(),
        block.standard_wall_indicators.len(),
    )?;
    validate_optional_indicator_count(
        "ut_local_indicators",
        block.local_time_types.len(),
        block.ut_local_indicators.len(),
    )?;
    validate_indicator_relationship(block)?;
    validate_u32_count("timecnt", block.transition_times.len())?;
    validate_u32_count("typecnt", block.local_time_types.len())?;
    validate_u32_count("charcnt", block.designations.len())?;
    validate_u32_count("leapcnt", block.leap_seconds.len())?;
    validate_u32_count("isstdcnt", block.standard_wall_indicators.len())?;
    validate_u32_count("isutcnt", block.ut_local_indicators.len())?;
    validate_strictly_ascending_transitions(block)?;
    for (index, &transition_type) in block.transition_types.iter().enumerate() {
        if usize::from(transition_type) >= block.local_time_types.len() {
            return Err(TzifError::InvalidTransitionType {
                index,
                transition_type,
            });
        }
    }
    for (index, local_time_type) in block.local_time_types.iter().enumerate() {
        if local_time_type.utc_offset == i32::MIN {
            return Err(TzifError::InvalidUtcOffset { index });
        }
        if usize::from(local_time_type.designation_index) >= block.designations.len() {
            return Err(TzifError::InvalidDesignationIndex {
                index,
                designation_index: local_time_type.designation_index,
            });
        }
        let Some(designation) = designation_at(block, local_time_type.designation_index) else {
            return Err(TzifError::UnterminatedDesignation {
                index,
                designation_index: local_time_type.designation_index,
            });
        };
        if !is_valid_wire_designation(block, designation) {
            return Err(TzifError::InvalidDesignation {
                index,
                designation: designation.to_vec(),
            });
        }
    }
    validate_leap_seconds(block, version)?;
    if matches!(time_size, TimeSize::ThirtyTwo) {
        for (index, &value) in block.transition_times.iter().enumerate() {
            if i32::try_from(value).is_err() {
                return Err(TzifError::Version1TransitionOutOfRange { index, value });
            }
        }
        for (index, leap_second) in block.leap_seconds.iter().enumerate() {
            if i32::try_from(leap_second.occurrence).is_err() {
                return Err(TzifError::Version1LeapSecondOutOfRange {
                    index,
                    value: leap_second.occurrence,
                });
            }
        }
    }
    Ok(())
}

fn validate_strictly_ascending_transitions(block: &DataBlock) -> Result<(), TzifError> {
    for (index, pair) in block.transition_times.windows(2).enumerate() {
        let [previous, next] = pair else {
            continue;
        };
        if previous >= next {
            return Err(TzifError::TransitionTimesNotAscending { index: index + 1 });
        }
    }
    Ok(())
}

fn validate_indicator_relationship(block: &DataBlock) -> Result<(), TzifError> {
    if block.ut_local_indicators.is_empty() {
        return Ok(());
    }
    for (index, &is_ut) in block.ut_local_indicators.iter().enumerate() {
        let is_standard = block
            .standard_wall_indicators
            .get(index)
            .copied()
            .unwrap_or(false);
        if is_ut && !is_standard {
            return Err(TzifError::InvalidUtLocalIndicatorCombination { index });
        }
    }
    Ok(())
}

fn is_valid_wire_designation(block: &DataBlock, designation: &[u8]) -> bool {
    if designation == b"-00" {
        return true;
    }
    if designation.is_empty() {
        return is_placeholder_block(block);
    }
    (3..=6).contains(&designation.len())
        && designation
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'+' || *byte == b'-')
}

fn is_placeholder_block(block: &DataBlock) -> bool {
    block.transition_times.is_empty()
        && block.transition_types.is_empty()
        && block.local_time_types.len() == 1
        && block.designations == [0]
}

fn designation_at(block: &DataBlock, designation_index: u8) -> Option<&[u8]> {
    let bytes = block.designations.get(usize::from(designation_index)..)?;
    let end = bytes.iter().position(|&byte| byte == 0)?;
    bytes.get(..end)
}

const fn validate_optional_indicator_count(
    field: &'static str,
    type_count: usize,
    actual: usize,
) -> Result<(), TzifError> {
    if actual != 0 && actual != type_count {
        return Err(TzifError::CountMismatch {
            field,
            expected: type_count,
            actual,
        });
    }
    Ok(())
}

fn validate_u32_count(field: &'static str, count: usize) -> Result<(), TzifError> {
    if u32::try_from(count).is_err() {
        return Err(TzifError::CountOverflow { field, count });
    }
    Ok(())
}
