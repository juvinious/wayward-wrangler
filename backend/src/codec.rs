//! Version-1 Python JSON packet encoding. Evidence files are still hashed raw.

use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    Ok(data)
}

pub fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 15) as usize] as char);
    }
    encoded
}

pub fn sha256(data: &[u8]) -> String {
    lower_hex(Sha256::digest(data).as_ref())
}

pub fn canonical_sha256(value: &Value) -> Result<String, serde_json::Error> {
    Ok(sha256(&canonical(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lowercase_hex_keeps_leading_zero_bytes() {
        assert_eq!(lower_hex(&[0,1,15,16,255]),"00010f10ff");
    }
    #[test]
    fn sha256_known_vectors_preserve_the_evidence_format() {
        assert_eq!(sha256(b""),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
    #[test]
    fn packet_timestamp_round_trip_preserves_frozen_bytes_and_hash() {
        let original=serde_json::json!({"assignment":{"started_at":1792258300.7197313_f64},"legacy":1});
        let bytes=canonical(&original).unwrap();
        let parsed:Value=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical(&parsed).unwrap(),bytes);
        assert_eq!(canonical_sha256(&parsed).unwrap(),sha256(&bytes));
    }
}
