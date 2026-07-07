use crate::convert::{ReencodableChangesets, decode_changesets, encode_changesets};
use crate::envelope::unwrap_one;
use crate::error::{CodecError, CodecResult};

pub struct Consolidation {
    pub payload: Vec<u8>,
    pub consumed: usize,
    pub consumed_bytes: usize,
}

pub fn consolidate_stream(bytes: &[u8]) -> CodecResult<Option<Consolidation>> {
    let mut input = bytes;
    let mut parts: Vec<ReencodableChangesets> = Vec::new();
    let mut consumed = 0usize;
    let mut consumed_bytes = 0usize;

    while !input.is_empty() {
        let before = input;
        match unwrap_one(&mut input) {
            Ok(_) => {}
            Err(CodecError::Fenced(_)) => break,
            Err(e) => return Err(e),
        }
        let taken = before.len() - input.len();
        let envelope_bytes = &before[..taken];
        let decoded = match decode_changesets(envelope_bytes) {
            Ok(d) => d,
            Err(CodecError::Fenced(_)) => break,
            Err(e) => return Err(e),
        };
        match decoded.into_reencodable() {
            Ok(r) => parts.push(r),
            Err(_) => break,
        }
        consumed += 1;
        consumed_bytes += taken;
    }

    if consumed < 2 {
        return Ok(None);
    }

    let payload = encode_changesets(ReencodableChangesets::concat(parts))?;
    Ok(Some(Consolidation {
        payload,
        consumed,
        consumed_bytes,
    }))
}
