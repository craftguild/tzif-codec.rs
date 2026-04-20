use crate::{DataBlock, TzifError, Version};

pub fn validate_footer(version: Version, block: &DataBlock, footer: &str) -> Result<(), TzifError> {
    if !footer.is_ascii() {
        return Err(TzifError::InvalidFooterAscii);
    }
    if footer.bytes().any(|byte| byte == b'\0' || byte == b'\n') {
        return Err(TzifError::InvalidFooterControlByte);
    }
    if version == Version::V2 && footer_uses_tz_string_extension(footer) {
        return Err(TzifError::FooterExtensionRequiresVersion3 { version });
    }
    let Some(footer) = parse_footer(version, footer)? else {
        return Ok(());
    };
    validate_footer_consistency(block, &footer)?;
    Ok(())
}

pub fn footer_uses_tz_string_extension(footer: &str) -> bool {
    footer
        .split('/')
        .skip(1)
        .any(|part| match part.as_bytes().first() {
            Some(b'+' | b'-') => true,
            Some(byte) if byte.is_ascii_digit() => {
                let hours_end = part
                    .bytes()
                    .position(|byte| !byte.is_ascii_digit())
                    .unwrap_or(part.len());
                part[..hours_end]
                    .parse::<i32>()
                    .is_ok_and(|hours| hours > 24)
            }
            _ => false,
        })
}

#[derive(Clone, Debug)]
struct ParsedFooter {
    standard: ParsedFooterType,
    daylight: Option<ParsedFooterType>,
    start: Option<ParsedRule>,
    end: Option<ParsedRule>,
}

#[derive(Clone, Debug)]
struct ParsedFooterType {
    designation: String,
    offset_seconds: i32,
    is_dst: bool,
}

#[derive(Clone, Copy, Debug)]
struct ParsedRule {
    date: ParsedRuleDate,
    time_seconds: i32,
}

#[derive(Clone, Copy, Debug)]
enum ParsedRuleDate {
    JulianWithoutLeapDay { day: u32 },
    ZeroBasedDay { day: u32 },
    MonthWeekday { month: u32, week: u32, weekday: u32 },
}

impl ParsedFooter {
    fn type_at(&self, unix_time: i64) -> &ParsedFooterType {
        let (Some(daylight), Some(start), Some(end)) = (&self.daylight, self.start, self.end)
        else {
            return &self.standard;
        };
        let (year, _, _) = civil_from_days(unix_time.div_euclid(86_400));
        let start = rule_transition_utc(year, start, self.standard.offset_seconds);
        let end = rule_transition_utc(year, end, daylight.offset_seconds);
        let is_dst = if start < end {
            unix_time >= start && unix_time < end
        } else {
            unix_time >= start || unix_time < end
        };
        if is_dst {
            daylight
        } else {
            &self.standard
        }
    }
}

fn validate_footer_consistency(block: &DataBlock, footer: &ParsedFooter) -> Result<(), TzifError> {
    let Some((&last_type_index, &last_transition_time)) = block
        .transition_types
        .last()
        .zip(block.transition_times.last())
    else {
        return Ok(());
    };
    let Some(local_time_type) = block.local_time_types.get(usize::from(last_type_index)) else {
        return Ok(());
    };
    let Some(designation) = designation_at(block, local_time_type.designation_index) else {
        return Ok(());
    };
    let Ok(designation) = std::str::from_utf8(designation) else {
        return Ok(());
    };
    let expected =
        if let (Some(daylight), None, None) = (&footer.daylight, footer.start, footer.end) {
            if local_time_type.is_dst {
                daylight
            } else {
                &footer.standard
            }
        } else {
            footer.type_at(last_transition_time)
        };
    let offset_matches = expected.offset_seconds == local_time_type.utc_offset;
    if expected.is_dst != local_time_type.is_dst
        || !offset_matches
        || expected.designation != designation
    {
        return Err(TzifError::FooterInconsistentWithLastTransition);
    }
    Ok(())
}

fn designation_at(block: &DataBlock, designation_index: u8) -> Option<&[u8]> {
    let bytes = block.designations.get(usize::from(designation_index)..)?;
    let end = bytes.iter().position(|&byte| byte == 0)?;
    bytes.get(..end)
}

fn parse_footer(version: Version, footer: &str) -> Result<Option<ParsedFooter>, TzifError> {
    if footer.is_empty() {
        return Ok(None);
    }
    let mut parser = FooterParser::new(version, footer);
    let standard_designation = parser.parse_designation()?;
    let standard_offset_seconds = parser.parse_offset(false)?;
    let standard = ParsedFooterType {
        designation: standard_designation,
        offset_seconds: standard_offset_seconds,
        is_dst: false,
    };
    if parser.is_done() {
        return Ok(Some(ParsedFooter {
            standard,
            daylight: None,
            start: None,
            end: None,
        }));
    }
    if parser.peek() == Some(b',') {
        return Err(TzifError::InvalidFooterSyntax);
    }
    let daylight_designation = parser.parse_designation()?;
    let daylight_offset_seconds = if parser.is_done() || parser.peek() == Some(b',') {
        standard.offset_seconds + 3600
    } else {
        parser.parse_offset(false)?
    };
    if parser.is_done() {
        return Ok(Some(ParsedFooter {
            standard,
            daylight: Some(ParsedFooterType {
                designation: daylight_designation,
                offset_seconds: daylight_offset_seconds,
                is_dst: true,
            }),
            start: None,
            end: None,
        }));
    }
    parser.expect(b',')?;
    let start = parser.parse_rule()?;
    parser.expect(b',')?;
    let end = parser.parse_rule()?;
    if !parser.is_done() {
        return Err(TzifError::InvalidFooterSyntax);
    }
    Ok(Some(ParsedFooter {
        standard,
        daylight: Some(ParsedFooterType {
            designation: daylight_designation,
            offset_seconds: daylight_offset_seconds,
            is_dst: true,
        }),
        start: Some(start),
        end: Some(end),
    }))
}

struct FooterParser<'a> {
    version: Version,
    input: &'a str,
    index: usize,
}

impl<'a> FooterParser<'a> {
    const fn new(version: Version, input: &'a str) -> Self {
        Self {
            version,
            input,
            index: 0,
        }
    }

    const fn is_done(&self) -> bool {
        self.index == self.input.len()
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), TzifError> {
        if self.peek() == Some(byte) {
            self.index += 1;
            Ok(())
        } else {
            Err(TzifError::InvalidFooterSyntax)
        }
    }

    fn parse_designation(&mut self) -> Result<String, TzifError> {
        if self.peek() == Some(b'<') {
            self.index += 1;
            let start = self.index;
            while let Some(byte) = self.peek() {
                if byte == b'>' {
                    let value = &self.input[start..self.index];
                    self.index += 1;
                    return validate_footer_designation(value);
                }
                self.index += 1;
            }
            return Err(TzifError::InvalidFooterSyntax);
        }
        let start = self.index;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_alphabetic() {
                self.index += 1;
            } else {
                break;
            }
        }
        if self.index == start {
            return Err(TzifError::InvalidFooterSyntax);
        }
        validate_footer_designation(&self.input[start..self.index])
    }

    fn parse_offset(&mut self, allow_extension: bool) -> Result<i32, TzifError> {
        let signed_seconds = self.parse_time_like(allow_extension)?;
        Ok(-signed_seconds)
    }

    fn parse_rule(&mut self) -> Result<ParsedRule, TzifError> {
        let date = match self.peek() {
            Some(b'J') => {
                self.index += 1;
                let day = self.parse_number()?;
                if !(1..=365).contains(&day) {
                    return Err(TzifError::InvalidFooterSyntax);
                }
                ParsedRuleDate::JulianWithoutLeapDay { day }
            }
            Some(b'M') => {
                self.index += 1;
                let month = self.parse_number()?;
                self.expect(b'.')?;
                let week = self.parse_number()?;
                self.expect(b'.')?;
                let weekday = self.parse_number()?;
                if !(1..=12).contains(&month) || !(1..=5).contains(&week) || weekday > 6 {
                    return Err(TzifError::InvalidFooterSyntax);
                }
                ParsedRuleDate::MonthWeekday {
                    month,
                    week,
                    weekday,
                }
            }
            Some(byte) if byte.is_ascii_digit() => {
                let day = self.parse_number()?;
                if day > 365 {
                    return Err(TzifError::InvalidFooterSyntax);
                }
                ParsedRuleDate::ZeroBasedDay { day }
            }
            _ => return Err(TzifError::InvalidFooterSyntax),
        };
        let time_seconds = if self.peek() == Some(b'/') {
            self.index += 1;
            self.parse_time_like(self.version >= Version::V3)?
        } else {
            2 * 3600
        };
        Ok(ParsedRule { date, time_seconds })
    }

    fn parse_time_like(&mut self, allow_extension: bool) -> Result<i32, TzifError> {
        let sign = match self.peek() {
            Some(b'-') => {
                self.index += 1;
                -1
            }
            Some(b'+') => {
                self.index += 1;
                1
            }
            _ => 1,
        };
        let hours = self.parse_number()?;
        if hours > if allow_extension { 167 } else { 24 } {
            return Err(TzifError::InvalidFooterSyntax);
        }
        let mut seconds = i32::try_from(hours).map_err(|_| TzifError::InvalidFooterSyntax)? * 3600;
        if self.peek() == Some(b':') {
            self.index += 1;
            let minutes = self.parse_number()?;
            if minutes > 59 {
                return Err(TzifError::InvalidFooterSyntax);
            }
            seconds += i32::try_from(minutes).map_err(|_| TzifError::InvalidFooterSyntax)? * 60;
            if self.peek() == Some(b':') {
                self.index += 1;
                let extra_seconds = self.parse_number()?;
                if extra_seconds > 59 {
                    return Err(TzifError::InvalidFooterSyntax);
                }
                seconds +=
                    i32::try_from(extra_seconds).map_err(|_| TzifError::InvalidFooterSyntax)?;
            }
        }
        Ok(sign * seconds)
    }

    fn parse_number(&mut self) -> Result<u32, TzifError> {
        let start = self.index;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() {
                self.index += 1;
            } else {
                break;
            }
        }
        if self.index == start {
            return Err(TzifError::InvalidFooterSyntax);
        }
        self.input[start..self.index]
            .parse()
            .map_err(|_| TzifError::InvalidFooterSyntax)
    }
}

fn validate_footer_designation(value: &str) -> Result<String, TzifError> {
    if value.len() < 3
        || value.len() > 6
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'+' || byte == b'-')
    {
        return Err(TzifError::InvalidFooterSyntax);
    }
    Ok(value.to_string())
}

fn rule_transition_utc(year: i64, rule: ParsedRule, offset_before_seconds: i32) -> i64 {
    let day = match rule.date {
        ParsedRuleDate::JulianWithoutLeapDay { day } => {
            let mut zero_based = i64::from(day - 1);
            if is_leap_year(year) && day >= 60 {
                zero_based += 1;
            }
            days_from_civil(year, 1, 1) + zero_based
        }
        ParsedRuleDate::ZeroBasedDay { day } => days_from_civil(year, 1, 1) + i64::from(day),
        ParsedRuleDate::MonthWeekday {
            month,
            week,
            weekday,
        } => {
            let first_day = days_from_civil(year, month, 1);
            let first_weekday = weekday_from_days(first_day);
            let mut day = 1 + ((weekday + 7 - first_weekday) % 7) + (week - 1) * 7;
            let days_in_month = days_in_month(year, month);
            if day > days_in_month {
                day -= 7;
            }
            days_from_civil(year, month, day)
        }
    };
    day * 86_400 + i64::from(rule.time_seconds) - i64::from(offset_before_seconds)
}

const fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - (month <= 2) as i64;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i64;
    let day = day as i64;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
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

const fn weekday_from_days(days_since_epoch: i64) -> u32 {
    match (days_since_epoch + 4).rem_euclid(7) {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        _ => 6,
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => unreachable!("validated month"),
    }
}

const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
