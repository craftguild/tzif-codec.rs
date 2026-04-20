use crate::{interop::is_placeholder_type, validate::validate_file, TzdistError, TzifFile};

const APPLICATION_TZIF: &str = "application/tzif";
const APPLICATION_TZIF_LEAP: &str = "application/tzif-leap";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TzifMediaType {
    Tzif,
    TzifLeap,
}

impl TzifMediaType {
    pub const APPLICATION_TZIF: &'static str = APPLICATION_TZIF;
    pub const APPLICATION_TZIF_LEAP: &'static str = APPLICATION_TZIF_LEAP;

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tzif => APPLICATION_TZIF,
            Self::TzifLeap => APPLICATION_TZIF_LEAP,
        }
    }
}

impl TryFrom<&str> for TzifMediaType {
    type Error = TzdistError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            APPLICATION_TZIF => Ok(Self::Tzif),
            APPLICATION_TZIF_LEAP => Ok(Self::TzifLeap),
            value => Err(TzdistError::UnsupportedMediaType(value.to_string())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TzdistTruncation {
    pub start: Option<i64>,
    pub end: Option<i64>,
}

impl TzdistTruncation {
    #[must_use]
    pub const fn start(start: i64) -> Self {
        Self {
            start: Some(start),
            end: None,
        }
    }

    #[must_use]
    pub const fn end(end: i64) -> Self {
        Self {
            start: None,
            end: Some(end),
        }
    }

    #[must_use]
    pub const fn range(start: i64, end: i64) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
        }
    }
}

/// Validates a TZDIST capability format list for `TZif` media types.
///
/// # Errors
///
/// Returns an error if `application/tzif-leap` is advertised without also
/// advertising `application/tzif`.
pub fn validate_tzdist_capability_formats<'a, I>(formats: I) -> Result<(), TzdistError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut has_tzif = false;
    let mut has_tzif_leap = false;
    for format in formats {
        has_tzif |= format == APPLICATION_TZIF;
        has_tzif_leap |= format == APPLICATION_TZIF_LEAP;
    }
    if has_tzif_leap && !has_tzif {
        return Err(TzdistError::TzifLeapCapabilityRequiresTzif);
    }
    Ok(())
}

impl TzifFile {
    #[must_use]
    pub fn has_leap_seconds(&self) -> bool {
        !self.v1.leap_seconds.is_empty()
            || self
                .v2_plus
                .as_ref()
                .is_some_and(|block| !block.leap_seconds.is_empty())
    }

    #[must_use]
    pub fn suggested_media_type(&self) -> &'static str {
        if self.has_leap_seconds() {
            APPLICATION_TZIF_LEAP
        } else {
            APPLICATION_TZIF
        }
    }

    /// Validates that this file can be served as the requested media type.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is invalid, or if leap seconds are present in
    /// an `application/tzif` response.
    pub fn validate_for_media_type(&self, media_type: TzifMediaType) -> Result<(), TzdistError> {
        validate_file(self).map_err(TzdistError::InvalidTzif)?;
        if media_type == TzifMediaType::Tzif && self.has_leap_seconds() {
            return Err(TzdistError::LeapSecondsNotAllowedForApplicationTzif);
        }
        Ok(())
    }

    /// Validates the structural `TZif` requirements for a TZDIST truncation response.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not version 2 or later, lacks required
    /// transitions, has mismatched truncation boundaries, or does not use the
    /// required `-00` placeholder types.
    pub fn validate_tzdist_truncation(
        &self,
        truncation: TzdistTruncation,
    ) -> Result<(), TzdistError> {
        let block = self
            .v2_plus
            .as_ref()
            .ok_or(TzdistError::TruncationRequiresVersion2Plus)?;
        if block.transition_times.is_empty() {
            return Err(TzdistError::TruncationRequiresVersion2PlusTransitions);
        }

        if let Some(start) = truncation.start {
            let first = *block
                .transition_times
                .first()
                .ok_or(TzdistError::TruncationRequiresVersion2PlusTransitions)?;
            if first != start {
                return Err(TzdistError::StartTruncationTransitionMismatch {
                    expected: start,
                    actual: first,
                });
            }
        }

        if let Some(end) = truncation.end {
            let last = *block
                .transition_times
                .last()
                .ok_or(TzdistError::TruncationRequiresVersion2PlusTransitions)?;
            if last != end {
                return Err(TzdistError::EndTruncationTransitionMismatch {
                    expected: end,
                    actual: last,
                });
            }
            if self
                .footer
                .as_deref()
                .is_some_and(|footer| !footer.is_empty())
            {
                return Err(TzdistError::EndTruncationRequiresEmptyFooter);
            }
        }

        validate_file(self).map_err(TzdistError::InvalidTzif)?;

        if let Some(start) = truncation.start {
            let first = *block
                .transition_times
                .first()
                .ok_or(TzdistError::TruncationRequiresVersion2PlusTransitions)?;
            if first != start {
                return Err(TzdistError::StartTruncationTransitionMismatch {
                    expected: start,
                    actual: first,
                });
            }
            if !is_placeholder_type(block, 0) {
                return Err(TzdistError::StartTruncationTypeZeroMustBePlaceholder);
            }
        }

        if truncation.end.is_some() {
            let last_type = usize::from(
                *block
                    .transition_types
                    .last()
                    .ok_or(TzdistError::TruncationRequiresVersion2PlusTransitions)?,
            );
            if !is_placeholder_type(block, last_type) {
                return Err(TzdistError::EndTruncationLastTypeMustBePlaceholder);
            }
        }

        if let (Some(start), Some(end)) = (truncation.start, truncation.end) {
            if start >= end {
                return Err(TzdistError::InvalidTruncationRange { start, end });
            }
        }

        Ok(())
    }
}
