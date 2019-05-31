pub fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24)
        + ((array[1] as u32) << 16)
        + ((array[2] as u32) << 8)
        + ((array[3] as u32) << 0)
}

pub fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 0)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}

pub fn ascii_hex_to_bytes(data: &Vec<u8>) -> Vec<u8> {
    let ascii_decoded = &hex::decode(&data).unwrap();

    // The raw data might have leading zeros, but hex::decode ignores them.
    // Add them back in.
    let start = 4 - ascii_decoded.len();
    let mut padded_decoded: [u8; 4] = [0; 4];
    padded_decoded[start..].copy_from_slice(&ascii_decoded);

    padded_decoded.to_vec()
}
