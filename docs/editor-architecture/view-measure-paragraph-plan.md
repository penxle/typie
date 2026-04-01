# Paragraph → TextBlock 측정 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Paragraph 노드의 자식 Text 노드들을 parley로 텍스트 셰이핑하여 MeasuredContent::TextBlock을 생성한다.

**Architecture:** 4단계 파이프라인(collect_text_runs → resolve_font_mapping → build_parley_layout → extract_measured_lines)으로 분해. ShapingContext(FontContext + LayoutContext + FontRegistry)를 Arc\<Mutex\>로 에디터 간 공유. CSS 표준 strut 모델로 최소 line height 보장.

**Tech Stack:** parley 0.7 (텍스트 셰이핑/줄바꿈), editor-model (Doc, Modifier), editor-common (FontRegistry)

**설계 문서:** `docs/editor-architecture/view-measure-paragraph-design.md`

**주의:** 이 프로젝트는 git commit을 에이전트가 직접 수행하지 않습니다. 사용자가 수동으로 커밋합니다.

---

## 파일 구조

| 동작 | 파일 | 역할 |
|------|------|------|
| Modify | `crates/editor-common/src/font.rs` | FontRegistry에 family interning 추가 |
| Create | `crates/editor-view/src/shaping.rs` | ShapingContext, TextBrush, StrutMetrics 정의 (font_mappings 포함) |
| Modify | `crates/editor-view/src/lib.rs` | `shaping` 모듈 선언 |
| Modify | `crates/editor-view/Cargo.toml` | parley 의존성 추가 |
| Modify | `crates/editor-view/src/engine/resolve.rs` | resolve_text_style, resolve_paragraph_indent 추가 |
| Create | `crates/editor-view/src/engine/measure_nodes/paragraph/mod.rs` | measure_paragraph 조립 + 하위 모듈 re-export |
| Create | `crates/editor-view/src/engine/measure_nodes/paragraph/text_run.rs` | TextRun, collect_text_runs |
| Create | `crates/editor-view/src/engine/measure_nodes/paragraph/font_run.rs` | FontRun, resolve_font_mapping (문자별 매핑) |
| Create | `crates/editor-view/src/engine/measure_nodes/paragraph/layout_builder.rs` | build_parley_layout |
| Create | `crates/editor-view/src/engine/measure_nodes/paragraph/line_extraction.rs` | extract_measured_lines |
| Modify | `crates/editor-view/src/engine/measure_nodes/mod.rs` | paragraph 모듈 및 re-export 추가 |
| Modify | `crates/editor-view/src/engine/mod.rs` | LayoutEngine에 shaping 필드 추가, measure_inner에 Paragraph 분기 추가 |

---

## Task 1: Dependencies와 ShapingContext 타입

**Files:**
- Modify: `crates/editor-view/Cargo.toml`
- Create: `crates/editor-view/src/shaping.rs`
- Modify: `crates/editor-view/src/lib.rs`

- [ ] **Step 1: Cargo.toml에 parley 의존성 추가**

`crates/editor-view/Cargo.toml`의 `[dependencies]` 섹션에 추가:

```toml
parley = { git = "https://github.com/user/parley", rev = "..." }
```

정확한 git URL과 rev는 `crates/editor/Cargo.toml`의 기존 parley 의존성과 동일하게 맞춘다. workspace Cargo.toml에 parley가 정의되어 있다면 `parley = { workspace = true }`를 사용한다.

- [ ] **Step 2: shaping.rs 작성**

```rust
// crates/editor-view/src/shaping.rs
use std::collections::HashMap;

use editor_common::FontRegistry;
use editor_model::NodeId;
use parley::{FontContext, LayoutContext};
use rustc_hash::FxHashMap;

/// parley Brush 타입. glyph run에서 원본 Text 노드를 역추적하는 용도.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextBrush {
    pub node_id: NodeId,
}

pub struct ShapingContext {
    pub fcx: FontContext,
    pub lcx: LayoutContext<TextBrush>,
    pub font_registry: FontRegistry,
}

impl ShapingContext {
    pub fn new() -> Self {
        Self {
            fcx: FontContext::new(),
            lcx: LayoutContext::new(),
            font_registry: FontRegistry::new(),
        }
    }
}

pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
}
```

TextBrush의 derive 목록은 parley의 `Brush` trait bound에 맞춰야 한다. 컴파일 시 추가 trait이 필요하면 derive에 추가한다 (예: Hash, Eq).

- [ ] **Step 3: lib.rs에 모듈 선언 추가**

`crates/editor-view/src/lib.rs`에 추가:

```rust
pub mod shaping;
```

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공. shaping.rs의 타입들이 정상적으로 컴파일되는지 확인.

---

## Task 2: resolve_text_style

**Files:**
- Modify: `crates/editor-view/src/engine/resolve.rs`

- [ ] **Step 1: 테스트 작성**

`crates/editor-view/src/engine/resolve.rs`의 `mod tests` 안에 추가:

```rust
#[test]
fn resolve_text_style_from_self() {
    let (doc, t1) = doc! {
        root {
            paragraph {
                t1: text("hello") [font_size(2400), font_weight(700)]
            }
        }
    };

    let node = doc.node(t1).unwrap();
    let style = resolve_text_style(&node);

    assert_eq!(style.font_weight, 700);
    // 2400 centiunits = 24pt → 24 * 96/72 = 32px
    assert!((style.font_size - 32.0).abs() < 0.01);
}

#[test]
fn resolve_text_style_inherits_from_ancestor() {
    let (doc, t1) = doc! {
        root [font_size(1600), line_height(200)] {
            paragraph {
                t1: text("hello")
            }
        }
    };

    let node = doc.node(t1).unwrap();
    let style = resolve_text_style(&node);

    // 1600 centiunits = 16pt → 16 * 96/72 ≈ 21.33px
    assert!((style.font_size - 21.333).abs() < 0.01);
    assert!((style.line_height - 2.0).abs() < 0.01);
}

#[test]
fn resolve_text_style_defaults_when_absent() {
    let (doc, t1) = doc! {
        root {
            paragraph {
                t1: text("hello")
            }
        }
    };

    let node = doc.node(t1).unwrap();
    let style = resolve_text_style(&node);

    assert_eq!(style.font_weight, 400);
    assert!((style.font_size - 16.0).abs() < 0.01); // default 16px
    assert!((style.line_height - 1.6).abs() < 0.01); // default 160%
    assert!((style.letter_spacing - 0.0).abs() < 0.01);
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view resolve_text_style`
Expected: 컴파일 에러 — `resolve_text_style` 함수가 존재하지 않음.

- [ ] **Step 3: ResolvedTextStyle 및 resolve_text_style 구현**

`crates/editor-view/src/engine/resolve.rs`에 추가:

```rust
pub struct ResolvedTextStyle {
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub line_height: f32,
}

const DEFAULT_FONT_SIZE_PX: f32 = 16.0;
const DEFAULT_FONT_WEIGHT: u16 = 400;
const DEFAULT_LINE_HEIGHT: f32 = 1.6;
const PT_TO_PX: f32 = 96.0 / 72.0;

pub fn resolve_text_style(node: &NodeRef<'_>) -> ResolvedTextStyle {
    let mut font_family: Option<String> = None;
    let mut font_weight: Option<u16> = None;
    let mut font_size: Option<f32> = None;
    let mut letter_spacing: Option<f32> = None;
    let mut line_height: Option<f32> = None;

    let mut resolved_count = 0u8;
    const TOTAL_PROPERTIES: u8 = 5;

    // self + 조상 체인을 한 번에 순회하는 클로저
    let mut visit = |modifiers: &[Modifier]| {
        for m in modifiers {
            match m {
                Modifier::FontFamily(f) if font_family.is_none() => {
                    font_family = Some(f.clone());
                    resolved_count += 1;
                }
                Modifier::FontWeight(w) if font_weight.is_none() => {
                    font_weight = Some(*w);
                    resolved_count += 1;
                }
                Modifier::FontSize(s) if font_size.is_none() => {
                    let pt = *s as f32 / 100.0;
                    font_size = Some(pt * PT_TO_PX);
                    resolved_count += 1;
                }
                Modifier::LetterSpacing(ls) if letter_spacing.is_none() => {
                    letter_spacing = Some(*ls as f32 / 100.0);
                    resolved_count += 1;
                }
                Modifier::LineHeight(lh) if line_height.is_none() => {
                    line_height = Some(*lh as f32 / 100.0);
                    resolved_count += 1;
                }
                _ => {}
            }
        }
    };

    visit(node.modifiers());
    for ancestor in node.ancestors() {
        if resolved_count >= TOTAL_PROPERTIES {
            break;
        }
        visit(ancestor.modifiers());
    }

    let final_font_size = font_size.unwrap_or(DEFAULT_FONT_SIZE_PX);
    let ls_em = letter_spacing.unwrap_or(0.0);

    ResolvedTextStyle {
        font_family: font_family.unwrap_or_default(),
        font_weight: font_weight.unwrap_or(DEFAULT_FONT_WEIGHT),
        font_size: final_font_size,
        letter_spacing: ls_em * final_font_size,
        line_height: line_height.unwrap_or(DEFAULT_LINE_HEIGHT),
    }
}

pub fn resolve_paragraph_indent(node: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node, ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent(v)) => *v as f32 / 100.0 * DEFAULT_FONT_SIZE_PX,
        _ => 0.0,
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view resolve_text_style`
Expected: 3개 테스트 모두 PASS.

---

## Task 3: TextRun과 collect_text_runs

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph/text_run.rs`
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph/mod.rs`
- Modify: `crates/editor-view/src/engine/measure_nodes/mod.rs`

- [ ] **Step 1: 모듈 구조 생성**

`crates/editor-view/src/engine/measure_nodes/paragraph/mod.rs`:

```rust
mod text_run;
mod font_run;
mod layout_builder;
mod line_extraction;

pub use text_run::*;
pub use font_run::*;
pub use layout_builder::*;
pub use line_extraction::*;
```

아직 존재하지 않는 하위 모듈(font_run, layout_builder, line_extraction)은 빈 파일로 생성하고, 이후 Task에서 채운다.

`crates/editor-view/src/engine/measure_nodes/mod.rs`에 추가:

```rust
pub(crate) mod paragraph;
```

- [ ] **Step 2: 테스트 작성**

`crates/editor-view/src/engine/measure_nodes/paragraph/text_run.rs`:

```rust
use std::ops::Range;

use editor_model::{Doc, NodeId, NodeRef};

use crate::engine::resolve::ResolvedTextStyle;

pub struct TextRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub style: ResolvedTextStyle,
}

pub fn collect_text_runs(doc: &Doc, paragraph: &NodeRef<'_>) -> (String, Vec<TextRun>) {
    todo!()
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

    use super::*;

    #[test]
    fn single_text_node() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("hello")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        assert_eq!(text, "hello");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].byte_range, 0..5);
    }

    #[test]
    fn multiple_text_nodes() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("hello")
                    text(" world")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        assert_eq!(text, "hello world");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].byte_range, 0..5);
        assert_eq!(runs[1].byte_range, 5..11);
    }

    #[test]
    fn text_node_with_modifiers() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("normal")
                    text("big") [font_size(2400)]
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        assert_eq!(text, "normalbig");
        assert_eq!(runs.len(), 2);
        // "big"의 font_size: 2400 centiunits = 24pt = 32px
        assert!((runs[1].style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn empty_paragraph() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        assert!(text.is_empty());
        assert!(runs.is_empty());
    }
}
```

- [ ] **Step 3: 테스트 실패 확인**

Run: `cargo test -p editor-view text_run`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 4: collect_text_runs 구현**

`text_run.rs`의 `collect_text_runs` 함수를 구현:

```rust
use crate::engine::resolve::resolve_text_style;

pub fn collect_text_runs(doc: &Doc, paragraph: &NodeRef<'_>) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();

    for child in paragraph.children() {
        if let editor_model::Node::Text(text_node) = child.node() {
            let start = text.len();
            text.push_str(&text_node.text);
            let end = text.len();

            if start < end {
                runs.push(TextRun {
                    node_id: child.id(),
                    byte_range: start..end,
                    style: resolve_text_style(&child),
                });
            }
        }
    }

    (text, runs)
}
```

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p editor-view text_run`
Expected: 4개 테스트 모두 PASS.

---

## Task 4: FontRun과 resolve_font_mapping

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph/font_run.rs`

- [ ] **Step 1: 테스트 작성 및 stub 구현**

```rust
// crates/editor-view/src/engine/measure_nodes/paragraph/font_run.rs
use std::ops::Range;

use editor_common::FontRegistry;

use super::text_run::TextRun;

pub struct FontRun {
    pub byte_range: Range<usize>,
    pub family: String,
    pub weight: u16,
}

pub fn resolve_font_mapping(
    text: &str,
    runs: &[TextRun],
    font_registry: &FontRegistry,
) -> Vec<FontRun> {
    todo!()
}

#[cfg(test)]
mod tests {
    use editor_common::FontRegistry;
    use editor_macros::doc;
    use editor_model::*;

    use super::*;
    use crate::engine::measure_nodes::paragraph::text_run::collect_text_runs;

    fn registry_with_families(families: &[&str]) -> FontRegistry {
        FontRegistry::from_families(
            families.iter().map(|f| (f.to_string(), vec![400, 700])),
        )
    }

    #[test]
    fn single_run_known_family() {
        let (doc, p1) = doc! {
            root [font_family("Arial")] {
                p1: paragraph {
                    text("hello")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let registry = registry_with_families(&["Arial"]);
        let font_runs = resolve_font_mapping(&text, &runs, &registry);

        assert_eq!(font_runs.len(), 1);
        assert_eq!(font_runs[0].family, "Arial");
        assert_eq!(font_runs[0].byte_range, 0..5);
    }

    #[test]
    fn unknown_family_uses_fallback() {
        let (doc, p1) = doc! {
            root [font_family("UnknownFont")] {
                p1: paragraph {
                    text("hello")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let registry = FontRegistry::new();
        let font_runs = resolve_font_mapping(&text, &runs, &registry);

        // unknown family → 그대로 전달 (parley가 fallback 처리)
        assert_eq!(font_runs.len(), 1);
        assert_eq!(font_runs[0].byte_range, 0..5);
    }

    #[test]
    fn adjacent_runs_same_family_merged() {
        let (doc, p1) = doc! {
            root [font_family("Arial")] {
                p1: paragraph {
                    text("hello")
                    text(" world")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let registry = registry_with_families(&["Arial"]);
        let font_runs = resolve_font_mapping(&text, &runs, &registry);

        assert_eq!(font_runs.len(), 1);
        assert_eq!(font_runs[0].byte_range, 0..11);
    }

    #[test]
    fn codepoint_mapping_splits_run() {
        let (doc, p1) = doc! {
            root [font_family("Pretendard")] {
                p1: paragraph {
                    text("A한B")
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        let mut registry = registry_with_families(&["Pretendard", "Paperlogy"]);
        let pretendard_id = registry.intern("Pretendard");
        let paperlogy_id = registry.intern("Paperlogy");

        // '한' (U+D55C)을 Paperlogy/700으로 매핑
        registry.add_codepoint_mapping(pretendard_id, 400, '한' as u32, paperlogy_id, 700);

        let font_runs = resolve_font_mapping(&text, &runs, &registry);

        assert_eq!(font_runs.len(), 3);
        assert_eq!(font_runs[0].family, "Pretendard"); // "A"
        assert_eq!(font_runs[1].family, "Paperlogy");  // "한"
        assert_eq!(font_runs[1].weight, 700);
        assert_eq!(font_runs[2].family, "Pretendard"); // "B"
    }
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cargo test -p editor-view font_run`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 3: resolve_font_mapping 구현**

```rust
use rustc_hash::FxHashMap;

use crate::shaping::FontMappings;

pub fn resolve_font_mapping(
    text: &str,
    runs: &[TextRun],
    font_registry: &FontRegistry,
) -> Vec<FontRun> {
    let mut font_runs: Vec<FontRun> = Vec::new();

    for run in runs {
        let family_id = font_registry.intern_id(&run.style.font_family);
        let weight = run.style.font_weight;

        // 이 (family, weight) 조합의 codepoint 매핑 테이블을 가져옴
        let cp_map = family_id.and_then(|fid| font_registry.codepoint_map(fid, weight));

        // TextRun 내 문자별로 매핑 lookup
        let run_text = &text[run.byte_range.clone()];
        let mut byte_offset = run.byte_range.start;

        for ch in run_text.chars() {
            let char_bytes = ch.len_utf8();
            let char_byte_end = byte_offset + char_bytes;

            // codepoint 매핑 테이블에서 lookup
            let (resolved_family, resolved_weight) = cp_map
                .and_then(|m| m.get(&(ch as u32)))
                .map(|&(fid, w)| (font_registry.resolve(fid).to_string(), w))
                .unwrap_or_else(|| {
                    // 매핑 없음 → nearest weight로 fallback
                    let w = font_registry
                        .nearest_weight(&run.style.font_family, weight)
                        .unwrap_or(weight);
                    (run.style.font_family.clone(), w)
                });

            let can_merge = font_runs.last().is_some_and(|last: &FontRun| {
                last.family == resolved_family
                    && last.weight == resolved_weight
                    && last.byte_range.end == byte_offset
            });

            if can_merge {
                font_runs.last_mut().unwrap().byte_range.end = char_byte_end;
            } else {
                font_runs.push(FontRun {
                    byte_range: byte_offset..char_byte_end,
                    family: resolved_family,
                    weight: resolved_weight,
                });
            }

            byte_offset = char_byte_end;
        }
    }

    font_runs
}
```

문자별로 `font_mappings` 테이블을 조회하여 대체 폰트를 결정한다. 매핑이 없는 문자는 원래 폰트를 유지하되, `FontRegistry.nearest_weight()`로 실제 사용 가능한 weight를 선택한다.

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p editor-view font_run`
Expected: 4개 테스트 모두 PASS.

---

## Task 5: build_parley_layout

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph/layout_builder.rs`

- [ ] **Step 1: 구현 작성**

```rust
// crates/editor-view/src/engine/measure_nodes/paragraph/layout_builder.rs
use editor_model::TextAlign;
use parley::{
    Alignment, AlignmentOptions, FontFamily, FontWeight, IndentOptions,
    Layout, LineHeight, StyleProperty, TextStyle,
};

use crate::shaping::{ShapingContext, TextBrush};

use super::font_run::FontRun;
use super::text_run::TextRun;

pub fn build_parley_layout(
    text: &str,
    runs: &[TextRun],
    font_runs: &[FontRun],
    align: TextAlign,
    indent: f32,
    width: f32,
    shaping: &mut ShapingContext,
) -> Layout<TextBrush> {
    let mut builder = shaping.lcx.style_run_builder(&mut shaping.fcx, text, 1.0, true);

    // TextRun과 FontRun을 병합하여 style run을 생성한다.
    // FontRun이 TextRun을 분할할 수 있으므로, 두 iterator를 교차 순회한다.
    let mut run_idx = 0;
    for font_run in font_runs {
        let fr_start = font_run.byte_range.start;
        let fr_end = font_run.byte_range.end;

        // 이 FontRun에 걸치는 TextRun들을 찾는다
        while run_idx < runs.len() && runs[run_idx].byte_range.end <= fr_start {
            run_idx += 1;
        }

        // FontRun 범위 내에서 TextRun을 기반으로 style을 생성
        let mut pos = fr_start;
        let mut current_run_idx = run_idx;

        while pos < fr_end && current_run_idx < runs.len() {
            let tr = &runs[current_run_idx];
            let seg_start = pos.max(tr.byte_range.start);
            let seg_end = fr_end.min(tr.byte_range.end);

            if seg_start >= seg_end {
                current_run_idx += 1;
                continue;
            }

            let style = TextStyle {
                font_size: tr.style.font_size,
                font_weight: FontWeight::new(tr.style.font_weight as f32),
                letter_spacing: tr.style.letter_spacing,
                line_height: LineHeight::FontSizeRelative(tr.style.line_height),
                brush: TextBrush { node_id: tr.node_id },
                font_family: FontFamily::Named(std::borrow::Cow::Owned(
                    font_run.family.clone(),
                )),
                ..Default::default()
            };

            let idx = builder.push_style(style);
            builder.push_style_run(idx, seg_start..seg_end);

            pos = seg_end;
            if pos >= tr.byte_range.end {
                current_run_idx += 1;
            }
        }
    }

    let mut layout = builder.build(text);

    if indent > 0.0 {
        layout.indent(indent, IndentOptions::default());
    }

    layout.break_all_lines(Some(width));

    let alignment = match align {
        TextAlign::Left => Alignment::Start,
        TextAlign::Center => Alignment::Center,
        TextAlign::Right => Alignment::End,
        TextAlign::Justify => Alignment::Justify,
    };
    layout.align(Some(width), alignment, AlignmentOptions::default());

    layout
}
```

TextStyle의 정확한 필드명과 FontFamily variant는 parley 0.7 소스와 맞춰야 한다. 컴파일 시 불일치가 있으면 parley의 `TextStyle` 정의를 확인하여 수정한다.

- [ ] **Step 2: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공. 이 모듈은 parley API에 의존하므로 단위 테스트보다는 Task 7의 통합 테스트에서 검증한다.

---

## Task 6: StrutMetrics 계산 및 extract_measured_lines

**Files:**
- Modify: `crates/editor-view/src/shaping.rs` — measure_strut 추가
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph/line_extraction.rs`

- [ ] **Step 1: measure_strut 구현**

`crates/editor-view/src/shaping.rs`에 추가:

```rust
use crate::engine::resolve::ResolvedTextStyle;
use parley::{FontFamily, FontWeight, Layout, LineHeight, TextStyle};

impl ShapingContext {
    pub fn measure_strut(&mut self, style: &ResolvedTextStyle) -> StrutMetrics {
        let text = " ";
        let mut builder = self.lcx.style_run_builder(&mut self.fcx, text, 1.0, true);

        let ts = TextStyle {
            font_size: style.font_size,
            font_weight: FontWeight::new(style.font_weight as f32),
            font_family: FontFamily::Named(std::borrow::Cow::Owned(
                style.font_family.clone(),
            )),
            line_height: LineHeight::Absolute(style.font_size),
            ..Default::default()
        };

        let idx = builder.push_style(ts);
        builder.push_style_run(idx, 0..text.len());

        let layout: Layout<TextBrush> = builder.build(text);

        // 첫 번째 line의 첫 번째 run에서 font metrics 추출
        let line = layout.lines().next().expect("strut layout should have one line");
        let run = line.runs().next().expect("strut layout should have one run");
        let metrics = run.metrics();

        StrutMetrics {
            ascent: metrics.ascent,
            descent: metrics.descent,
        }
    }
}
```

descent의 부호를 확인해야 한다. parley RunMetrics.descent가 음수면 `.abs()`를 적용한다. 컴파일 후 간단한 테스트로 부호를 확인한다.

- [ ] **Step 2: extract_measured_lines 구현**

```rust
// crates/editor-view/src/engine/measure_nodes/paragraph/line_extraction.rs
use editor_model::NodeId;
use parley::{Layout, PositionedLayoutItem};

use crate::fragment::line::LineSegment;
use crate::measure::MeasuredLine;
use crate::shaping::{StrutMetrics, TextBrush};

pub fn extract_measured_lines(
    text: &str,
    layout: &Layout<TextBrush>,
    strut: &StrutMetrics,
    line_height_ratio: f32,
    base_font_size: f32,
) -> Vec<MeasuredLine> {
    let mut lines = Vec::new();

    for line in layout.lines() {
        let metrics = line.metrics();

        // Strut minimum 적용
        let ascent = metrics.ascent.max(strut.ascent);
        let descent = metrics.descent.max(strut.descent);
        let content_height = ascent + descent;

        // Line box height = max(font_size × ratio, content_height)
        let line_box_height = (base_font_size * line_height_ratio).max(content_height);
        let leading = (line_box_height - content_height).max(0.0);
        let baseline = leading / 2.0 + ascent;

        // Line segments 추출
        let mut segments = Vec::new();

        for item in line.items() {
            let glyph_run = match item {
                PositionedLayoutItem::GlyphRun(gr) => gr,
                PositionedLayoutItem::InlineBox(_) => continue,
            };

            let style = glyph_run.style();
            let node_id = style.brush.node_id;
            let run_offset = glyph_run.offset();

            // Run 내 clusters를 순회하여 segments 생성
            let run = glyph_run.run();
            let mut seg_start_byte: Option<usize> = None;
            let mut seg_x = run_offset;
            let mut seg_char_advances: Vec<f32> = Vec::new();
            let mut seg_text = String::new();
            let mut seg_byte_start = 0usize;

            for cluster in run.clusters() {
                let cluster_range = cluster.text_range();

                if seg_start_byte.is_none() {
                    seg_start_byte = Some(cluster_range.start);
                    seg_byte_start = cluster_range.start;
                    seg_x = run_offset + cluster_x_offset(run, cluster_range.start);
                }

                let cluster_text = &text[cluster_range.clone()];
                seg_text.push_str(cluster_text);

                let advance = cluster.advance();
                // cluster가 여러 문자를 포함할 수 있음 (리거처 등)
                let char_count = cluster_text.chars().count();
                if char_count > 0 {
                    let per_char = advance / char_count as f32;
                    for _ in 0..char_count {
                        seg_char_advances.push(per_char);
                    }
                }
            }

            if !seg_text.is_empty() {
                let byte_start = seg_byte_start;
                let char_offset = text[..byte_start].chars().count();

                segments.push(LineSegment {
                    node_id,
                    offset: char_offset,
                    text: seg_text,
                    x: seg_x,
                    width: seg_char_advances.iter().sum(),
                    char_advances: seg_char_advances,
                });
            }
        }

        lines.push(MeasuredLine {
            height: line_box_height,
            baseline,
            segments,
        });
    }

    lines
}

fn cluster_x_offset(run: &parley::Run<'_, TextBrush>, target_byte: usize) -> f32 {
    let mut x = 0.0;
    for cluster in run.clusters() {
        if cluster.text_range().start >= target_byte {
            break;
        }
        x += cluster.advance();
    }
    x
}
```

이 구현은 각 GlyphRun을 하나의 LineSegment로 매핑한다. 같은 node_id의 연속 GlyphRun은 하나의 LineSegment로 병합할 수 있으나, 초기 구현에서는 단순하게 유지한다.

- [ ] **Step 3: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공. 이 모듈은 Task 7의 통합 테스트에서 검증한다.

---

## Task 7: measure_paragraph 조립 및 LayoutEngine 통합

**Files:**
- Create: `crates/editor-view/src/engine/measure_nodes/paragraph.rs` (기존 paragraph/mod.rs를 이 파일로 이동하거나, 이 파일이 paragraph/ 디렉토리의 mod.rs 역할)
- Modify: `crates/editor-view/src/engine/measure_nodes/mod.rs`
- Modify: `crates/editor-view/src/engine/mod.rs`

- [ ] **Step 1: measure_paragraph 구현**

`paragraph/mod.rs`에 measure_paragraph 함수 추가:

```rust
mod text_run;
mod font_run;
mod layout_builder;
mod line_extraction;

pub use text_run::*;
pub use font_run::*;
pub use layout_builder::*;
pub use line_extraction::*;

use editor_common::{Alignment, Size};
use editor_model::{Doc, Node, NodeRef, TextAlign};

use crate::engine::resolve::{resolve_gap_after, resolve_paragraph_indent, resolve_text_style};
use crate::engine::LayoutEngine;
use crate::measure::{MeasuredContent, Measurement};

pub fn measure_paragraph(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
) -> Measurement {
    let (text, runs) = collect_text_runs(doc, node);

    // 빈 paragraph
    if text.is_empty() {
        return empty_paragraph_measurement(engine, node, width);
    }

    // Paragraph의 기본 텍스트 스타일 (strut + line_height_ratio용)
    let base_style = resolve_text_style(node);
    let indent = resolve_paragraph_indent(node);
    let align = match node.node() {
        Node::Paragraph(p) => p.align,
        _ => TextAlign::Left,
    };

    let mut shaping = engine.shaping.lock().unwrap();

    // Strut 계산
    let strut = shaping.measure_strut(&base_style);

    // Font mapping (문자별 codepoint 매핑 포함)
    let font_runs = resolve_font_mapping(&text, &runs, &shaping.font_registry);

    // Parley layout
    let layout = build_parley_layout(
        &text,
        &runs,
        &font_runs,
        align,
        indent,
        width,
        &mut shaping,
    );

    // Lock 해제 (layout 소유권 이동 후)
    drop(shaping);

    // Line extraction
    let lines = extract_measured_lines(
        &text,
        &layout,
        &strut,
        base_style.line_height,
        base_style.font_size,
    );

    let height: f32 = lines.iter().map(|l| l.height).sum();

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::TextBlock { lines },
    }
}

fn empty_paragraph_measurement(
    engine: &mut LayoutEngine,
    node: &NodeRef<'_>,
    width: f32,
) -> Measurement {
    let base_style = resolve_text_style(node);

    let mut shaping = engine.shaping.lock().unwrap();
    let strut = shaping.measure_strut(&base_style);
    drop(shaping);

    let height = (base_style.font_size * base_style.line_height)
        .max(strut.ascent + strut.descent);

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::TextBlock { lines: vec![] },
    }
}
```

- [ ] **Step 2: measure_nodes/mod.rs에 export 추가**

```rust
pub(crate) mod paragraph;

pub use paragraph::measure_paragraph;
```

- [ ] **Step 3: LayoutEngine에 shaping 필드 추가**

`crates/editor-view/src/engine/mod.rs` 수정:

```rust
use std::sync::{Arc, Mutex};
use crate::shaping::ShapingContext;

#[derive(Debug)]
pub struct LayoutEngine {
    pub(crate) cache: LayoutCache,
    pages: Vec<Page>,
    pub(crate) shaping: Arc<Mutex<ShapingContext>>,
}

impl LayoutEngine {
    pub fn new(shaping: Arc<Mutex<ShapingContext>>) -> Self {
        Self {
            cache: LayoutCache::new(),
            pages: vec![],
            shaping,
        }
    }
}
```

ShapingContext는 Debug를 구현하지 않을 수 있으므로, LayoutEngine의 `#[derive(Debug)]`를 수동 구현으로 변경한다:

```rust
impl std::fmt::Debug for LayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutEngine")
            .field("cache", &self.cache)
            .field("pages", &self.pages)
            .finish()
    }
}
```

- [ ] **Step 4: measure_inner에 Paragraph 분기 추가**

`engine/mod.rs`의 `measure_inner` 함수에서, `_ => measure_nodes::measure_default_container(...)` 앞에 추가:

```rust
Node::Paragraph(_) => measure_nodes::measure_paragraph(self, doc, node, width),
```

- [ ] **Step 5: 기존 테스트 수정**

`engine/mod.rs`의 테스트에서 `LayoutEngine::new()` 호출을 모두 수정:

```rust
use crate::shaping::ShapingContext;

fn test_shaping() -> Arc<Mutex<ShapingContext>> {
    Arc::new(Mutex::new(ShapingContext::new()))
}

// 모든 LayoutEngine::new() → LayoutEngine::new(test_shaping())로 변경
```

다른 파일에서도 `LayoutEngine::new()`를 사용하는 곳을 모두 찾아 동일하게 수정한다.

- [ ] **Step 6: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: 성공.

- [ ] **Step 7: 기존 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 기존 테스트 모두 PASS.

- [ ] **Step 8: 통합 테스트 작성**

`engine/mod.rs`의 `mod tests`에 추가:

```rust
#[test]
fn compute_with_paragraph_text() {
    let (doc,) = doc! {
        root {
            paragraph {
                text("Hello, world!")
            }
        }
    };

    let viewport = Viewport { width: 400.0, scale_factor: 1.0 };
    let vs = ViewState::new();

    let mut engine = LayoutEngine::new(test_shaping());
    engine.compute(&doc, &viewport, &vs);

    assert!(!engine.pages().is_empty());
    let page = &engine.pages()[0];
    assert!(!page.fragments.is_empty());
}

#[test]
fn paragraph_produces_text_block() {
    let (doc, p1) = doc! {
        root {
            p1: paragraph {
                text("Hello")
            }
        }
    };

    let mut engine = LayoutEngine::new(test_shaping());
    let vs = ViewState::new();
    let m = engine.measure(&doc, p1, 400.0, &vs);

    assert!(matches!(m.content, MeasuredContent::TextBlock { .. }));
    if let MeasuredContent::TextBlock { ref lines } = m.content {
        assert!(!lines.is_empty());
        assert!(!lines[0].segments.is_empty());
        assert_eq!(lines[0].segments[0].text, "Hello");
    }
}

#[test]
fn paragraph_multiple_styled_runs() {
    let (doc, p1) = doc! {
        root {
            p1: paragraph {
                text("normal")
                text("bold") [font_size(2400)]
            }
        }
    };

    let mut engine = LayoutEngine::new(test_shaping());
    let vs = ViewState::new();
    let m = engine.measure(&doc, p1, 400.0, &vs);

    if let MeasuredContent::TextBlock { ref lines } = m.content {
        assert!(!lines.is_empty());
        // 두 개의 Text 노드 → segments가 존재
        let total_text: String = lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(total_text, "normalbold");
    }
}

#[test]
fn empty_paragraph_has_height() {
    let (doc, p1) = doc! {
        root {
            p1: paragraph
        }
    };

    let mut engine = LayoutEngine::new(test_shaping());
    let vs = ViewState::new();
    let m = engine.measure(&doc, p1, 400.0, &vs);

    assert!(m.size.height > 0.0, "empty paragraph should have strut-based height");
}
```

- [ ] **Step 9: 통합 테스트 통과 확인**

Run: `cargo test -p editor-view`
Expected: 새 테스트 포함 모든 테스트 PASS.

---

## Task 8: view-remaining-work.md 업데이트

**Files:**
- Modify: `docs/editor-architecture/view-remaining-work.md`

- [ ] **Step 1: Paragraph 항목을 완료로 표시**

`view-remaining-work.md`에서 "Paragraph → TextBlock 변환" 섹션을 완료 섹션으로 이동하고, 설계 문서 참조를 추가한다.
