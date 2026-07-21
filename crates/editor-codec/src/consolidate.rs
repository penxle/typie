use std::collections::HashMap;
use std::collections::hash_map::Entry;

use editor_crdt::Dot;

use crate::bundle::decode_bundle_from_envelope;
use crate::convert::{ReencodableChangesets, changesets_from_ctx_and_bundles, encode_changesets};
use crate::envelope::unwrap_one;
use crate::error::{CodecError, CodecResult, Corruption};

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
        // Decode via the envelope `unwrap_one` already parsed — avoids re-parsing
        // the same header a second time through a bytes-based decode entry point.
        // Once `unwrap_one` succeeds, none of the body-level decoding below can
        // raise `Fenced` (that's an envelope-header-only error class), so only
        // `unwrap_one` itself needs to treat it as a stream boundary.
        let envelope = match unwrap_one(&mut input) {
            Ok(e) => e,
            Err(CodecError::Fenced(_)) => break,
            Err(e) => return Err(e),
        };
        let taken = before.len() - input.len();
        let (ctx, bundles) = decode_bundle_from_envelope(&envelope)?;
        let decoded = changesets_from_ctx_and_bundles(ctx, bundles)?;
        match decoded.into_reencodable() {
            Ok(r) => parts.push(r),
            Err(_) => break,
        }
        consumed += 1;
        consumed_bytes += taken;
    }

    if consumed == 0 {
        return Ok(None);
    }

    let mut merged = ReencodableChangesets::concat(parts);
    let slice = merged.as_slice();
    let mut first_seen: HashMap<Dot, usize> = HashMap::with_capacity(slice.len());
    let mut keep = vec![true; slice.len()];
    let mut removed = 0usize;
    for (i, cs) in slice.iter().enumerate() {
        let Some(first) = cs.ops.first() else {
            continue;
        };
        match first_seen.entry(first.id) {
            Entry::Vacant(e) => {
                e.insert(i);
            }
            Entry::Occupied(e) => {
                if &slice[*e.get()] == cs {
                    keep[i] = false;
                    removed += 1;
                } else {
                    return Err(Corruption::DivergentDuplicate { dot: first.id }.into());
                }
            }
        }
    }
    if removed > 0 {
        let mut i = 0usize;
        merged.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });
    }
    if consumed < 2 && removed == 0 {
        return Ok(None);
    }

    let payload = encode_changesets(merged)?;
    Ok(Some(Consolidation {
        payload,
        consumed,
        consumed_bytes,
    }))
}
