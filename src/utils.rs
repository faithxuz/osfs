pub fn u16_to_u8arr(a: u16) -> [u8; 2] {
    let mut arr = [0u8; 2];
    arr[0] = (a >> 8)       as u8;
    arr[1] = (a % (1 << 8)) as u8;
    arr
}

pub fn u32_to_u8arr(a: u32) -> [u8; 4] {
    let mut arr = [0u8; 4];
    arr[0] = ( a >> 24)              as u8;
    arr[1] = ((a % (1 << 24)) >> 16) as u8;
    arr[2] = ((a % (1 << 16)) >>  8) as u8;
    arr[3] = ( a % (1 <<  8))        as u8;
    arr
}

pub fn u64_to_u8arr(a: u64) -> [u8; 8] {
    let mut arr = [0u8; 8];
    arr[0] = ( a >> 56)              as u8;
    arr[1] = ((a % (1 << 56)) >> 48) as u8;
    arr[2] = ((a % (1 << 48)) >> 40) as u8;
    arr[3] = ((a % (1 << 40)) >> 32) as u8;
    arr[4] = ((a % (1 << 32)) >> 24) as u8;
    arr[5] = ((a % (1 << 24)) >> 16) as u8;
    arr[6] = ((a % (1 << 16)) >>  8) as u8;
    arr[7] = ( a % (1 <<  8))        as u8;
    arr
}

pub fn u8arr_to_u16(arr: &[u8]) -> u16 {
    u16::from_be_bytes(<[u8;2]>::try_from(arr).unwrap())
}

pub fn u8arr_to_u32(arr: &[u8]) -> u32 {
    u32::from_be_bytes(<[u8;4]>::try_from(arr).unwrap())
}

pub fn u8arr_to_u64(arr: &[u8]) -> u64 {
    u64::from_be_bytes(<[u8;8]>::try_from(arr).unwrap())
}