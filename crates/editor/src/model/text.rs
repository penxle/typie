use crate::model::{Codec, Mark, MarkType};
use anyhow::{Context, Result};
use loro::{LoroMap, LoroText, LoroValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

pub struct Text {
    loro_text: LoroText,
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        let self_segments = self.get_rich_text_segments();
        let other_segments = other.get_rich_text_segments();

        if self_segments.len() != other_segments.len() {
            return false;
        }

        self_segments
            .iter()
            .zip(other_segments.iter())
            .all(|((t1, m1), (t2, m2))| {
                if t1 != t2 || m1.len() != m2.len() {
                    return false;
                }
                let mut m1_sorted: Vec<_> = m1.iter().collect();
                let mut m2_sorted: Vec<_> = m2.iter().collect();
                m1_sorted.sort_by_key(|m| m.as_type());
                m2_sorted.sort_by_key(|m| m.as_type());
                m1_sorted == m2_sorted
            })
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let segments = self.get_rich_text_segments();
        segments.len().hash(state);
        for (text, marks) in segments {
            text.hash(state);
            marks.len().hash(state);
            let mut sorted_marks: Vec<_> = marks.iter().collect();
            sorted_marks.sort_by_key(|m| m.as_type());
            for mark in sorted_marks {
                mark.hash(state);
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

    pub fn from<S: Into<String>>(s: S) -> Self {
        let loro_text = LoroText::new();
        let _ = loro_text.insert(0, &s.into());
        Text { loro_text }
    }

    pub fn from_segments(segments: &[(String, Vec<Mark>)]) -> Self {
        let text = Self::new();
        let mut total_text = String::new();
        for (content, _) in segments {
            total_text.push_str(content);
        }
        text.insert(0, &total_text);

        let mut offset = 0;
        for (content, marks) in segments {
            let len = content.chars().count();
            for mark in marks {
                let _ = text.mark(offset..offset + len, mark);
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
        let segments = self.get_rich_text_segments();
        let result = Text::new();
        let mut current_offset = 0;
        let mut segment_ranges = Vec::new();

        for (segment_text, segment_marks) in segments {
            let segment_len = segment_text.chars().count();
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

            let chars: Vec<char> = segment_text.chars().collect();
            let sliced_text: String = chars[slice_start..slice_end].iter().collect();

            if !sliced_text.is_empty() {
                let start = result.char_len();
                result.insert(start, &sliced_text);
                let end = result.char_len();
                segment_ranges.push((start..end, segment_marks));
            }

            current_offset = segment_end;
        }

        for (range, marks) in segment_ranges {
            for mark in marks {
                let _ = result.mark(range.clone(), &mark);
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

    pub fn mark(&self, range: std::ops::Range<usize>, mark: &Mark) -> anyhow::Result<()> {
        if mark.is_default() {
            return self.unmark(range, mark.as_type());
        }
        let key = mark.key();
        let value = mark.to_loro_value();
        self.loro_text
            .mark(range, key, value)
            .context("Failed to apply mark")
    }

    pub fn unmark(&self, range: std::ops::Range<usize>, mark_type: MarkType) -> anyhow::Result<()> {
        let key = mark_type.key();
        self.loro_text
            .mark(range, key, LoroValue::Null)
            .context("Failed to remove mark")
    }

    pub fn get_rich_text_segments(&self) -> Vec<(String, Vec<Mark>)> {
        let rich_value = self.loro_text.get_richtext_value();

        let mut segments = Vec::new();

        if let LoroValue::List(list) = rich_value {
            for item in list.iter() {
                if let LoroValue::Map(map) = item {
                    let text = map
                        .get("insert")
                        .and_then(|v| v.as_string())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let mut marks = Vec::new();
                    if let Some(attrs_value) = map.get("attributes") {
                        if let LoroValue::Map(attrs) = attrs_value {
                            for (key, value) in attrs.iter() {
                                if let Some(mark) = Mark::from_key_value(&key, value.clone()) {
                                    marks.push(mark);
                                }
                            }
                        }
                    }

                    segments.push((text, marks));
                }
            }
        }

        segments
    }
}

impl Clone for Text {
    fn clone(&self) -> Self {
        let loro_text = LoroText::new();
        let segments = self.get_rich_text_segments();

        let mut segment_ranges = Vec::new();
        for (text, marks) in &segments {
            let start = loro_text.len_unicode();
            let _ = loro_text.insert(start, text);
            let end = loro_text.len_unicode();
            segment_ranges.push((start..end, marks.clone()));
        }

        for (range, marks) in segment_ranges {
            for mark in marks {
                let _ = loro_text.mark(range.clone(), mark.key(), mark.to_loro_value());
            }
        }

        Text { loro_text }
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
struct TextSegment {
    text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    marks: Vec<Mark>,
}

impl Serialize for Text {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let segments: Vec<TextSegment> = self
            .get_rich_text_segments()
            .into_iter()
            .map(|(text, marks)| TextSegment { text, marks })
            .collect();
        segments.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let segments: Vec<TextSegment> = Vec::deserialize(deserializer)?;
        let text = Text::new();
        for segment in segments {
            let start = text.char_len();
            let end = start + segment.text.chars().count();
            text.insert(start, &segment.text);
            for mark in segment.marks {
                let _ = text.mark(start..end, &mark);
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
