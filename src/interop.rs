use crate::{common::TimeSize, validate::validate_file, DataBlock, TzifError, TzifFile, Version};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InteroperabilityWarning {
    VersionOneDataMayBeIncomplete,
    VersionThreeOrLaterFooterMayConfuseVersionTwoReaders,
    VersionFourLeapSecondTableMayConfuseStrictRfc8536Readers,
    FooterMayBeIgnoredByReaders,
    MissingEarlyNoOpTransition {
        block: &'static str,
    },
    FirstTransitionAfterRecommendedCompatibilityPoint {
        block: &'static str,
        timestamp: i64,
    },
    TransitionBeforeRecommendedLowerBound {
        block: &'static str,
        index: usize,
        timestamp: i64,
    },
    MinimumI64Transition {
        block: &'static str,
        index: usize,
    },
    NegativeTransition {
        block: &'static str,
        index: usize,
        timestamp: i64,
    },
    FooterContainsAngleBracket,
    DesignationNonAscii {
        block: &'static str,
        index: usize,
        designation: Vec<u8>,
    },
    DesignationLengthOutsideRecommendedRange {
        block: &'static str,
        index: usize,
        designation: String,
    },
    DesignationContainsNonRecommendedAscii {
        block: &'static str,
        index: usize,
        designation: String,
    },
    UnspecifiedLocalTimeDesignation {
        block: &'static str,
        index: usize,
    },
    DaylightOffsetLessThanStandardOffset {
        block: &'static str,
        daylight_offset: i32,
        standard_offset: i32,
    },
    LeapSecondWithSubMinuteOffset {
        block: &'static str,
        offset: i32,
    },
    OffsetOutsideConventionalRange {
        block: &'static str,
        index: usize,
        offset: i32,
    },
    OffsetOutsideRecommendedRange {
        block: &'static str,
        index: usize,
        offset: i32,
    },
    NegativeSubHourOffset {
        block: &'static str,
        index: usize,
        offset: i32,
    },
    OffsetNotMultipleOfMinute {
        block: &'static str,
        index: usize,
        offset: i32,
    },
    OffsetNotMultipleOfQuarterHour {
        block: &'static str,
        index: usize,
        offset: i32,
    },
    UnusedLocalTimeType {
        block: &'static str,
        index: usize,
    },
    UnusedDesignationOctet {
        block: &'static str,
        index: usize,
    },
}

impl TzifFile {
    /// Returns interoperability warnings for readers with common legacy `TZif` limitations.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not structurally valid.
    pub fn interoperability_warnings(&self) -> Result<Vec<InteroperabilityWarning>, TzifError> {
        validate_file(self)?;
        let mut warnings = Vec::new();
        push_file_warnings(self, &mut warnings);
        push_block_warnings("v1", &self.v1, TimeSize::ThirtyTwo, &mut warnings);
        if let Some(block) = &self.v2_plus {
            push_block_warnings("v2_plus", block, TimeSize::SixtyFour, &mut warnings);
        }
        warnings.sort();
        warnings.dedup();
        Ok(warnings)
    }
}

pub fn designation_at(block: &DataBlock, designation_index: u8) -> Option<&[u8]> {
    let start = usize::from(designation_index);
    let bytes = block.designations.get(start..)?;
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    bytes.get(..len)
}

pub fn is_placeholder_type(block: &DataBlock, type_index: usize) -> bool {
    let Some(local_time_type) = block.local_time_types.get(type_index) else {
        return false;
    };
    designation_at(block, local_time_type.designation_index) == Some(b"-00")
}

fn push_file_warnings(file: &TzifFile, warnings: &mut Vec<InteroperabilityWarning>) {
    if let Some(v2_plus) = &file.v2_plus {
        if !v2_plus.transition_times.is_empty() {
            let v1_count = file.v1.transition_times.len();
            let representable_v2_count = v2_plus
                .transition_times
                .iter()
                .filter(|&&timestamp| i32::try_from(timestamp).is_ok())
                .count();
            if v1_count < representable_v2_count {
                warnings.push(InteroperabilityWarning::VersionOneDataMayBeIncomplete);
            }
        }
    }

    if file.version >= Version::V3
        && file
            .footer
            .as_deref()
            .is_some_and(|footer| !footer.is_empty())
    {
        warnings
            .push(InteroperabilityWarning::VersionThreeOrLaterFooterMayConfuseVersionTwoReaders);
    }

    if file.version == Version::V4
        && file
            .v2_plus
            .as_ref()
            .is_some_and(|block| !block.leap_seconds.is_empty())
    {
        warnings.push(
            InteroperabilityWarning::VersionFourLeapSecondTableMayConfuseStrictRfc8536Readers,
        );
    }

    if file
        .footer
        .as_deref()
        .is_some_and(|footer| !footer.is_empty())
    {
        warnings.push(InteroperabilityWarning::FooterMayBeIgnoredByReaders);
        if file
            .footer
            .as_deref()
            .is_some_and(|footer| footer.contains(['<', '>']))
        {
            warnings.push(InteroperabilityWarning::FooterContainsAngleBracket);
        }
    }
}

fn push_block_warnings(
    block_name: &'static str,
    block: &DataBlock,
    time_size: TimeSize,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    if let Some((&first, &first_type)) = block
        .transition_times
        .first()
        .zip(block.transition_types.first())
    {
        if first_type != 0 {
            warnings
                .push(InteroperabilityWarning::MissingEarlyNoOpTransition { block: block_name });
        }
        if first > i64::from(i32::MIN) {
            warnings.push(
                InteroperabilityWarning::FirstTransitionAfterRecommendedCompatibilityPoint {
                    block: block_name,
                    timestamp: first,
                },
            );
        }
    }

    for (index, &timestamp) in block.transition_times.iter().enumerate() {
        if timestamp < 0 {
            warnings.push(InteroperabilityWarning::NegativeTransition {
                block: block_name,
                index,
                timestamp,
            });
        }
        if matches!(time_size, TimeSize::SixtyFour) && timestamp < -(1_i64 << 59) {
            warnings.push(
                InteroperabilityWarning::TransitionBeforeRecommendedLowerBound {
                    block: block_name,
                    index,
                    timestamp,
                },
            );
        }
        if matches!(time_size, TimeSize::SixtyFour) && timestamp == i64::MIN {
            warnings.push(InteroperabilityWarning::MinimumI64Transition {
                block: block_name,
                index,
            });
        }
    }

    for (index, local_time_type) in block.local_time_types.iter().enumerate() {
        push_designation_warnings(
            block_name,
            block,
            index,
            local_time_type.designation_index,
            warnings,
        );
        push_offset_warnings(block_name, index, local_time_type.utc_offset, warnings);
    }
    push_negative_dst_warnings(block_name, block, warnings);
    push_leap_second_offset_warnings(block_name, block, warnings);
    push_unused_local_time_type_warnings(block_name, block, warnings);
    push_unused_designation_octet_warnings(block_name, block, warnings);
}

fn push_designation_warnings(
    block_name: &'static str,
    block: &DataBlock,
    index: usize,
    designation_index: u8,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    let Some(designation) = designation_at(block, designation_index) else {
        return;
    };
    if designation == b"-00" {
        warnings.push(InteroperabilityWarning::UnspecifiedLocalTimeDesignation {
            block: block_name,
            index,
        });
        return;
    }
    let Ok(designation_string) = std::str::from_utf8(designation) else {
        warnings.push(InteroperabilityWarning::DesignationNonAscii {
            block: block_name,
            index,
            designation: designation.to_vec(),
        });
        return;
    };
    if !designation_string.is_ascii() {
        warnings.push(InteroperabilityWarning::DesignationNonAscii {
            block: block_name,
            index,
            designation: designation.to_vec(),
        });
        return;
    }
    if designation_string.len() < 3 || designation_string.len() > 6 {
        warnings.push(
            InteroperabilityWarning::DesignationLengthOutsideRecommendedRange {
                block: block_name,
                index,
                designation: designation_string.to_string(),
            },
        );
    }
    if !designation_string
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'+')
    {
        warnings.push(
            InteroperabilityWarning::DesignationContainsNonRecommendedAscii {
                block: block_name,
                index,
                designation: designation_string.to_string(),
            },
        );
    }
}

fn push_offset_warnings(
    block_name: &'static str,
    index: usize,
    offset: i32,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    if !(-12 * 3600..=12 * 3600).contains(&offset) {
        warnings.push(InteroperabilityWarning::OffsetOutsideConventionalRange {
            block: block_name,
            index,
            offset,
        });
    }
    if !(-89_999..=93_599).contains(&offset) {
        warnings.push(InteroperabilityWarning::OffsetOutsideRecommendedRange {
            block: block_name,
            index,
            offset,
        });
    }
    if (-3599..=-1).contains(&offset) {
        warnings.push(InteroperabilityWarning::NegativeSubHourOffset {
            block: block_name,
            index,
            offset,
        });
    }
    if offset % 60 != 0 {
        warnings.push(InteroperabilityWarning::OffsetNotMultipleOfMinute {
            block: block_name,
            index,
            offset,
        });
    } else if offset % (15 * 60) != 0 {
        warnings.push(InteroperabilityWarning::OffsetNotMultipleOfQuarterHour {
            block: block_name,
            index,
            offset,
        });
    }
}

fn push_unused_local_time_type_warnings(
    block_name: &'static str,
    block: &DataBlock,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    let mut used = vec![false; block.local_time_types.len()];
    if let Some(first) = used.first_mut() {
        *first = true;
    }
    for &transition_type in &block.transition_types {
        if let Some(slot) = used.get_mut(usize::from(transition_type)) {
            *slot = true;
        }
    }
    for (index, is_used) in used.into_iter().enumerate() {
        if !is_used {
            warnings.push(InteroperabilityWarning::UnusedLocalTimeType {
                block: block_name,
                index,
            });
        }
    }
}

fn push_unused_designation_octet_warnings(
    block_name: &'static str,
    block: &DataBlock,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    let mut used = vec![false; block.designations.len()];
    for local_time_type in &block.local_time_types {
        let start = usize::from(local_time_type.designation_index);
        let Some(bytes) = block.designations.get(start..) else {
            continue;
        };
        let Some(end) = bytes.iter().position(|&byte| byte == 0) else {
            continue;
        };
        for is_used in used.iter_mut().skip(start).take(end + 1) {
            *is_used = true;
        }
    }
    for (index, is_used) in used.into_iter().enumerate() {
        if !is_used {
            warnings.push(InteroperabilityWarning::UnusedDesignationOctet {
                block: block_name,
                index,
            });
        }
    }
}

fn push_negative_dst_warnings(
    block_name: &'static str,
    block: &DataBlock,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    let standard_offsets: Vec<i32> = block
        .local_time_types
        .iter()
        .filter(|local_time_type| !local_time_type.is_dst)
        .map(|local_time_type| local_time_type.utc_offset)
        .collect();
    for daylight in block
        .local_time_types
        .iter()
        .filter(|local_time_type| local_time_type.is_dst)
    {
        if let Some(&standard_offset) = standard_offsets
            .iter()
            .filter(|&&standard| daylight.utc_offset < standard)
            .max()
        {
            warnings.push(
                InteroperabilityWarning::DaylightOffsetLessThanStandardOffset {
                    block: block_name,
                    daylight_offset: daylight.utc_offset,
                    standard_offset,
                },
            );
        }
    }
}

fn push_leap_second_offset_warnings(
    block_name: &'static str,
    block: &DataBlock,
    warnings: &mut Vec<InteroperabilityWarning>,
) {
    if block.leap_seconds.is_empty() {
        return;
    }
    for offset in block
        .local_time_types
        .iter()
        .map(|local_time_type| local_time_type.utc_offset)
        .filter(|offset| offset % 60 != 0)
    {
        warnings.push(InteroperabilityWarning::LeapSecondWithSubMinuteOffset {
            block: block_name,
            offset,
        });
    }
}
