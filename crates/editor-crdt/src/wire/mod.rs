pub mod ctx;
pub mod dot;
pub mod envelope;
pub mod error;
pub mod preamble;
pub mod primitives;
pub mod varint;

pub use ctx::{CollectCtx, DecCtx, EncCtx};
pub use error::{WireError, WireResult};

use crate::{Changeset, Op};

pub trait Wire: Sized {
    fn collect(&self, _ctx: &mut CollectCtx) {}

    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()>;
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self>;
}

pub trait WireChangeset: Sized {
    /// Per-bundle state carried across changesets so cross-cs implicit encoding
    /// (e.g. carry over `prev_node_id`) can apply when adjacent cs share context.
    type BundleState: Default;

    fn collect_changeset(ops: &[Op<Self>], ctx: &mut CollectCtx);

    /// Returns the number of entries emitted (entry_count, written by the caller before the entries body).
    /// `state` is mutated in/out so the next cs in the bundle can pick up where this one left off.
    fn encode_changeset(
        ops: &[Op<Self>],
        state: &mut Self::BundleState,
        ctx: &EncCtx,
        out: &mut Vec<u8>,
    ) -> WireResult<u32>;

    fn decode_changeset(
        state: &mut Self::BundleState,
        ctx: &DecCtx,
        first_op_parents: Vec<crate::Dot>,
        entry_count: u32,
        input: &mut &[u8],
    ) -> WireResult<Vec<Op<Self>>>;

    /// Returns the last op id of the changeset (used to derive `prev_last_op_id` for the next cs).
    /// Default impl reads from the ops slice; payload types with virtual ops would override.
    fn last_op_id(ops: &[Op<Self>]) -> Option<crate::Dot> {
        ops.last().map(|op| op.id)
    }
}

pub fn encode<P: WireChangeset>(changesets: &[Changeset<P>]) -> WireResult<Vec<u8>> {
    if changesets.is_empty() {
        return Ok(Vec::new());
    }

    let mut cc = CollectCtx::new();
    for cs in changesets {
        P::collect_changeset(&cs.ops, &mut cc);
        if let Some(first) = cs.ops.first() {
            for p in &first.parents {
                cc.observe(p);
            }
        }
    }
    let (actor_table, baselines) = cc.finalize();
    let ec = EncCtx::from_table(&actor_table, baselines.clone());

    let mut body = Vec::new();
    preamble::encode_preamble(&actor_table, &baselines, &mut body);
    varint::write_varint(changesets.len() as u64, &mut body);

    let mut state = P::BundleState::default();
    let mut prev_last_op_id: Option<crate::Dot> = None;

    for cs in changesets {
        // first_op_parents encoding:
        // - parent_count = 0 in the first cs (prev_last_op_id is None) → genesis
        // - parent_count = 0 in later cs → implicit-prev (parents = [prev_last_op_id])
        // - parent_count > 0 → explicit list
        let parents: &[crate::Dot] = if let Some(first) = cs.ops.first() {
            &first.parents
        } else {
            &[]
        };
        let implicit_prev =
            matches!(prev_last_op_id, Some(prev) if parents.len() == 1 && parents[0] == prev);

        if implicit_prev {
            varint::write_varint(0, &mut body);
        } else {
            varint::write_varint(parents.len() as u64, &mut body);
            for p in parents {
                p.encode(&ec, &mut body)?;
            }
        }

        let mut entries_buf = Vec::new();
        let entry_count = P::encode_changeset(&cs.ops, &mut state, &ec, &mut entries_buf)?;
        varint::write_varint(entry_count as u64, &mut body);
        body.extend_from_slice(&entries_buf);

        prev_last_op_id = P::last_op_id(&cs.ops).or(prev_last_op_id);
    }

    Ok(envelope::wrap(body))
}

pub fn decode<P: WireChangeset>(bytes: &[u8]) -> WireResult<Vec<Changeset<P>>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let body = envelope::unwrap(bytes)?;
    let mut input = &body[..];
    let dc = preamble::decode_preamble(&mut input)?;
    let cs_count = varint::read_varint(&mut input)? as usize;

    let mut out = Vec::with_capacity(cs_count);
    let mut state = P::BundleState::default();
    let mut prev_last_op_id: Option<crate::Dot> = None;

    for _ in 0..cs_count {
        let parent_count = varint::read_varint(&mut input)? as usize;
        let parents = if parent_count == 0 {
            // 0 = genesis (first cs) or implicit-prev (later cs).
            match prev_last_op_id {
                None => Vec::new(),
                Some(prev) => vec![prev],
            }
        } else {
            let mut parents = Vec::with_capacity(parent_count);
            for _ in 0..parent_count {
                parents.push(crate::Dot::decode(&dc, &mut input)?);
            }
            parents
        };
        let entry_count = varint::read_varint(&mut input)? as u32;
        let ops = P::decode_changeset(&mut state, &dc, parents, entry_count, &mut input)?;
        prev_last_op_id = P::last_op_id(&ops).or(prev_last_op_id);
        out.push(Changeset { ops });
    }

    if !input.is_empty() {
        return Err(WireError::TrailingBytes {
            remaining: input.len(),
        });
    }
    Ok(out)
}

pub fn encode_dots(dots: &[crate::Dot]) -> WireResult<Vec<u8>> {
    if dots.is_empty() {
        return Ok(Vec::new());
    }
    let mut cc = CollectCtx::new();
    for d in dots {
        d.collect(&mut cc);
    }
    let (actor_table, baselines) = cc.finalize();
    let ec = EncCtx::from_table(&actor_table, baselines.clone());

    let mut body = Vec::new();
    preamble::encode_preamble(&actor_table, &baselines, &mut body);
    varint::write_varint(dots.len() as u64, &mut body);
    for d in dots {
        d.encode(&ec, &mut body)?;
    }

    Ok(envelope::wrap(body))
}

pub fn decode_dots(bytes: &[u8]) -> WireResult<Vec<crate::Dot>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let body = envelope::unwrap(bytes)?;
    let mut input = &body[..];
    let dc = preamble::decode_preamble(&mut input)?;
    let count = varint::read_varint(&mut input)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(crate::Dot::decode(&dc, &mut input)?);
    }
    if !input.is_empty() {
        return Err(WireError::TrailingBytes {
            remaining: input.len(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
