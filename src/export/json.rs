use super::Export;
use crate::error::Result;

pub fn encode(document: &Export) -> Result<Vec<u8>> {
    super::ensure_validated(document)?;
    let mut bytes = serde_json::to_vec_pretty(document)?;
    bytes.push(b'\n');
    Ok(bytes)
}
