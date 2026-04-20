use crate::{DataBlock, TzifError, Version};

pub fn validate_leap_seconds(block: &DataBlock, version: Version) -> Result<(), TzifError> {
    let Some(first) = block.leap_seconds.first() else {
        return Ok(());
    };
    if first.occurrence < 0 {
        return Err(TzifError::FirstLeapSecondOccurrenceNegative {
            value: first.occurrence,
        });
    }
    for (index, pair) in block.leap_seconds.windows(2).enumerate() {
        let [previous, next] = pair else {
            continue;
        };
        if previous.occurrence >= next.occurrence {
            return Err(TzifError::LeapSecondOccurrencesNotAscending { index: index + 1 });
        }
    }

    let truncated_at_start = first.correction != 1 && first.correction != -1;
    if truncated_at_start && version != Version::V4 {
        return Err(TzifError::LeapSecondTruncationRequiresVersion4 { version });
    }

    for (index, pair) in block.leap_seconds.windows(2).enumerate() {
        let index = index + 1;
        let [previous, next] = pair else {
            continue;
        };
        let correction_delta = i64::from(next.correction) - i64::from(previous.correction);
        if correction_delta == 1 || correction_delta == -1 {
            continue;
        }
        let is_expiration = version == Version::V4
            && index == block.leap_seconds.len() - 1
            && next.correction == previous.correction;
        if is_expiration {
            continue;
        }
        if next.correction == previous.correction {
            return Err(TzifError::LeapSecondExpirationRequiresVersion4 { version });
        }
        return Err(TzifError::InvalidLeapSecondCorrection { index });
    }

    validate_leap_second_month_boundaries(block)?;

    if truncated_at_start && version == Version::V4 {
        return Ok(());
    }
    if first.correction != 1 && first.correction != -1 {
        return Err(TzifError::InvalidFirstLeapSecondCorrection {
            correction: first.correction,
        });
    }
    Ok(())
}

fn validate_leap_second_month_boundaries(block: &DataBlock) -> Result<(), TzifError> {
    for (index, leap_second) in block.leap_seconds.iter().enumerate() {
        let previous = index
            .checked_sub(1)
            .and_then(|previous| block.leap_seconds.get(previous));
        let is_expiration = index > 0
            && index == block.leap_seconds.len() - 1
            && previous.is_some_and(|previous| leap_second.correction == previous.correction);
        if is_expiration {
            continue;
        }
        let previous_correction = if index == 0 {
            if leap_second.correction > 0 {
                leap_second.correction - 1
            } else {
                leap_second.correction + 1
            }
        } else {
            previous
                .ok_or(TzifError::LeapSecondOccurrenceNotAtMonthEnd { index })?
                .correction
        };
        let unix_after_leap = leap_second
            .occurrence
            .checked_sub(i64::from(previous_correction))
            .ok_or(TzifError::LeapSecondOccurrenceNotAtMonthEnd { index })?;
        if !is_utc_month_boundary(unix_after_leap) {
            return Err(TzifError::LeapSecondOccurrenceNotAtMonthEnd { index });
        }
    }
    Ok(())
}

const fn is_utc_month_boundary(unix_seconds: i64) -> bool {
    const SECONDS_PER_DAY: i64 = 86_400;
    let days = unix_seconds.div_euclid(SECONDS_PER_DAY);
    let seconds_of_day = unix_seconds.rem_euclid(SECONDS_PER_DAY);
    if seconds_of_day != 0 {
        return false;
    }
    let (_, _, day) = civil_from_days(days);
    day == 1
}

const fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += (month <= 2) as i64;
    (year, month, day)
}
