use tzif_codec::LocalTimeType;

pub const fn ltt(utc_offset: i32, is_dst: bool, designation_index: u8) -> LocalTimeType {
    LocalTimeType {
        utc_offset,
        is_dst,
        designation_index,
    }
}

pub fn v1_header(
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
