use std::collections::BTreeMap;

use editor_crdt::Dot;

use crate::error::{CodecResult, Corruption, EncodeInvariant};
use crate::primitives::take;
use crate::varint::{read_varint, write_varint};

#[derive(Debug, Default)]
pub struct CollectCtx {
    min_clock: BTreeMap<u64, u64>,
}

impl CollectCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, dot: &Dot) {
        self.min_clock
            .entry(dot.actor)
            .and_modify(|min| *min = (*min).min(dot.clock))
            .or_insert(dot.clock);
    }

    pub fn finalize(self) -> (Vec<u64>, Vec<u64>) {
        let mut actors = Vec::with_capacity(self.min_clock.len());
        let mut baselines = Vec::with_capacity(self.min_clock.len());
        for (actor, min) in self.min_clock {
            actors.push(actor);
            baselines.push(min);
        }
        (actors, baselines)
    }
}

// 라이터 불변식: 리더가 거부하는 형태(길이 불일치·비정렬·중복)를 쓰기 전에 거부한다
fn validate_actor_table(actors: &[u64], baselines: &[u64]) -> CodecResult<()> {
    if actors.len() != baselines.len() {
        return Err(EncodeInvariant::MismatchedActorTable {
            actors: actors.len(),
            baselines: baselines.len(),
        }
        .into());
    }
    if !actors.windows(2).all(|w| w[0] < w[1]) {
        return Err(EncodeInvariant::NonCanonicalActorTable.into());
    }
    Ok(())
}

#[derive(Debug)]
pub struct EncCtx {
    idx_of: BTreeMap<u64, usize>,
    baselines: Vec<u64>,
}

impl EncCtx {
    pub fn from_parts(actors: &[u64], baselines: Vec<u64>) -> CodecResult<Self> {
        validate_actor_table(actors, &baselines)?;
        // 인덱스는 usize 그대로 보존 — u32 등으로의 캐스팅 절단이 표현 불가능하게
        let idx_of = actors.iter().enumerate().map(|(i, &a)| (a, i)).collect();
        Ok(Self { idx_of, baselines })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecCtx {
    pub actors: Vec<u64>,
    pub baselines: Vec<u64>,
}

pub fn write_preamble(actors: &[u64], baselines: &[u64], out: &mut Vec<u8>) -> CodecResult<()> {
    validate_actor_table(actors, baselines)?;
    write_varint(actors.len() as u64, out);
    for &a in actors {
        out.extend_from_slice(&a.to_le_bytes());
    }
    for &b in baselines {
        write_varint(b, out);
    }
    Ok(())
}

pub fn read_preamble(input: &mut &[u8]) -> CodecResult<DecCtx> {
    let count = read_varint(input)?;
    // actor 수 상한은 두지 않는다: actor는 에디터 로드마다 발급되어 히스토리와 함께
    // 정당하게 무한 성장하므로, 고정 상한은 오래 산 문서를 Corruption으로 벽돌화하는
    // 시한폭탄이 된다. 할당은 아래 count*8 ≤ 잔여 검사로 이미 입력 크기에 유계다.
    if count.saturating_mul(8) > input.len() as u64 {
        return Err(Corruption::Truncated {
            expected: (count as usize).saturating_mul(8),
            actual: input.len(),
        }
        .into());
    }
    let count = count as usize;
    let mut actors = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes = take(input, 8)?;
        actors.push(u64::from_le_bytes(bytes.try_into().expect("8 bytes")));
    }
    // 정준성: actor 테이블은 엄격 오름차순(중복 금지) — 라이터(CollectCtx)의 유일한
    // 산출형이며, 리더 측 강제가 없으면 중복 인덱스의 의미 모호성과 비정준 표기가 샌다
    if !actors.windows(2).all(|w| w[0] < w[1]) {
        return Err(Corruption::NonCanonicalActorTable.into());
    }
    let mut baselines = Vec::with_capacity(count);
    for _ in 0..count {
        baselines.push(read_varint(input)?);
    }
    Ok(DecCtx { actors, baselines })
}

pub fn write_dot(dot: &Dot, ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
    let idx = *ctx
        .idx_of
        .get(&dot.actor)
        .ok_or(EncodeInvariant::ActorNotCollected { actor: dot.actor })?;
    let baseline = ctx.baselines[idx];
    if dot.clock < baseline {
        return Err(EncodeInvariant::BaselineUnderflow {
            actor: dot.actor,
            clock: dot.clock,
            baseline,
        }
        .into());
    }
    write_varint(idx as u64, out);
    write_varint(dot.clock - baseline, out);
    Ok(())
}

pub fn read_dot(input: &mut &[u8], ctx: &DecCtx) -> CodecResult<Dot> {
    let idx = read_varint(input)?;
    // pub 필드로 불일치 DecCtx가 구성될 수 있으므로 두 테이블 모두에 대해 검사 (panic 금지)
    let table_len = ctx.actors.len().min(ctx.baselines.len());
    if idx >= table_len as u64 {
        return Err(Corruption::ActorIndexOutOfRange { idx, table_len }.into());
    }
    let delta = read_varint(input)?;
    let baseline = ctx.baselines[idx as usize];
    let clock = baseline
        .checked_add(delta)
        .ok_or(Corruption::ClockOverflow)?;
    Ok(Dot::new(ctx.actors[idx as usize], clock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodecError;

    fn ctx_for(dots: &[Dot]) -> (EncCtx, DecCtx) {
        let mut cc = CollectCtx::new();
        for d in dots {
            cc.observe(d);
        }
        let (actors, baselines) = cc.finalize();
        let enc = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        (enc, DecCtx { actors, baselines })
    }

    fn raw_preamble(actors: &[u64], baselines: &[u64]) -> Vec<u8> {
        // 라이터가 거부하는 형태를 리더 테스트용으로 손수 조립
        let mut buf = Vec::new();
        write_varint(actors.len() as u64, &mut buf);
        for a in actors {
            buf.extend_from_slice(&a.to_le_bytes());
        }
        for b in baselines {
            write_varint(*b, &mut buf);
        }
        buf
    }

    #[test]
    fn collect_builds_sorted_table_and_min_baselines() {
        let dots = [
            Dot::new(99, 5),
            Dot::new(7, 12),
            Dot::new(99, 3),
            Dot::new(7, 10),
        ];
        let mut cc = CollectCtx::new();
        for d in &dots {
            cc.observe(d);
        }
        let (actors, baselines) = cc.finalize();
        assert_eq!(actors, vec![7, 99]);
        assert_eq!(baselines, vec![10, 3]);
    }

    #[test]
    fn preamble_round_trip() {
        let actors = vec![7u64, 99u64];
        let baselines = vec![3u64, 5u64];
        let mut buf = Vec::new();
        write_preamble(&actors, &baselines, &mut buf).unwrap();
        let mut slice = &buf[..];
        let dc = read_preamble(&mut slice).unwrap();
        assert_eq!(dc, DecCtx { actors, baselines });
        assert!(slice.is_empty());
    }

    #[test]
    fn dot_round_trip_multi_actor() {
        let dots = [
            Dot::new(7, 10),
            Dot::new(99, 3),
            Dot::new(7, 12),
            Dot::new(99, 3),
        ];
        let (enc, dec) = ctx_for(&dots);
        let mut buf = Vec::new();
        for d in &dots {
            write_dot(d, &enc, &mut buf).unwrap();
        }
        let mut slice = &buf[..];
        for d in &dots {
            assert_eq!(read_dot(&mut slice, &dec).unwrap(), *d);
        }
        assert!(slice.is_empty());
    }

    #[test]
    fn small_dot_is_two_bytes() {
        let d = Dot::new(7, 10);
        let (enc, _) = ctx_for(&[d]);
        let mut buf = Vec::new();
        write_dot(&d, &enc, &mut buf).unwrap();
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn uncollected_actor_is_encode_invariant() {
        let (enc, _) = ctx_for(&[Dot::new(7, 10)]);
        let mut buf = Vec::new();
        assert!(matches!(
            write_dot(&Dot::new(8, 0), &enc, &mut buf),
            Err(CodecError::Encode(EncodeInvariant::ActorNotCollected {
                actor: 8
            }))
        ));
    }

    #[test]
    fn below_baseline_clock_is_encode_invariant() {
        let (enc, _) = ctx_for(&[Dot::new(7, 10)]);
        let mut buf = Vec::new();
        assert!(matches!(
            write_dot(&Dot::new(7, 9), &enc, &mut buf),
            Err(CodecError::Encode(EncodeInvariant::BaselineUnderflow {
                actor: 7,
                clock: 9,
                baseline: 10
            }))
        ));
    }

    #[test]
    fn out_of_range_actor_index_is_corruption() {
        let dec = DecCtx {
            actors: vec![7],
            baselines: vec![0],
        };
        let mut buf = Vec::new();
        write_varint(5, &mut buf); // idx 5, 테이블 크기 1
        write_varint(0, &mut buf);
        let mut slice = &buf[..];
        assert!(matches!(
            read_dot(&mut slice, &dec),
            Err(CodecError::Corruption(Corruption::ActorIndexOutOfRange {
                idx: 5,
                table_len: 1
            }))
        ));
    }

    #[test]
    fn clock_overflow_is_corruption() {
        let dec = DecCtx {
            actors: vec![7],
            baselines: vec![u64::MAX],
        };
        let mut buf = Vec::new();
        write_varint(0, &mut buf);
        write_varint(1, &mut buf); // baseline u64::MAX + delta 1 → overflow
        let mut slice = &buf[..];
        assert!(matches!(
            read_dot(&mut slice, &dec),
            Err(CodecError::Corruption(Corruption::ClockOverflow))
        ));
    }

    #[test]
    fn preamble_hostile_count_does_not_preallocate() {
        let mut buf = Vec::new();
        write_varint(u64::MAX, &mut buf); // actor 수 u64::MAX 선언, 데이터 없음
        let mut slice = &buf[..];
        assert!(matches!(
            read_preamble(&mut slice),
            Err(CodecError::Corruption(Corruption::Truncated { .. }))
        ));
    }

    #[test]
    fn unsorted_or_duplicate_actor_table_is_rejected() {
        for actors in [[9u64, 7], [7, 7]] {
            let buf = raw_preamble(&actors, &[0, 0]);
            let mut slice = &buf[..];
            assert!(matches!(
                read_preamble(&mut slice),
                Err(CodecError::Corruption(Corruption::NonCanonicalActorTable))
            ));
        }
    }

    #[test]
    fn writer_rejects_noncanonical_actor_table() {
        let mut out = Vec::new();
        assert!(matches!(
            write_preamble(&[9, 7], &[0, 0], &mut out),
            Err(CodecError::Encode(EncodeInvariant::NonCanonicalActorTable))
        ));
        assert!(matches!(
            write_preamble(&[7], &[0, 0], &mut out),
            Err(CodecError::Encode(EncodeInvariant::MismatchedActorTable {
                actors: 1,
                baselines: 2
            }))
        ));
        assert!(matches!(
            EncCtx::from_parts(&[7, 7], vec![0, 0]),
            Err(CodecError::Encode(EncodeInvariant::NonCanonicalActorTable))
        ));
    }

    #[test]
    fn mismatched_dec_ctx_errors_instead_of_panicking() {
        // pub 필드로 구성 가능한 불일치 ctx — 인덱싱 panic 대신 분류된 에러
        let dec = DecCtx {
            actors: vec![7],
            baselines: vec![],
        };
        let mut buf = Vec::new();
        write_varint(0, &mut buf);
        write_varint(0, &mut buf);
        let mut slice = &buf[..];
        assert!(matches!(
            read_dot(&mut slice, &dec),
            Err(CodecError::Corruption(Corruption::ActorIndexOutOfRange {
                idx: 0,
                table_len: 0
            }))
        ));

        let dec = DecCtx {
            actors: vec![7, 8],
            baselines: vec![0],
        };
        let mut buf = Vec::new();
        write_varint(1, &mut buf);
        write_varint(0, &mut buf);
        let mut slice = &buf[..];
        assert!(matches!(
            read_dot(&mut slice, &dec),
            Err(CodecError::Corruption(Corruption::ActorIndexOutOfRange {
                idx: 1,
                table_len: 1
            }))
        ));
    }
}
