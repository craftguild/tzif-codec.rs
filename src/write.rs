use crate::{
    common::{TimeSize, HEADER_RESERVED_LEN, TZIF_MAGIC},
    validate::validate_file,
    DataBlock, TzifError, TzifFile, Version,
};

impl TzifFile {
    /// Serializes this file to `TZif` bytes after validating it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is invalid or contains counts or timestamps that
    /// cannot be represented by the selected `TZif` version.
    pub fn serialize(&self) -> Result<Vec<u8>, TzifError> {
        validate_file(self)?;
        let mut out = Vec::new();
        write_header(&mut out, self.version, &self.v1)?;
        write_data_block(&mut out, &self.v1, TimeSize::ThirtyTwo)?;
        if self.version.is_v2_plus() {
            let block = self
                .v2_plus
                .as_ref()
                .ok_or(TzifError::MissingV2PlusData(self.version))?;
            write_header(&mut out, self.version, block)?;
            write_data_block(&mut out, block, TimeSize::SixtyFour)?;
            out.push(b'\n');
            out.extend_from_slice(self.footer.as_deref().unwrap_or("").as_bytes());
            out.push(b'\n');
        }
        Ok(out)
    }

    /// Serializes this file to `TZif` bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization validation fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TzifError> {
        self.serialize()
    }
}

fn write_header(out: &mut Vec<u8>, version: Version, block: &DataBlock) -> Result<(), TzifError> {
    out.extend_from_slice(TZIF_MAGIC);
    out.push(version.byte());
    out.extend_from_slice(&[0; HEADER_RESERVED_LEN]);
    write_u32(out, block.ut_local_indicators.len())?;
    write_u32(out, block.standard_wall_indicators.len())?;
    write_u32(out, block.leap_seconds.len())?;
    write_u32(out, block.transition_times.len())?;
    write_u32(out, block.local_time_types.len())?;
    write_u32(out, block.designations.len())?;
    Ok(())
}

fn write_data_block(
    out: &mut Vec<u8>,
    block: &DataBlock,
    time_size: TimeSize,
) -> Result<(), TzifError> {
    for (index, &time) in block.transition_times.iter().enumerate() {
        write_time(out, time, time_size, index, "transition")?;
    }
    out.extend_from_slice(&block.transition_types);
    for local_time_type in &block.local_time_types {
        out.extend_from_slice(&local_time_type.utc_offset.to_be_bytes());
        out.push(u8::from(local_time_type.is_dst));
        out.push(local_time_type.designation_index);
    }
    out.extend_from_slice(&block.designations);
    for (index, leap_second) in block.leap_seconds.iter().enumerate() {
        write_time(out, leap_second.occurrence, time_size, index, "leap")?;
        out.extend_from_slice(&leap_second.correction.to_be_bytes());
    }
    for &is_standard in &block.standard_wall_indicators {
        out.push(u8::from(is_standard));
    }
    for &is_ut in &block.ut_local_indicators {
        out.push(u8::from(is_ut));
    }
    Ok(())
}

fn write_time(
    out: &mut Vec<u8>,
    value: i64,
    time_size: TimeSize,
    index: usize,
    kind: &'static str,
) -> Result<(), TzifError> {
    match time_size {
        TimeSize::ThirtyTwo => {
            let value = i32::try_from(value).map_err(|_| match kind {
                "transition" => TzifError::Version1TransitionOutOfRange { index, value },
                _ => TzifError::Version1LeapSecondOutOfRange { index, value },
            })?;
            out.extend_from_slice(&value.to_be_bytes());
        }
        TimeSize::SixtyFour => out.extend_from_slice(&value.to_be_bytes()),
    }
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, value: usize) -> Result<(), TzifError> {
    let value = u32::try_from(value).map_err(|_| TzifError::CountOverflow {
        field: "count",
        count: value,
    })?;
    out.extend_from_slice(&value.to_be_bytes());
    Ok(())
}
