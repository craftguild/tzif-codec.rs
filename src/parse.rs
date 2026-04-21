use crate::{
    common::{Header, TimeSize, HEADER_RESERVED_LEN, TZIF_MAGIC},
    validate::validate_file,
    DataBlock, LeapSecond, LocalTimeType, TzifError, TzifFile, Version,
};

impl TzifFile {
    /// Parses a `TZif` byte slice into a validated file.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is malformed, truncated, has invalid counts or
    /// indexes, or fails semantic validation.
    pub fn parse(input: &[u8]) -> Result<Self, TzifError> {
        let mut reader = Reader::new(input);
        let first = reader.read_header()?;
        let v1 = reader.read_data_block(&first, TimeSize::ThirtyTwo)?;
        if first.version == Version::V1 {
            reader.expect_eof()?;
            let file = Self::v1(v1);
            validate_file(&file)?;
            return Ok(file);
        }

        let second = reader.read_header()?;
        if second.version != first.version {
            return Err(TzifError::VersionMismatch {
                first: first.version,
                second: second.version,
            });
        }
        let v2_plus = reader.read_data_block(&second, TimeSize::SixtyFour)?;
        let footer = reader.read_footer()?;
        reader.expect_eof()?;
        let file = Self {
            version: first.version,
            v1,
            v2_plus: Some(v2_plus),
            footer: Some(footer),
        };
        validate_file(&file)?;
        Ok(file)
    }
}

struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn read_header(&mut self) -> Result<Header, TzifError> {
        let header_offset = self.offset;
        let magic = self.read_exact(4, "magic")?;
        if magic != TZIF_MAGIC {
            return Err(TzifError::InvalidMagic {
                offset: header_offset,
            });
        }
        let version = Version::from_byte(self.read_u8("version")?)?;
        self.read_exact(HEADER_RESERVED_LEN, "reserved header bytes")?;
        Ok(Header {
            version,
            isutcnt: self.read_count("isutcnt")?,
            isstdcnt: self.read_count("isstdcnt")?,
            leapcnt: self.read_count("leapcnt")?,
            timecnt: self.read_count("timecnt")?,
            typecnt: self.read_count("typecnt")?,
            charcnt: self.read_count("charcnt")?,
        })
    }

    fn read_data_block(
        &mut self,
        header: &Header,
        time_size: TimeSize,
    ) -> Result<DataBlock, TzifError> {
        self.ensure_remaining(header.data_block_len(time_size)?, "data block")?;

        let mut transition_times = Vec::with_capacity(header.timecnt);
        for _ in 0..header.timecnt {
            transition_times.push(self.read_time(time_size, "transition time")?);
        }

        let transition_types = self
            .read_exact(header.timecnt, "transition types")?
            .to_vec();

        let mut local_time_types = Vec::with_capacity(header.typecnt);
        for index in 0..header.typecnt {
            let utc_offset = self.read_i32("local time type UTC offset")?;
            let is_dst = match self.read_u8("local time type DST indicator")? {
                0 => false,
                1 => true,
                value => return Err(TzifError::InvalidDstIndicator { index, value }),
            };
            let designation_index = self.read_u8("local time type designation index")?;
            local_time_types.push(LocalTimeType {
                utc_offset,
                is_dst,
                designation_index,
            });
        }

        let designations = self.read_exact(header.charcnt, "designations")?.to_vec();

        let mut leap_seconds = Vec::with_capacity(header.leapcnt);
        for _ in 0..header.leapcnt {
            leap_seconds.push(LeapSecond {
                occurrence: self.read_time(time_size, "leap-second occurrence")?,
                correction: self.read_i32("leap-second correction")?,
            });
        }

        let standard_wall_indicators =
            self.read_bool_indicators("standard_wall_indicators", header.isstdcnt)?;
        let ut_local_indicators =
            self.read_bool_indicators("ut_local_indicators", header.isutcnt)?;

        Ok(DataBlock {
            transition_times,
            transition_types,
            local_time_types,
            designations,
            leap_seconds,
            standard_wall_indicators,
            ut_local_indicators,
        })
    }

    fn read_footer(&mut self) -> Result<String, TzifError> {
        let start = self.offset;
        if self.read_u8("footer start newline")? != b'\n' {
            return Err(TzifError::MissingFooterStart { offset: start });
        }
        let footer_start = self.offset;
        let footer_bytes = self
            .input
            .get(footer_start..)
            .ok_or(TzifError::UnexpectedEof {
                offset: footer_start,
                context: "footer",
            })?;
        let footer_len = footer_bytes.iter().position(|&byte| byte == b'\n').ok_or(
            TzifError::MissingFooterEnd {
                offset: footer_start,
            },
        )?;
        let footer = std::str::from_utf8(
            self.input
                .get(footer_start..footer_start + footer_len)
                .ok_or(TzifError::UnexpectedEof {
                    offset: footer_start,
                    context: "footer",
                })?,
        )
        .map_err(|_| TzifError::InvalidFooterUtf8)?
        .to_string();
        self.offset = footer_start + footer_len + 1;
        Ok(footer)
    }

    const fn expect_eof(&self) -> Result<(), TzifError> {
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err(TzifError::TrailingData {
                offset: self.offset,
            })
        }
    }

    fn read_bool_indicators(
        &mut self,
        field: &'static str,
        count: usize,
    ) -> Result<Vec<bool>, TzifError> {
        let mut values = Vec::with_capacity(count);
        for index in 0..count {
            values.push(match self.read_u8(field)? {
                0 => false,
                1 => true,
                value => {
                    return Err(TzifError::InvalidBooleanIndicator {
                        field,
                        index,
                        value,
                    })
                }
            });
        }
        Ok(values)
    }

    fn read_time(&mut self, time_size: TimeSize, context: &'static str) -> Result<i64, TzifError> {
        match time_size {
            TimeSize::ThirtyTwo => Ok(i64::from(self.read_i32(context)?)),
            TimeSize::SixtyFour => Ok(i64::from_be_bytes(self.read_array(context)?)),
        }
    }

    fn read_count(&mut self, field: &'static str) -> Result<usize, TzifError> {
        let count = self.read_u32(field)?;
        usize::try_from(count).map_err(|_| TzifError::CountTooLarge { field, count })
    }

    fn read_i32(&mut self, context: &'static str) -> Result<i32, TzifError> {
        Ok(i32::from_be_bytes(self.read_array(context)?))
    }

    fn read_u32(&mut self, context: &'static str) -> Result<u32, TzifError> {
        Ok(u32::from_be_bytes(self.read_array(context)?))
    }

    fn read_u8(&mut self, context: &'static str) -> Result<u8, TzifError> {
        let [byte] = self.read_array(context)?;
        Ok(byte)
    }

    fn read_array<const N: usize>(&mut self, context: &'static str) -> Result<[u8; N], TzifError> {
        let mut bytes = [0; N];
        bytes.copy_from_slice(self.read_exact(N, context)?);
        Ok(bytes)
    }

    fn read_exact(&mut self, len: usize, context: &'static str) -> Result<&'a [u8], TzifError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(TzifError::UnexpectedEof {
                offset: self.offset,
                context,
            })?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(TzifError::UnexpectedEof {
                offset: self.offset,
                context,
            })?;
        self.offset = end;
        Ok(bytes)
    }

    fn ensure_remaining(&self, len: usize, context: &'static str) -> Result<(), TzifError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(TzifError::UnexpectedEof {
                offset: self.offset,
                context,
            })?;
        if end <= self.input.len() {
            Ok(())
        } else {
            Err(TzifError::UnexpectedEof {
                offset: self.offset,
                context,
            })
        }
    }
}
