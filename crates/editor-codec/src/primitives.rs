use crate::error::{CodecResult, Corruption, EncodeInvariant};
use crate::varint::{read_varint, write_varint};

pub fn take<'a>(input: &mut &'a [u8], n: usize) -> CodecResult<&'a [u8]> {
    if input.len() < n {
        return Err(Corruption::Truncated {
            expected: n,
            actual: input.len(),
        }
        .into());
    }
    let (taken, rest) = input.split_at(n);
    *input = rest;
    Ok(taken)
}

pub fn read_len_prefixed<'a>(input: &mut &'a [u8]) -> CodecResult<&'a [u8]> {
    let len = read_varint(input)?;
    if len > input.len() as u64 {
        return Err(Corruption::LengthOverflow {
            declared: len,
            remaining: input.len(),
        }
        .into());
    }
    take(input, len as usize)
}

pub fn write_u8(v: u8, out: &mut Vec<u8>) {
    out.push(v);
}

pub fn read_u8(input: &mut &[u8]) -> CodecResult<u8> {
    Ok(take(input, 1)?[0])
}

pub fn write_bool(v: bool, out: &mut Vec<u8>) {
    out.push(v as u8);
}

pub fn read_bool(input: &mut &[u8]) -> CodecResult<bool> {
    match read_u8(input)? {
        0 => Ok(false),
        1 => Ok(true),
        got => Err(Corruption::InvalidBool { got }.into()),
    }
}

pub fn write_char(v: char, out: &mut Vec<u8>) {
    let mut buf = [0u8; 4];
    out.extend_from_slice(v.encode_utf8(&mut buf).as_bytes());
}

pub fn read_char(input: &mut &[u8]) -> CodecResult<char> {
    let b0 = *input.first().ok_or(Corruption::Truncated {
        expected: 1,
        actual: 0,
    })?;
    let len = if b0 < 0x80 {
        1
    } else if b0 >= 0xf0 {
        4
    } else if b0 >= 0xe0 {
        3
    } else if b0 >= 0xc0 {
        2
    } else {
        return Err(Corruption::InvalidChar { got: u32::from(b0) }.into());
    };
    let bytes = take(input, len)?;
    let s = std::str::from_utf8(bytes).map_err(|_| Corruption::InvalidUtf8)?;
    s.chars()
        .next()
        .ok_or_else(|| Corruption::InvalidUtf8.into())
}

pub fn write_string(v: &str, out: &mut Vec<u8>) {
    write_varint(v.len() as u64, out);
    out.extend_from_slice(v.as_bytes());
}

pub fn read_string(input: &mut &[u8]) -> CodecResult<String> {
    let bytes = read_len_prefixed(input)?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| Corruption::InvalidUtf8.into())
}

pub fn write_option<T>(
    v: &Option<T>,
    out: &mut Vec<u8>,
    f: impl FnOnce(&T, &mut Vec<u8>) -> CodecResult<()>,
) -> CodecResult<()> {
    match v {
        None => {
            out.push(0);
            Ok(())
        }
        Some(inner) => {
            out.push(1);
            f(inner, out)
        }
    }
}

pub fn read_option<T>(
    input: &mut &[u8],
    f: impl FnOnce(&mut &[u8]) -> CodecResult<T>,
) -> CodecResult<Option<T>> {
    match read_u8(input)? {
        0 => Ok(None),
        1 => Ok(Some(f(input)?)),
        got => Err(Corruption::InvalidBool { got }.into()),
    }
}

pub fn write_vec<T>(
    v: &[T],
    out: &mut Vec<u8>,
    mut f: impl FnMut(&T, &mut Vec<u8>) -> CodecResult<()>,
) -> CodecResult<()> {
    write_varint(v.len() as u64, out);
    for item in v {
        let before = out.len();
        f(item, out)?;
        if out.len() == before {
            // 리더가 거부하는 형태(0바이트 원소)를 쓰기 전에 거부 — 라이터 불변식
            return Err(EncodeInvariant::ZeroWidthVecElement.into());
        }
    }
    Ok(())
}

pub fn read_vec<T>(
    input: &mut &[u8],
    mut f: impl FnMut(&mut &[u8]) -> CodecResult<T>,
) -> CodecResult<Vec<T>> {
    let count = read_varint(input)?;
    let max_elems = input.len() / size_of::<T>().max(1);
    let mut out = Vec::with_capacity((count as usize).min(max_elems));
    for _ in 0..count {
        let before = input.len();
        out.push(f(input)?);
        if input.len() == before {
            // 원소가 0바이트를 소비하면 count가 자원 소모를 직접 지배한다 — 거부.
            // (wire 스키마는 0바이트 원소 Vec을 정의하지 않는다)
            return Err(Corruption::NoProgress.into());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodecError;

    #[test]
    fn take_and_truncation() {
        let mut slice: &[u8] = &[1, 2, 3];
        assert_eq!(take(&mut slice, 2).unwrap(), &[1, 2]);
        assert_eq!(slice, &[3]);
        assert!(matches!(
            take(&mut slice, 2),
            Err(CodecError::Corruption(Corruption::Truncated {
                expected: 2,
                actual: 1
            }))
        ));
    }

    #[test]
    fn len_prefixed_guard_rejects_overdeclared_length() {
        // 선언 길이 1000, 잔여 2바이트 → 할당 없이 즉시 LengthOverflow
        let mut bytes = Vec::new();
        write_varint(1000, &mut bytes);
        bytes.extend_from_slice(&[0xaa, 0xbb]);
        let mut slice = &bytes[..];
        assert!(matches!(
            read_len_prefixed(&mut slice),
            Err(CodecError::Corruption(Corruption::LengthOverflow {
                declared: 1000,
                remaining: 2
            }))
        ));
    }

    #[test]
    fn bool_strict() {
        let mut out = Vec::new();
        write_bool(true, &mut out);
        write_bool(false, &mut out);
        assert_eq!(out, vec![1, 0]);
        let mut bad: &[u8] = &[2];
        assert!(matches!(
            read_bool(&mut bad),
            Err(CodecError::Corruption(Corruption::InvalidBool { got: 2 }))
        ));
    }

    #[test]
    fn char_utf8_round_trip() {
        for c in ['a', '한', '𝄞', '\u{10FFFF}'] {
            let mut out = Vec::new();
            write_char(c, &mut out);
            let mut slice = &out[..];
            assert_eq!(read_char(&mut slice).unwrap(), c);
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn char_continuation_byte_leader_is_invalid_char() {
        let mut slice: &[u8] = &[0x80];
        assert!(matches!(
            read_char(&mut slice),
            Err(CodecError::Corruption(Corruption::InvalidChar {
                got: 0x80
            }))
        ));
    }

    #[test]
    fn string_round_trip_and_invalid_utf8() {
        let mut out = Vec::new();
        write_string("타이피 typie", &mut out);
        let mut slice = &out[..];
        assert_eq!(read_string(&mut slice).unwrap(), "타이피 typie");

        let mut bad = Vec::new();
        write_varint(2, &mut bad);
        bad.extend_from_slice(&[0xff, 0xfe]);
        let mut slice = &bad[..];
        assert!(matches!(
            read_string(&mut slice),
            Err(CodecError::Corruption(Corruption::InvalidUtf8))
        ));
    }

    #[test]
    fn option_round_trip() {
        let mut out = Vec::new();
        write_option(&Some(300u64), &mut out, |v, o| {
            write_varint(*v, o);
            Ok(())
        })
        .unwrap();
        write_option(&None::<u64>, &mut out, |v, o| {
            write_varint(*v, o);
            Ok(())
        })
        .unwrap();
        let mut slice = &out[..];
        assert_eq!(read_option(&mut slice, read_varint).unwrap(), Some(300));
        assert_eq!(read_option(&mut slice, read_varint).unwrap(), None);
        assert!(slice.is_empty());
    }

    #[test]
    fn vec_capacity_guard_does_not_preallocate_from_hostile_count() {
        // count = u64::MAX 선언, 실제 데이터 없음 → OOM 없이 Truncated로 실패해야 함
        let mut bytes = Vec::new();
        write_varint(u64::MAX, &mut bytes);
        let mut slice = &bytes[..];
        let result = read_vec(&mut slice, read_varint);
        assert!(matches!(
            result,
            Err(CodecError::Corruption(Corruption::Truncated { .. }))
        ));
    }

    #[test]
    fn vec_round_trip() {
        let values: Vec<u64> = vec![0, 127, 128, u64::MAX];
        let mut out = Vec::new();
        write_vec(&values, &mut out, |v, o| {
            write_varint(*v, o);
            Ok(())
        })
        .unwrap();
        let mut slice = &out[..];
        assert_eq!(read_vec(&mut slice, read_varint).unwrap(), values);
        assert!(slice.is_empty());
    }

    #[test]
    fn vec_hostile_count_with_multibyte_elements_errors_without_panic() {
        let mut bytes = Vec::new();
        write_varint(u64::MAX, &mut bytes);
        bytes.extend_from_slice(&[0u8; 16]);
        let mut slice = &bytes[..];
        let result = read_vec(&mut slice, |input| {
            let raw = take(input, 8)?;
            Ok(u64::from_le_bytes(raw.try_into().expect("8 bytes")))
        });
        assert!(matches!(
            result,
            Err(CodecError::Corruption(Corruption::Truncated { .. }))
        ));
    }

    #[test]
    fn write_vec_rejects_zero_width_elements() {
        let mut out = Vec::new();
        assert!(matches!(
            write_vec(&[(), ()], &mut out, |_item, _o| Ok(())),
            Err(CodecError::Encode(EncodeInvariant::ZeroWidthVecElement))
        ));
    }

    #[test]
    fn vec_zero_progress_element_is_rejected() {
        // 아무것도 소비하지 않는 element 디코더 + 거대 count → 루프/자원 고갈 대신 즉시 에러
        let mut bytes = Vec::new();
        write_varint(u64::MAX, &mut bytes);
        bytes.push(0xaa);
        let mut slice = &bytes[..];
        let result = read_vec(&mut slice, |_input| Ok(0u8));
        assert!(matches!(
            result,
            Err(CodecError::Corruption(Corruption::NoProgress))
        ));
    }
}
