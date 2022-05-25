use std::cmp::min;
use super::events::MSG_SIZE;

pub fn fill_buffer(src: &[u8], dest: & mut [u8; MSG_SIZE]) {
    for i in 0..min(src.len(), dest.len()) {
        dest[i] = src[i];
    }
}
