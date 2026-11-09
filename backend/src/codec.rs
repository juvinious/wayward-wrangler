//! Version-1 Python JSON packet encoding. Evidence files are still hashed raw.

use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    Ok(data)
}

pub fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

pub fn canonical_sha256(value: &Value) -> Result<String, serde_json::Error> {
    Ok(sha256(&canonical(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_timestamp_round_trip_preserves_frozen_bytes_and_hash() {
        let original=serde_json::json!({"assignment":{"started_at":1792258300.7197313_f64},"legacy":1});
        let bytes=canonical(&original).unwrap();
        let parsed:Value=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical(&parsed).unwrap(),bytes);
        assert_eq!(canonical_sha256(&parsed).unwrap(),sha256(&bytes));
    }
}
