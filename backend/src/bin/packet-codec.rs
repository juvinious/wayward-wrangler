use std::{env, fs};
use waywr_backend::codec::{canonical, canonical_sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).ok_or("usage: packet-codec JSON_FILE")?;
    let raw = fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&raw)?;
    let bytes = canonical(&value)?;
    let digest = canonical_sha256(&value)?;
    println!(
        "{}",
        serde_json::json!({"canonical_sha256":digest,"raw_sha256":waywr_backend::codec::sha256(&raw),"canonical_matches_raw":bytes==raw})
    );
    Ok(())
}
