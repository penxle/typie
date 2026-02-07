#[cfg(feature = "wasm")]
extern crate web_sys;

#[allow(unused)]
macro_rules! log {
    ( $( $t:tt )* ) => {
        {
            #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
            {
                web_sys::console::log_1(&format!( $( $t )* ).into());
            }
            #[cfg(feature = "native")]
            {
                crate::ffi::native::native_log(crate::ffi::native::LOG_LEVEL_INFO, &format!( $( $t )* ));
            }
            #[cfg(not(any(all(feature = "wasm", target_arch = "wasm32"), feature = "native")))]
            {
                println!( $( $t )* );
            }
        }
    }
}

#[allow(unused)]
macro_rules! error {
    ( $( $t:tt )* ) => {
        {
            #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
            {
                web_sys::console::error_1(&format!( $( $t )* ).into());
            }
            #[cfg(feature = "native")]
            {
                crate::ffi::native::native_log(crate::ffi::native::LOG_LEVEL_ERROR, &format!( $( $t )* ));
            }
            #[cfg(not(any(all(feature = "wasm", target_arch = "wasm32"), feature = "native")))]
            {
                eprintln!( $( $t )* );
            }
        }
    }
}

#[allow(unused)]
macro_rules! warn {
    ( $( $t:tt )* ) => {
        {
            #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
            {
                web_sys::console::warn_1(&format!( $( $t )* ).into());
            }
            #[cfg(feature = "native")]
            {
                crate::ffi::native::native_log(crate::ffi::native::LOG_LEVEL_WARN, &format!( $( $t )* ));
            }
            #[cfg(not(any(all(feature = "wasm", target_arch = "wasm32"), feature = "native")))]
            {
                eprintln!( $( $t )* );
            }
        }
    }
}

pub fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    bytecount::num_chars(text[..text.floor_char_boundary(byte_offset)].as_bytes())
}

pub fn char_to_byte_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(byte_offset, _)| byte_offset)
        .unwrap_or(text.len())
}

pub fn compute_word_boundaries(text: &str) -> Vec<usize> {
    use icu_provider::buf::AsDeserializingBufferProvider;
    use icu_segmenter::WordSegmenter;
    use icu_segmenter::options::WordBreakOptions;

    let provider = crate::icu_data::get_icu_provider();
    let deserializing_provider = provider.as_deserializing();
    let segmenter = WordSegmenter::try_new_dictionary_unstable(
        &deserializing_provider,
        WordBreakOptions::default(),
    )
    .expect("Failed to create WordSegmenter");

    segmenter
        .as_borrowed()
        .segment_str(text)
        .map(|byte_offset| byte_to_char_offset(text, byte_offset))
        .collect()
}

pub fn compute_sentence_boundaries(text: &str) -> Vec<usize> {
    use icu_provider::buf::AsDeserializingBufferProvider;
    use icu_segmenter::SentenceSegmenter;

    let provider = crate::icu_data::get_icu_provider();
    let deserializing_provider = provider.as_deserializing();
    let segmenter =
        SentenceSegmenter::try_new_unstable(&deserializing_provider, Default::default())
            .expect("Failed to create SentenceSegmenter");

    segmenter
        .as_borrowed()
        .segment_str(text)
        .map(|byte_offset| byte_to_char_offset(text, byte_offset))
        .collect()
}

pub fn compute_grapheme_boundaries(text: &str) -> Vec<usize> {
    use icu_provider::buf::AsDeserializingBufferProvider;
    use icu_segmenter::GraphemeClusterSegmenter;

    let provider = crate::icu_data::get_icu_provider();
    let deserializing_provider = provider.as_deserializing();
    let segmenter = GraphemeClusterSegmenter::try_new_unstable(&deserializing_provider)
        .expect("Failed to create GraphemeClusterSegmenter");

    segmenter
        .as_borrowed()
        .segment_str(text)
        .map(|byte_offset| byte_to_char_offset(text, byte_offset))
        .collect()
}

pub fn find_prev_grapheme_boundary(text: &str, offset: usize) -> usize {
    let grapheme_boundaries = compute_grapheme_boundaries(text);

    let idx = grapheme_boundaries.partition_point(|&boundary| boundary < offset);

    if idx > 0 {
        grapheme_boundaries[idx - 1]
    } else {
        0
    }
}

pub fn find_next_grapheme_boundary(text: &str, offset: usize) -> usize {
    let grapheme_boundaries = compute_grapheme_boundaries(text);

    let idx = grapheme_boundaries.partition_point(|&boundary| boundary <= offset);

    if idx < grapheme_boundaries.len() {
        grapheme_boundaries[idx]
    } else {
        text.chars().count()
    }
}

pub fn resolve_affinity_boundary(
    left_is_hard_break: bool,
    right_is_hard_break: bool,
    default_affinity: crate::types::Affinity,
) -> crate::types::Affinity {
    match (left_is_hard_break, right_is_hard_break) {
        (true, true) => crate::types::Affinity::Downstream,
        (true, false) => crate::types::Affinity::Downstream,
        (false, true) => crate::types::Affinity::Upstream,
        (false, false) => default_affinity,
    }
}

pub fn resolve_explicit_break_line_end(
    current_offset: usize,
    end_offset: usize,
    current_affinity: crate::types::Affinity,
) -> Option<(usize, crate::types::Affinity)> {
    if end_offset == 0 {
        return None;
    }

    let break_offset = end_offset.saturating_sub(1);

    if current_offset == break_offset && current_affinity == crate::types::Affinity::Upstream {
        Some((current_offset, current_affinity))
    } else {
        Some((break_offset, crate::types::Affinity::Upstream))
    }
}

pub fn rgba_from_u32(color_u32: u32) -> [u8; 4] {
    let r = ((color_u32 >> 24) & 0xFF) as u8;
    let g = ((color_u32 >> 16) & 0xFF) as u8;
    let b = ((color_u32 >> 8) & 0xFF) as u8;
    let a = (color_u32 & 0xFF) as u8;
    [r, g, b, a]
}

pub fn collect_codepoints(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    Px,
    Pt,
    Em,
}

pub fn convert_length(value: f32, from: LengthUnit, to: LengthUnit) -> f32 {
    if from == to {
        return value;
    }

    const PT_TO_PX: f32 = 96.0 / 72.0;
    const DEFAULT_FONT_SIZE_PX: f32 = 16.0;

    let px = match from {
        LengthUnit::Px => value,
        LengthUnit::Pt => value * PT_TO_PX,
        LengthUnit::Em => value * DEFAULT_FONT_SIZE_PX,
    };

    match to {
        LengthUnit::Px => px,
        LengthUnit::Pt => px / PT_TO_PX,
        LengthUnit::Em => px / DEFAULT_FONT_SIZE_PX,
    }
}
