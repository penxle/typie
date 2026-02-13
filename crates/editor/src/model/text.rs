use crate::model::{Annotation, Codec, Style, StyleType};
use anyhow::{Context, Result};
use loro::{LoroMap, LoroText, LoroValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub struct TextSegment {
    pub text: String,
    pub styles: Vec<Style>,
    pub annotations: Vec<Annotation>,
}

pub struct Text {
    loro_text: LoroText,
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        let self_segments = self.get_segments();
        let other_segments = other.get_segments();

        if self_segments.len() != other_segments.len() {
            return false;
        }

        self_segments
            .iter()
            .zip(other_segments.iter())
            .all(|(s1, s2)| {
                if s1.text != s2.text || s1.styles.len() != s2.styles.len() {
                    return false;
                }
                let mut m1_sorted: Vec<_> = s1.styles.iter().collect();
                let mut m2_sorted: Vec<_> = s2.styles.iter().collect();
                m1_sorted.sort_by_key(|m| m.as_type());
                m2_sorted.sort_by_key(|m| m.as_type());
                m1_sorted == m2_sorted
                    && s1.annotations.len() == s2.annotations.len()
                    && s1.annotations.iter().all(|a| s2.annotations.contains(a))
            })
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let segments = self.get_segments();
        segments.len().hash(state);
        for seg in segments {
            seg.text.hash(state);
            seg.styles.len().hash(state);
            let mut sorted: Vec<_> = seg.styles.iter().collect();
            sorted.sort_by_key(|s| s.as_type());
            for style in sorted {
                style.hash(state);
            }
            seg.annotations.len().hash(state);
            for ann in &seg.annotations {
                ann.hash(state);
            }
        }
    }
}

impl Text {
    pub fn new() -> Self {
        Text {
            loro_text: LoroText::new(),
        }
    }

    pub fn into_loro_text(&self) -> LoroText {
        self.loro_text.clone()
    }

    pub fn from<S: Into<String>>(s: S) -> Self {
        let loro_text = LoroText::new();
        let _ = loro_text.insert(0, &s.into());
        Text { loro_text }
    }

    pub fn from_segments(segments: &[TextSegment]) -> Self {
        let text = Self::new();
        let mut total_text = String::new();
        for seg in segments {
            total_text.push_str(&seg.text);
        }
        text.insert(0, &total_text);

        let mut offset = 0;
        for seg in segments {
            let len = seg.text.chars().count();
            let range = offset..offset + len;
            for style in &seg.styles {
                let _ = text.apply_style(range.clone(), style);
            }
            for ann in &seg.annotations {
                let _ = text.apply_annotation(range.clone(), ann);
            }
            offset += len;
        }
        text
    }

    pub fn as_str(&self) -> String {
        self.loro_text.to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.loro_text.is_empty()
    }

    pub fn len(&self) -> usize {
        self.loro_text.len_utf8()
    }

    pub fn char_len(&self) -> usize {
        self.loro_text.len_unicode()
    }

    pub fn char_to_byte(&self, char_offset: usize) -> usize {
        self.loro_text
            .to_string()
            .char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.loro_text.len_utf8())
    }

    pub fn byte_to_char(&self, byte_offset: usize) -> usize {
        let s = self.loro_text.to_string();
        bytecount::num_chars(&s.as_bytes()[..byte_offset])
    }

    pub fn split_at(&self, char_offset: usize) -> (Text, Text) {
        let len = self.char_len();
        let left = self.slice(0, char_offset);
        let right = self.slice(char_offset, len);
        (left, right)
    }

    pub fn slice(&self, from_char: usize, to_char: usize) -> Text {
        let segments = self.get_segments();
        let result = Text::new();
        let mut current_offset = 0;
        let mut segment_ranges = Vec::new();

        for seg in segments {
            let segment_len = seg.text.chars().count();
            let segment_end = current_offset + segment_len;

            if segment_end <= from_char {
                current_offset = segment_end;
                continue;
            }

            if current_offset >= to_char {
                break;
            }

            let slice_start = if current_offset < from_char {
                from_char - current_offset
            } else {
                0
            };

            let slice_end = if segment_end > to_char {
                to_char - current_offset
            } else {
                segment_len
            };

            let chars: Vec<char> = seg.text.chars().collect();
            let sliced_text: String = chars[slice_start..slice_end].iter().collect();

            if !sliced_text.is_empty() {
                let start = result.char_len();
                result.insert(start, &sliced_text);
                let end = result.char_len();
                segment_ranges.push((start..end, seg.styles, seg.annotations));
            }

            current_offset = segment_end;
        }

        for (range, styles, annotations) in segment_ranges {
            for style in &styles {
                let _ = result.apply_style(range.clone(), style);
            }
            for ann in &annotations {
                let _ = result.apply_annotation(range.clone(), ann);
            }
        }

        result
    }

    pub fn insert(&self, char_offset: usize, text: &str) {
        let _ = self.loro_text.insert(char_offset, text);
    }

    pub fn delete(&self, from: usize, to: usize) {
        let _ = self.loro_text.delete(from, to - from);
    }

    pub fn splice(&self, from: usize, to: usize, text: &str) -> String {
        self.loro_text
            .splice(from, to - from, text)
            .unwrap_or_default()
    }

    pub fn replace(&self, from: usize, to: usize, text: &str) {
        let _ = self.loro_text.splice(from, to - from, text);
    }

    pub fn set(&self, s: &str) {
        let len = self.char_len();
        if len == 0 {
            let _ = self.loro_text.insert(0, s);
        } else {
            let _ = self.loro_text.splice(0, len, s);
        }
    }

    pub fn clear(&self) {
        let len = self.char_len();
        if len > 0 {
            let _ = self.loro_text.delete(0, len);
        }
    }

    pub fn truncate(&self, from_char: usize) {
        let len = self.char_len();
        if from_char < len {
            let _ = self.loro_text.delete(from_char, len - from_char);
        }
    }

    pub fn apply_style(&self, range: std::ops::Range<usize>, style: &Style) -> Result<()> {
        let key = style.key();
        let value = style.to_loro_value();
        self.loro_text
            .mark(range, key, value)
            .context("Failed to apply style")
    }

    pub fn apply_annotation(
        &self,
        range: std::ops::Range<usize>,
        annotation: &Annotation,
    ) -> Result<()> {
        let key = annotation.key();
        let value = annotation.to_loro_value();
        self.loro_text
            .mark(range, key, value)
            .context("Failed to apply annotation")
    }

    pub fn remove_annotation(
        &self,
        range: std::ops::Range<usize>,
        ann_type: crate::model::AnnotationType,
    ) -> Result<()> {
        let key = ann_type.key();
        self.loro_text
            .mark(range, key, LoroValue::Null)
            .context("Failed to remove annotation")
    }

    pub fn remove_style(&self, range: std::ops::Range<usize>, style_type: StyleType) -> Result<()> {
        let key = style_type.key();
        self.loro_text
            .mark(range, key, LoroValue::Null)
            .context("Failed to remove style")
    }

    pub fn get_segments(&self) -> Vec<TextSegment> {
        let rich_value = self.loro_text.get_richtext_value();
        let mut segments = Vec::new();
        if let LoroValue::List(list) = rich_value {
            for item in list.iter() {
                if let LoroValue::Map(map) = item {
                    let text = match map.get("insert") {
                        Some(LoroValue::String(s)) => s.to_string(),
                        _ => continue,
                    };
                    let mut styles = Vec::new();
                    let mut annotations = Vec::new();
                    if let Some(LoroValue::Map(attrs)) = map.get("attributes") {
                        for (key, value) in attrs.iter() {
                            if matches!(value, LoroValue::Null) {
                                continue;
                            }
                            if let Some(style) = Style::from_key_value(key, value.clone()) {
                                styles.push(style);
                            } else if let Some(ann) = Annotation::from_key_value(key, value) {
                                annotations.push(ann);
                            }
                        }
                    }
                    segments.push(TextSegment {
                        text,
                        styles,
                        annotations,
                    });
                }
            }
        }
        segments
    }
}

impl Clone for Text {
    fn clone(&self) -> Self {
        Text::from_segments(&self.get_segments())
    }
}

impl Default for Text {
    fn default() -> Self {
        Text::new()
    }
}

impl Debug for Text {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Text")
            .field("content", &self.loro_text.to_string())
            .finish()
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.loro_text.to_string())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::from(s)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::from(s.to_string())
    }
}

impl From<Text> for String {
    fn from(text: Text) -> String {
        text.loro_text.to_string()
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedSegment {
    text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    styles: Vec<Style>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    annotations: Vec<Annotation>,
}

impl Serialize for Text {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let segments: Vec<SerializedSegment> = self
            .get_segments()
            .into_iter()
            .map(|seg| SerializedSegment {
                text: seg.text,
                styles: seg.styles,
                annotations: seg.annotations,
            })
            .collect();
        segments.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let segments: Vec<SerializedSegment> = Vec::deserialize(deserializer)?;
        let text = Text::new();
        let mut ranges = Vec::new();
        for segment in &segments {
            let start = text.char_len();
            text.insert(start, &segment.text);
            let end = text.char_len();
            ranges.push((start..end, &segment.styles, &segment.annotations));
        }
        for (range, styles, annotations) in ranges {
            for style in styles {
                let _ = text.apply_style(range.clone(), style);
            }
            for ann in annotations {
                let _ = text.apply_annotation(range.clone(), ann);
            }
        }
        Ok(text)
    }
}

impl Codec for Text {
    fn encode_field(&mut self, map: &LoroMap, key: &str) -> Result<()> {
        if self.loro_text.is_attached() {
            return Ok(());
        }

        self.loro_text = map
            .insert_container(key, self.loro_text.clone())
            .context("failed to insert text container")?;

        Ok(())
    }

    fn decode_field(map: &LoroMap, key: &str) -> Result<Self> {
        let loro_text = map
            .get(key)
            .context("text field not found")?
            .into_container()
            .ok()
            .context("text is not a container")?
            .into_text()
            .ok()
            .context("text is not a LoroText container")?;

        Ok(Text { loro_text })
    }
}
