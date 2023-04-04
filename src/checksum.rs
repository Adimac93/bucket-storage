use std::io::Read;
use axum::body::Bytes;
use sha1::{Sha1, Digest};
use sha1::digest::FixedOutput;
use tokio_util::codec::FramedRead;

pub fn get_cheksum(bytes: &[u8]) {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize_fixed();
    let hash = format!("{:x}", hash);
}