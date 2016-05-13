use std::mem::transmute;

/// Converts a 4 byte array slice into a 32 bit signed integer. The bytes
/// are assumed to be encoded in a big-endian format
pub fn bytes_to_i32(byte_array: &[u8]) -> i32 {
    assert!(byte_array.len() == 4, "Array isn't 4 bytes, cannot cast to int");

    // Re-arrange bytes from big to little-endian (so we can transmute them)
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    bytes[0] = byte_array[3];
    bytes[1] = byte_array[2];
    bytes[2] = byte_array[1];
    bytes[3] = byte_array[0];

    // Do the casting
    unsafe {
        transmute::<[u8; 4], i32>(bytes)
    }
}
