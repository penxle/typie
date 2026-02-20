#[macro_export]
#[allow(unused)]
macro_rules! id {
    () => {
        $crate::model::NodeId::ROOT
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! transact {
    ($state:expr, |$tr:ident| $body:expr) => {{
        let state: $crate::State = $state;
        #[allow(unused_mut)]
        let mut $tr = $crate::Transaction::new(&state);
        $body;
        let (new_state, _) = $tr.commit().unwrap();
        new_state
    }};
}

#[macro_export]
#[allow(unused)]
macro_rules! transact_with_effect {
    ($state:expr, |$tr:ident| $body:expr) => {{
        let state: crate::runtime::State = $state;
        #[allow(unused_mut)]
        let mut $tr = crate::transaction::Transaction::new(&state);
        $body;
        let (new_state, effects) = $tr.commit().unwrap();
        (new_state, effects)
    }};
}

#[allow(unused)]
macro_rules! try_transact {
    ($state:expr, |$tr:ident| $body:expr) => {{
        let state: $crate::runtime::State = $state;
        let mut $tr = $crate::transaction::Transaction::new(&state);
        let result = $body;
        match result {
            Ok(_) => {
                let (new_state, _) = $tr.commit()?;
                Ok(new_state)
            }
            Err(e) => Err(e),
        }
    }};
}

use crate::global::GLOBALS;
use crate::global::{register_font, set_fallback_fonts};
use crate::icu_data::load_icu_data;

#[allow(unused)]
pub fn init_test_env() {
    use std::cell::Cell;

    static ICU_INIT: std::sync::Once = std::sync::Once::new();
    ICU_INIT.call_once(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let icu_path = std::path::Path::new(&manifest_dir).join("pkg/icu_data.postcard");
        if icu_path.exists() {
            let data = std::fs::read(&icu_path).expect("Failed to read ICU data");
            let _ = load_icu_data(&data);
        } else {
            eprintln!("Warning: ICU data not found at {:?}", icu_path);
        }
    });

    thread_local! {
        static FONT_INIT: Cell<bool> = const { Cell::new(false) };
    }

    FONT_INIT.with(|init| {
        if !init.get() {
            init.set(true);

            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            let assets_dir = std::path::Path::new(&manifest_dir).join("assets");

            GLOBALS.with(|globals| {
                let globals = globals.borrow();
                let mut fcx = globals.parley_font_context.borrow_mut();

                let emoji_path = assets_dir.join("Noto-Phantom-Emoji.ttf");
                if emoji_path.exists() {
                    let data = std::fs::read(&emoji_path).expect("Failed to read emoji font");
                    register_font(&mut fcx, "Noto Emoji", 400, data);
                }

                let font_path = assets_dir.join("Noto-Phantom.ttf");
                if font_path.exists() {
                    let data = std::fs::read(&font_path).expect("Failed to read test font");
                    register_font(&mut fcx, "Noto Sans", 400, data);
                }
            });

            set_fallback_fonts(&["Noto Sans", "Noto Emoji"]);
        }
    });
}

#[allow(unused)]
pub fn click_fold_toggle(runtime: &mut crate::runtime::Runtime, fold_id: crate::model::NodeId) {
    runtime.layout();

    let mut hit = None;
    for (page_idx, page) in runtime.pages().iter().enumerate() {
        let width = page.root.node.size.width.ceil().max(0.0) as usize;
        let height = page.root.node.size.height.ceil().max(0.0) as usize;

        'search: for y in (0..=height).step_by(4) {
            for x in (0..=width).step_by(4) {
                let x = x as f32 + 0.5;
                let y = y as f32 + 0.5;
                if matches!(
                    page.find_interactive_at(x, y, runtime.is_read_only()),
                    Some(crate::layout::interactive::InteractionKind::Toggle { node_id }) if node_id == fold_id
                ) {
                    hit = Some((page_idx, x, y));
                    break 'search;
                }
            }
        }

        if hit.is_some() {
            break;
        }
    }

    let (page_idx, x, y) = hit.expect("fold toggle target should be hittable");
    runtime.update(crate::runtime::Message::PointerDown {
        page_idx,
        x,
        y,
        click_count: 1,
        button: crate::runtime::PointerButton::Primary,
        modifier: crate::runtime::Modifier::default(),
    });
    runtime.update(crate::runtime::Message::PointerUp {
        page_idx,
        x,
        y,
        button: crate::runtime::PointerButton::Primary,
        modifier: crate::runtime::Modifier::default(),
    });
}

#[macro_export]
#[allow(unused)]
macro_rules! doc {
    ($($items:tt)*) => {
        {
            $crate::test_utils::init_test_env();
            let doc = std::rc::Rc::new($crate::model::Doc::new());
            let state = $crate::runtime::State::new(
                doc,
                $crate::state::Selection::collapsed(
                    $crate::state::Position::new($crate::model::NodeId::ROOT, 0, $crate::types::Affinity::default())
                )
            );

            let tr = $crate::transaction::Transaction::new(&state);
            let mut _prev: Option<$crate::model::NodeId> = None;

            __doc_items!(tr, $crate::model::NodeId::ROOT, _prev; $($items)*);

            let (new_state, _) = tr.commit().unwrap();
            new_state.doc
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! state {
    (
        doc { $($items:tt)* }
        selection { $($sel:tt)* }
    ) => {
        {
            $crate::test_utils::init_test_env();
            let doc = std::rc::Rc::new($crate::model::Doc::new());
            let state = $crate::runtime::State::new(
                doc,
                $crate::state::Selection::collapsed(
                    $crate::state::Position::new($crate::model::NodeId::ROOT, 0, $crate::types::Affinity::default())
                )
            );

            let mut tr = $crate::transaction::Transaction::new(&state);
            let mut _prev: Option<$crate::model::NodeId> = None;

            __doc_items!(tr, $crate::model::NodeId::ROOT, _prev; $($items)*);

            let selection = __selection!($($sel)*);
            tr.set_selection(selection);

            let (new_state, _) = tr.commit().unwrap();
            new_state
        }
    };

    (
        doc { $($items:tt)* }
    ) => {
        {
            $crate::test_utils::init_test_env();
            let doc = std::rc::Rc::new($crate::model::Doc::new());
            let state = $crate::runtime::State::new(
                doc,
                $crate::state::Selection::collapsed(
                    $crate::state::Position::new($crate::model::NodeId::ROOT, 0, $crate::types::Affinity::default())
                )
            );

            #[allow(unused_mut)]
            let mut tr = $crate::transaction::Transaction::new(&state);
            let mut _prev: Option<$crate::model::NodeId> = None;

            __doc_items!(tr, $crate::model::NodeId::ROOT, _prev; $($items)*);

            let (new_state, _) = tr.commit().unwrap();
            new_state
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! runtime {
    (
        viewport { paginated { width: $width:expr, height: $height:expr, margin: $margin:expr } }
        $($rest:tt)*
    ) => {
        {
            let state = state! { $($rest)* };
            let mut runtime = $crate::runtime::Runtime::new($width as f32, 1.0, state);
            runtime.doc().update_settings(|s| {
                s.layout_mode = $crate::model::LayoutMode::Paginated {
                    page_width: $width as f32,
                    page_height: $height as f32,
                    page_margin_top: $margin as f32,
                    page_margin_bottom: $margin as f32,
                    page_margin_left: $margin as f32,
                    page_margin_right: $margin as f32,
                };
            }).unwrap();
            runtime.layout();
            runtime
        }
    };

    (
        viewport { continuous { width: $width:expr } }
        $($rest:tt)*
    ) => {
        {
            let state = state! { $($rest)* };
            let mut runtime = $crate::runtime::Runtime::new($width as f32, 1.0, state);
            runtime.doc().update_settings(|s| {
                s.layout_mode = $crate::model::LayoutMode::Continuous { max_width: $width as f32 };
            }).unwrap();
            runtime.layout();
            runtime
        }
    };

    (
        viewport { $width:expr, $_height:expr, $scale:expr }
        $($rest:tt)*
    ) => {
        {
            let state = state! { $($rest)* };
            $crate::runtime::Runtime::new($width as f32, $scale, state)
        }
    };

    (
        $($rest:tt)*
    ) => {
        {
            let state = state! { $($rest)* };
            $crate::runtime::Runtime::new(800.0, 1.0, state)
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __parse_styles {
    () => { vec![] };

    (bold() $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::Bold($crate::model::BoldStyle {})];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (italic() $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::Italic($crate::model::ItalicStyle {})];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (font_weight($weight:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::FontWeight($crate::model::FontWeightStyle { weight: $weight })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (font_size($size:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::FontSize($crate::model::FontSizeStyle { size: $size })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (text_color($key:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::TextColor($crate::model::TextColorStyle { color: $key.to_string() })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (bg_color($key:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::BackgroundColor($crate::model::BackgroundColorStyle { color: $key.to_string() })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (font_family($family:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::FontFamily($crate::model::FontFamilyStyle { family: $family.to_string() })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (letter_spacing($spacing:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::LetterSpacing($crate::model::LetterSpacingStyle { spacing: $spacing })];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (strikethrough() $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::Strikethrough($crate::model::StrikethroughStyle {})];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };

    (underline() $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut styles = vec![$crate::model::Style::Underline($crate::model::UnderlineStyle {})];
            $(styles.extend(__parse_styles!($($rest)*));)?
            styles
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __parse_annotations {
    () => { vec![] };

    (link($href:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut anns = vec![$crate::model::Annotation::Link($crate::model::LinkAnnotation { href: $href.to_string() })];
            $(anns.extend(__parse_annotations!($($rest)*));)?
            anns
        }
    };

    (ruby($text:expr) $(, $($rest:tt)*)?) => {
        {
            #[allow(unused_mut)]
            let mut anns = vec![$crate::model::Annotation::Ruby($crate::model::RubyAnnotation { text: $text.to_string() })];
            $(anns.extend(__parse_annotations!($($rest)*));)?
            anns
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __parse_text_segments {
    ($text:ident,) => {};
    ($text:ident) => {};

    ($text:ident, $content:literal => [$($styles:tt)*] $(, $($rest:tt)*)?) => {
        {
            let mut style_ranges: Vec<(std::ops::Range<usize>, Vec<$crate::model::Style>)> = Vec::new();
            let mut annotation_ranges: Vec<(std::ops::Range<usize>, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_collect!($text, style_ranges, annotation_ranges, $content => [$($styles)*] $(, $($rest)*)?);

            for (range, styles) in style_ranges {
                for style in styles {
                    let _ = $text.apply_style(range.clone(), &style);
                }
            }
            for (range, annotation) in annotation_ranges {
                let _ = $text.apply_annotation(range, &annotation);
            }
        }
    };

    ($text:ident, $content:literal $(, $($rest:tt)*)?) => {
        {
            let mut style_ranges: Vec<(std::ops::Range<usize>, Vec<$crate::model::Style>)> = Vec::new();
            let mut annotation_ranges: Vec<(std::ops::Range<usize>, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_collect!($text, style_ranges, annotation_ranges, $content $(, $($rest)*)?);

            for (range, styles) in style_ranges {
                for style in styles {
                    let _ = $text.apply_style(range.clone(), &style);
                }
            }
            for (range, annotation) in annotation_ranges {
                let _ = $text.apply_annotation(range, &annotation);
            }
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __parse_text_segments_collect {
    ($text:ident, $style_ranges:ident, $annotation_ranges:ident,) => {};
    ($text:ident, $style_ranges:ident, $annotation_ranges:ident) => {};

    ($text:ident, $style_ranges:ident, $annotation_ranges:ident, $content:literal => [$($styles:tt)*] @[$($anns:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let styles = __parse_styles!($($styles)*);
            $style_ranges.push((start..end, styles));
            let annotations = __parse_annotations!($($anns)*);
            for annotation in annotations {
                $annotation_ranges.push((start..end, annotation));
            }
        }
        $(__parse_text_segments_collect!($text, $style_ranges, $annotation_ranges, $($rest)*);)?
    };

    ($text:ident, $style_ranges:ident, $annotation_ranges:ident, $content:literal @[$($anns:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let annotations = __parse_annotations!($($anns)*);
            for annotation in annotations {
                $annotation_ranges.push((start..end, annotation));
            }
        }
        $(__parse_text_segments_collect!($text, $style_ranges, $annotation_ranges, $($rest)*);)?
    };

    ($text:ident, $style_ranges:ident, $annotation_ranges:ident, $content:literal => [$($styles:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let styles = __parse_styles!($($styles)*);
            $style_ranges.push((start..end, styles));
        }
        $(__parse_text_segments_collect!($text, $style_ranges, $annotation_ranges, $($rest)*);)?
    };

    ($text:ident, $style_ranges:ident, $annotation_ranges:ident, $content:literal $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
        }
        $(__parse_text_segments_collect!($text, $style_ranges, $annotation_ranges, $($rest)*);)?
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __parse_text_segments_with_pending {
    ($text:ident, $pending_styles:ident, $pending_annotations:ident,) => {};
    ($text:ident, $pending_styles:ident, $pending_annotations:ident) => {};

    ($text:ident, $pending_styles:ident, $pending_annotations:ident, $content:literal => [$($styles:tt)*] @[$($anns:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let styles = __parse_styles!($($styles)*);
            for style in styles {
                $pending_styles.push((start, end, style));
            }
            let annotations = __parse_annotations!($($anns)*);
            for annotation in annotations {
                $pending_annotations.push((start, end, annotation));
            }
        }
        $(__parse_text_segments_with_pending!($text, $pending_styles, $pending_annotations, $($rest)*);)?
    };

    ($text:ident, $pending_styles:ident, $pending_annotations:ident, $content:literal @[$($anns:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let annotations = __parse_annotations!($($anns)*);
            for annotation in annotations {
                $pending_annotations.push((start, end, annotation));
            }
        }
        $(__parse_text_segments_with_pending!($text, $pending_styles, $pending_annotations, $($rest)*);)?
    };

    ($text:ident, $pending_styles:ident, $pending_annotations:ident, $content:literal => [$($styles:tt)*] $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
            let end = $text.char_len();
            let styles = __parse_styles!($($styles)*);
            for style in styles {
                $pending_styles.push((start, end, style));
            }
        }
        $(__parse_text_segments_with_pending!($text, $pending_styles, $pending_annotations, $($rest)*);)?
    };

    ($text:ident, $pending_styles:ident, $pending_annotations:ident, $content:literal $(, $($rest:tt)*)?) => {
        {
            let start = $text.char_len();
            $text.insert(start, $content);
        }
        $(__parse_text_segments_with_pending!($text, $pending_styles, $pending_annotations, $($rest)*);)?
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __doc_items {
    ($tr:ident, $parent:expr, $prev:ident;) => {};

    ($tr:ident, $parent:expr, $prev:ident; @$label:ident $name:ident ( $($attrs:tt)* ) { $($children:tt)* } $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __doc_create_node_with_id!(
            $tr, $label, $parent, $prev, $name,
            [$($attrs)*],
            [$($children)*]
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __doc_items!($tr, $parent, $prev; $($rest)*);
    };

    ($tr:ident, $parent:expr, $prev:ident; @$label:ident $name:ident ( $($attrs:tt)* ) $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __doc_create_node_with_id!(
            $tr, $label, $parent, $prev, $name,
            [$($attrs)*],
            []
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __doc_items!($tr, $parent, $prev; $($rest)*);
    };

    ($tr:ident, $parent:expr, $prev:ident; @$label:ident $name:ident { $($children:tt)* } $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __doc_create_node_with_id!(
            $tr, $label, $parent, $prev, $name,
            [],
            [$($children)*]
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __doc_items!($tr, $parent, $prev; $($rest)*);
    };

    ($tr:ident, $parent:expr, $prev:ident; $name:ident ( $($attrs:tt)* ) { $($children:tt)* } $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __doc_create_node_with_id!(
                $tr, node_id, $parent, $prev, $name,
                [$($attrs)*],
                [$($children)*]
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __doc_items!($tr, $parent, $prev; $($rest)*);
    };

    ($tr:ident, $parent:expr, $prev:ident; $name:ident ( $($attrs:tt)* ) $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __doc_create_node_with_id!(
                $tr, node_id, $parent, $prev, $name,
                [$($attrs)*],
                []
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __doc_items!($tr, $parent, $prev; $($rest)*);
    };

    ($tr:ident, $parent:expr, $prev:ident; $name:ident { $($children:tt)* } $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __doc_create_node_with_id!(
                $tr, node_id, $parent, $prev, $name,
                [],
                [$($children)*]
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __doc_items!($tr, $parent, $prev; $($rest)*);
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __doc_create_node_with_id {
    ($tr:ident, $id:expr, $parent:expr, $prev:expr, paragraph, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Paragraph($crate::model::ParagraphNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, blockquote, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Blockquote($crate::model::BlockquoteNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, fold, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Fold($crate::model::FoldNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, callout, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Callout($crate::model::CalloutNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, fold_title, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::FoldTitle($crate::model::FoldTitleNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, fold_content, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::FoldContent($crate::model::FoldContentNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [], [$first:literal => [$($first_styles:tt)*] $(, $($rest_segments:tt)*)?]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };

            let text = $crate::model::Text::new();
            #[allow(unused_mut)]
            let mut pending_styles: Vec<(usize, usize, $crate::model::Style)> = Vec::new();
            #[allow(unused_mut)]
            let mut pending_annotations: Vec<(usize, usize, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_with_pending!(text, pending_styles, pending_annotations, $first => [$($first_styles)*] $(, $($rest_segments)*)?);

            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text,
                    ..Default::default()
                })
            ).unwrap();

            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);

            if let Some(node) = $tr.doc().node($id) {
                if let $crate::model::Node::Text(text_node) = node.node() {
                    for (start, end, style) in pending_styles {
                        let _ = text_node.text.apply_style(start..end, &style);
                    }
                    for (start, end, annotation) in pending_annotations {
                        let _ = text_node.text.apply_annotation(start..end, &annotation);
                    }
                }
            }
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [], [$first:literal @[$($first_anns:tt)*] $(, $($rest_segments:tt)*)?]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };

            let text = $crate::model::Text::new();
            #[allow(unused_mut)]
            let mut pending_styles: Vec<(usize, usize, $crate::model::Style)> = Vec::new();
            #[allow(unused_mut)]
            let mut pending_annotations: Vec<(usize, usize, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_with_pending!(text, pending_styles, pending_annotations, $first @[$($first_anns)*] $(, $($rest_segments)*)?);

            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text,
                    ..Default::default()
                })
            ).unwrap();

            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);

            if let Some(node) = $tr.doc().node($id) {
                if let $crate::model::Node::Text(text_node) = node.node() {
                    for (start, end, style) in pending_styles {
                        let _ = text_node.text.apply_style(start..end, &style);
                    }
                    for (start, end, annotation) in pending_annotations {
                        let _ = text_node.text.apply_annotation(start..end, &annotation);
                    }
                }
            }
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [], [$first:literal, $($rest_segments:tt)+]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };

            let text = $crate::model::Text::new();
            #[allow(unused_mut)]
            let mut style_ranges: Vec<(std::ops::Range<usize>, Vec<$crate::model::Style>)> = Vec::new();
            #[allow(unused_mut)]
            let mut annotation_ranges: Vec<(std::ops::Range<usize>, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_collect!(text, style_ranges, annotation_ranges, $first, $($rest_segments)+);

            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text,
                    ..Default::default()
                })
            ).unwrap();

            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);

            if let Some(node) = $tr.doc().node($id) {
                if let $crate::model::Node::Text(text_node) = node.node() {
                    for (range, styles) in style_ranges {
                        for style in styles {
                            let _ = text_node.text.apply_style(range.clone(), &style);
                        }
                    }
                    for (range, annotation) in annotation_ranges {
                        let _ = text_node.text.apply_annotation(range, &annotation);
                    }
                }
            }
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [styles: [$($styles:tt)*], $($rest:tt)*], [$text:expr]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };

            let text = $crate::model::Text::from($text.to_string());
            let text_len = text.char_len();

            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text,
                    $($rest)*
                    ..Default::default()
                })
            ).unwrap();

            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);

            let styles = __parse_styles!($($styles)*);
            if !styles.is_empty() {
                if let Some(node) = $tr.doc().node($id) {
                    if let $crate::model::Node::Text(text_node) = node.node() {
                        for style in styles {
                            let _ = text_node.text.apply_style(0..text_len, &style);
                        }
                    }
                }
            }
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [styles: [$($styles:tt)*]], [$text:expr]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };

            let text = $crate::model::Text::from($text.to_string());
            let text_len = text.char_len();

            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text,
                    ..Default::default()
                })
            ).unwrap();

            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);

            let styles = __parse_styles!($($styles)*);
            if !styles.is_empty() {
                if let Some(node) = $tr.doc().node($id) {
                    if let $crate::model::Node::Text(text_node) = node.node() {
                        for style in styles {
                            let _ = text_node.text.apply_style(0..text_len, &style);
                        }
                    }
                }
            }
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, text, [$($attrs:tt)*], [$text:expr]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Text($crate::model::TextNode {
                    text: $crate::model::Text::from($text.to_string()),
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();
            $crate::test_utils::__apply_default_attrs($tr.doc(), $id);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, image, [$($attrs:tt)*], []) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Image($crate::model::ImageNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, hard_break, [$($attrs:tt)*], []) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::HardBreak($crate::model::HardBreakNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();
        }
    };
    ($tr:ident, $id:expr, $parent:expr, $prev:expr, page_break, [$($attrs:tt)*], []) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::PageBreak($crate::model::PageBreakNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, horizontal_rule, [$($attrs:tt)*], []) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::HorizontalRule($crate::model::HorizontalRuleNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, bullet_list, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::BulletList($crate::model::BulletListNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, ordered_list, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::OrderedList($crate::model::OrderedListNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, list_item, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::ListItem($crate::model::ListItemNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, table, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::Table($crate::model::TableNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, table_row, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::TableRow($crate::model::TableRowNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };

    ($tr:ident, $id:expr, $parent:expr, $prev:expr, table_cell, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            let parent_node = $tr.doc().node($parent).unwrap();
            let index = if let Some(prev_id) = $prev {
                let prev_node = $tr.doc().node(prev_id).unwrap();
                prev_node.index().unwrap() + 1
            } else {
                0
            };
            parent_node.as_mut().insert_child_with_id(
                index,
                $id,
                $crate::model::Node::TableCell($crate::model::TableCellNode {
                    $($attrs)*
                    ..Default::default()
                })
            ).unwrap();

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __doc_items!($tr, $id, _child_prev; $($children)*);
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! fragment {
    (open_start: $open_start:expr, open_end: $open_end:expr, $($items:tt)+) => {
        __fragment_impl!($open_start, $open_end; $($items)+)
    };

    (open_end: $open_end:expr, open_start: $open_start:expr, $($items:tt)+) => {
        __fragment_impl!($open_start, $open_end; $($items)+)
    };

    (open_start: $open_start:expr, open_end: $open_end:expr; $($items:tt)+) => {
        __fragment_impl!($open_start, $open_end; $($items)+)
    };

    (open_end: $open_end:expr, open_start: $open_start:expr; $($items:tt)+) => {
        __fragment_impl!($open_start, $open_end; $($items)+)
    };

    (open_start: $open_start:expr, $($items:tt)+) => {
        __fragment_impl!($open_start, 0; $($items)+)
    };

    (open_start: $open_start:expr; $($items:tt)+) => {
        __fragment_impl!($open_start, 0; $($items)+)
    };

    (open_end: $open_end:expr, $($items:tt)+) => {
        __fragment_impl!(0, $open_end; $($items)+)
    };

    (open_end: $open_end:expr; $($items:tt)+) => {
        __fragment_impl!(0, $open_end; $($items)+)
    };

    ($($items:tt)+) => {
        __fragment_impl!(0, 0; $($items)+)
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __fragment_impl {
    ($open_start:expr, $open_end:expr; $($items:tt)+) => {{
        $crate::test_utils::init_test_env();
        let mut nodes: Vec<($crate::model::NodeId, $crate::model::FragmentNode)> = Vec::new();
        let mut _prev: Option<$crate::model::NodeId> = None;

        __fragment_items!(nodes, None, _prev; $($items)+);

        let mut builder = $crate::model::Fragment::builder();
        for node in nodes {
            builder = builder.add(node);
        }

        builder.open_start($open_start).open_end($open_end).build()
    }};
}

#[macro_export]
#[allow(unused)]
macro_rules! __fragment_items {
    ($nodes:ident, $parent:expr, $prev:ident;) => {};

    ($nodes:ident, $parent:expr, $prev:ident; @$label:ident $name:ident ( $($attrs:tt)* ) { $($children:tt)* } $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __fragment_create_node_with_id!(
            $nodes, $label, $parent, $name,
            [$($attrs)*],
            [$($children)*]
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };

    ($nodes:ident, $parent:expr, $prev:ident; @$label:ident $name:ident ( $($attrs:tt)* ) $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __fragment_create_node_with_id!(
            $nodes, $label, $parent, $name,
            [$($attrs)*],
            []
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };

    ($nodes:ident, $parent:expr, $prev:ident; @$label:ident $name:ident { $($children:tt)* } $($rest:tt)*) => {
        #[allow(unused)]
        { $label; }
        $label = $crate::model::NodeId::new();
        __fragment_create_node_with_id!(
            $nodes, $label, $parent, $name,
            [],
            [$($children)*]
        );
        #[allow(unused_assignments)]
        { $prev = Some($label); }
        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };

    ($nodes:ident, $parent:expr, $prev:ident; $name:ident ( $($attrs:tt)* ) { $($children:tt)* } $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __fragment_create_node_with_id!(
                $nodes, node_id, $parent, $name,
                [$($attrs)*],
                [$($children)*]
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };

    ($nodes:ident, $parent:expr, $prev:ident; $name:ident ( $($attrs:tt)* ) $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __fragment_create_node_with_id!(
                $nodes, node_id, $parent, $name,
                [$($attrs)*],
                []
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };

    ($nodes:ident, $parent:expr, $prev:ident; $name:ident { $($children:tt)* } $($rest:tt)*) => {
        {
            let node_id = $crate::model::NodeId::new();
            __fragment_create_node_with_id!(
                $nodes, node_id, $parent, $name,
                [],
                [$($children)*]
            );
            #[allow(unused_assignments)]
            { $prev = Some(node_id); }
        }

        __fragment_items!($nodes, $parent, $prev; $($rest)*);
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __fragment_create_node_with_id {
    ($nodes:ident, $id:expr, $parent:expr, paragraph, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Paragraph($crate::model::ParagraphNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, blockquote, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Blockquote($crate::model::BlockquoteNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, text, [], [$first:literal => [$($first_styles:tt)*] $(, $($rest_segments:tt)*)?]) => {
        {
            let mut text = $crate::model::Text::new();
            let mut pending_styles: Vec<(usize, usize, $crate::model::Style)> = Vec::new();
            let mut pending_annotations: Vec<(usize, usize, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_with_pending!(text, pending_styles, pending_annotations, $first => [$($first_styles)*] $(, $($rest_segments)*)?);

            let __defaults = $crate::model::DefaultAttrs::default().to_styles();
            $crate::test_utils::__apply_default_attrs_to_text(&text, &__defaults);

            for (start, end, style) in pending_styles {
                let _ = text.apply_style(start..end, &style);
            }
            for (start, end, annotation) in pending_annotations {
                let _ = text.apply_annotation(start..end, &annotation);
            }

            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Text($crate::model::TextNode {
                        text,
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, text, [], [$first:literal, $($rest_segments:tt)+]) => {
        {
            let mut text = $crate::model::Text::new();
            let mut style_ranges: Vec<(std::ops::Range<usize>, Vec<$crate::model::Style>)> = Vec::new();
            let mut annotation_ranges: Vec<(std::ops::Range<usize>, $crate::model::Annotation)> = Vec::new();
            __parse_text_segments_collect!(text, style_ranges, annotation_ranges, $first, $($rest_segments)+);

            let __defaults = $crate::model::DefaultAttrs::default().to_styles();
            $crate::test_utils::__apply_default_attrs_to_text(&text, &__defaults);

            for (range, styles) in style_ranges {
                for style in styles {
                    let _ = text.apply_style(range.clone(), &style);
                }
            }
            for (range, annotation) in annotation_ranges {
                let _ = text.apply_annotation(range, &annotation);
            }

            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Text($crate::model::TextNode {
                        text,
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, text, [styles: [$($styles:tt)*], $($rest:tt)*], [$text:expr]) => {
        {
            let text = $crate::model::Text::from($text.to_string());
            let text_len = text.char_len();

            let __defaults = $crate::model::DefaultAttrs::default().to_styles();
            $crate::test_utils::__apply_default_attrs_to_text(&text, &__defaults);

            let styles = __parse_styles!($($styles)*);
            for style in styles {
                let _ = text.apply_style(0..text_len, &style);
            }

            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Text($crate::model::TextNode {
                        text,
                        $($rest)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, text, [styles: [$($styles:tt)*]], [$text:expr]) => {
        {
            let text = $crate::model::Text::from($text.to_string());
            let text_len = text.char_len();

            let __defaults = $crate::model::DefaultAttrs::default().to_styles();
            $crate::test_utils::__apply_default_attrs_to_text(&text, &__defaults);

            let styles = __parse_styles!($($styles)*);
            for style in styles {
                let _ = text.apply_style(0..text_len, &style);
            }

            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Text($crate::model::TextNode {
                        text,
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, text, [$($attrs:tt)*], [$text:expr]) => {
        {
            let text = $crate::model::Text::from($text.to_string());

            let __defaults = $crate::model::DefaultAttrs::default().to_styles();
            $crate::test_utils::__apply_default_attrs_to_text(&text, &__defaults);

            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Text($crate::model::TextNode {
                        text,
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, image, [$($attrs:tt)*], []) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Image($crate::model::ImageNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, hard_break, [$($attrs:tt)*], []) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::HardBreak($crate::model::HardBreakNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, page_break, [$($attrs:tt)*], []) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::PageBreak($crate::model::PageBreakNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, horizontal_rule, [$($attrs:tt)*], []) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::HorizontalRule($crate::model::HorizontalRuleNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, bullet_list, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::BulletList($crate::model::BulletListNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, ordered_list, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::OrderedList($crate::model::OrderedListNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, list_item, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::ListItem($crate::model::ListItemNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, fold, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Fold($crate::model::FoldNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, fold_title, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::FoldTitle($crate::model::FoldTitleNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, fold_content, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::FoldContent($crate::model::FoldContentNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };

    ($nodes:ident, $id:expr, $parent:expr, callout, [$($attrs:tt)*], [$($children:tt)*]) => {
        {
            $nodes.push((
                $id,
                $crate::model::FragmentNode::new(
                    $crate::model::Node::Callout($crate::model::CalloutNode {
                        $($attrs)*
                        ..Default::default()
                    }),
                    $parent,
                ),
            ));

            #[allow(unused_mut)]
            let mut _child_prev: Option<$crate::model::NodeId> = None;
            __fragment_items!($nodes, Some($id), _child_prev; $($children)*);
        }
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __position {
    ($label:expr, $offset:expr) => {
        $crate::state::Position::new($label, $offset, $crate::types::Affinity::default())
    };

    ($label:expr, $offset:expr, $affinity:expr) => {
        $crate::state::Position::new($label, $offset, $affinity)
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __pos_opt_affinity {
    (($label:expr, $offset:expr)) => {
        __position!($label, $offset)
    };

    (($label:expr, $offset:expr, $affinity:expr)) => {
        __position!($label, $offset, $affinity)
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! __selection {
    ($anchor:tt -> $head:tt) => {
        $crate::state::Selection::new(__pos_opt_affinity!($anchor), __pos_opt_affinity!($head))
    };

    ($pos:tt) => {
        $crate::state::Selection::collapsed(__pos_opt_affinity!($pos))
    };
}

fn collect_nodes_dfs(
    doc: &crate::model::Doc,
    node_id: crate::model::NodeId,
    result: &mut Vec<crate::model::Node>,
) {
    if let Some(node_ref) = doc.node(node_id) {
        result.push(node_ref.node().clone());

        for child in node_ref.children() {
            collect_nodes_dfs(doc, child.node_id(), result);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathPosition {
    path: Vec<usize>,
    offset: usize,
    affinity: crate::types::Affinity,
}

fn position_to_path_position(
    doc: &crate::model::Doc,
    position: crate::state::Position,
) -> PathPosition {
    PathPosition {
        path: doc
            .node(position.node_id)
            .map(|n| n.path())
            .unwrap_or_default(),
        offset: position.offset,
        affinity: position.affinity,
    }
}

#[allow(unused)]
pub fn __apply_default_attrs(doc: &crate::model::Doc, node_id: crate::model::NodeId) {
    let defaults = doc.default_attrs().to_styles();
    if let Some(node) = doc.node(node_id) {
        if let crate::model::Node::Text(text_node) = node.node() {
            __apply_default_attrs_to_text(&text_node.text, &defaults);
        }
    }
}

#[allow(unused)]
pub fn __apply_default_attrs_to_text(text: &crate::model::Text, defaults: &[crate::model::Style]) {
    let len = text.char_len();
    if len > 0 {
        for style in defaults {
            let _ = text.apply_style(0..len, style);
        }
    }
}

#[allow(unused)]
pub fn __assert_state_eq_impl(state1: &crate::runtime::State, state2: &crate::runtime::State) {
    let (doc1, sel1) = (&*state1.doc, &state1.selection);
    let (doc2, sel2) = (&*state2.doc, &state2.selection);

    let mut nodes1 = Vec::new();
    let mut nodes2 = Vec::new();

    collect_nodes_dfs(doc1, crate::model::NodeId::ROOT, &mut nodes1);
    collect_nodes_dfs(doc2, crate::model::NodeId::ROOT, &mut nodes2);

    pretty_assertions::assert_eq!(
        nodes1.len(),
        nodes2.len(),
        "Documents have different number of nodes: {} vs {}\n\n[Left]\n{}\n[Right]\n{}",
        nodes1.len(),
        nodes2.len(),
        crate::inspect::inspect_state(doc1, sel1),
        crate::inspect::inspect_state(doc2, sel2),
    );

    for (i, (node1, node2)) in nodes1.iter().zip(nodes2.iter()).enumerate() {
        pretty_assertions::assert_eq!(
            node1,
            node2,
            "Node at index {} differs: {:?} vs {:?}\n\n[Left]\n{}\n[Right]\n{}",
            i,
            node1,
            node2,
            crate::inspect::inspect_state(doc1, sel1),
            crate::inspect::inspect_state(doc2, sel2),
        );
    }

    for (i, (node1, node2)) in nodes1.iter().zip(nodes2.iter()).enumerate() {
        if let (crate::model::Node::Text(t1), crate::model::Node::Text(t2)) = (node1, node2) {
            let seg1 = collect_style_ranges(&t1.text);
            let seg2 = collect_style_ranges(&t2.text);
            pretty_assertions::assert_eq!(
                seg1,
                seg2,
                "Text style ranges at index {} differ\n\n[Left]\n{}\n[Right]\n{}",
                i,
                crate::inspect::inspect_state(doc1, sel1),
                crate::inspect::inspect_state(doc2, sel2),
            );
        }
    }

    let anchor1 = position_to_path_position(doc1, sel1.anchor);
    let anchor2 = position_to_path_position(doc2, sel2.anchor);
    let head1 = position_to_path_position(doc1, sel1.head);
    let head2 = position_to_path_position(doc2, sel2.head);

    pretty_assertions::assert_eq!(
        anchor1,
        anchor2,
        "Selection anchors differ: {:?} vs {:?}\n\n[Left]\n{}\n[Right]\n{}",
        anchor1,
        anchor2,
        crate::inspect::inspect_state(doc1, sel1),
        crate::inspect::inspect_state(doc2, sel2),
    );

    pretty_assertions::assert_eq!(
        head1,
        head2,
        "Selection heads differ: {:?} vs {:?}\n\n[Left]\n{}\n[Right]\n{}",
        head1,
        head2,
        crate::inspect::inspect_state(doc1, sel1),
        crate::inspect::inspect_state(doc2, sel2),
    );
}

fn collect_style_ranges(
    text: &crate::model::Text,
) -> Vec<(std::ops::Range<usize>, Vec<crate::model::Style>)> {
    let mut result = Vec::new();
    let mut offset = 0;

    for segment in text.get_segments() {
        let len = segment.text.chars().count();
        let range = offset..offset + len;

        let mut styles = segment.styles.clone();
        styles.sort_by_key(|s| s.as_type());

        result.push((range, styles));
        offset += len;
    }

    result
}

#[macro_export]
#[allow(unused)]
macro_rules! assert_state_eq {
    ($state1:expr, $state2:expr) => {{ $crate::test_utils::__assert_state_eq_impl(&$state1, &$state2) }};
}

#[allow(unused)]
pub fn __assert_fragment_eq_impl(f1: &crate::model::Fragment, f2: &crate::model::Fragment) {
    pretty_assertions::assert_eq!(f1.open_start(), f2.open_start(), "Open start mismatch");
    pretty_assertions::assert_eq!(f1.open_end(), f2.open_end(), "Open end mismatch");

    let nodes1: Vec<_> = f1.iter().collect();
    let nodes2: Vec<_> = f2.iter().collect();

    pretty_assertions::assert_eq!(nodes1.len(), nodes2.len(), "Node count mismatch");

    let mut id_map = std::collections::HashMap::new();

    for (i, ((id1, node1), (id2, node2))) in nodes1.iter().zip(nodes2.iter()).enumerate() {
        id_map.insert(**id2, **id1);

        pretty_assertions::assert_eq!(
            node1.data(),
            node2.data(),
            "Node data mismatch at index {}. \nLeft: {:?}\nRight: {:?}",
            i,
            node1.data(),
            node2.data()
        );

        let parent1 = node1.parent();
        let parent2 = node2.parent();

        match (parent1, parent2) {
            (None, None) => {}
            (Some(p1), None) => {
                if f1.nodes.contains_key(&p1) {
                    panic!(
                        "Parent mismatch at index {}. Left has internal parent {:?}, Right has None",
                        i, p1
                    );
                }
            }
            (None, Some(p2)) => {
                panic!(
                    "Parent mismatch at index {}. Left has None, Right has parent {:?}",
                    i, p2
                );
            }
            (Some(p1), Some(p2)) => {
                let mapped_p2 = id_map.get(&p2).cloned();
                if let Some(mapped) = mapped_p2 {
                    pretty_assertions::assert_eq!(
                        p1,
                        mapped,
                        "Parent structure mismatch at index {}. Expected parent {:?} (mapped from {:?}), got parent {:?}",
                        i,
                        p1,
                        p2,
                        mapped
                    );
                } else {
                    panic!(
                        "Parent node {:?} for node at index {} not found in previous nodes",
                        p2, i
                    );
                }
            }
            _ => {
                panic!(
                    "Parent mismatch at index {}. Left: {:?}, Right: {:?}",
                    i, parent1, parent2
                );
            }
        }
    }
}

#[macro_export]
#[allow(unused)]
macro_rules! assert_fragment_eq {
    ($f1:expr, $f2:expr) => {{ $crate::test_utils::__assert_fragment_eq_impl(&$f1, &$f2) }};
}

#[allow(unused)]
pub struct ScopedFontRegistration {
    keys: Vec<(String, u16)>,
}

#[allow(unused)]
impl ScopedFontRegistration {
    pub fn new(font_map: std::collections::HashMap<String, Vec<u16>>) -> Self {
        let mut keys = Vec::new();
        GLOBALS.with(|globals| {
            let globals = globals.borrow();
            let mut fonts = globals.fonts.borrow_mut();
            for (family, weights) in &font_map {
                for &weight in weights {
                    let key = (family.clone(), weight);
                    if !fonts.contains_key(&key) {
                        fonts.insert(
                            key.clone(),
                            crate::global::Font {
                                data: std::sync::Arc::new(crate::global::SharedFontData::new(
                                    Vec::new(),
                                )),
                                split_offset: 0,
                            },
                        );
                        keys.push(key);
                    }
                }
            }
            let mut available = globals.available_fonts.borrow_mut();
            for (family, weights) in &font_map {
                available.insert(family.clone(), weights.clone());
            }
        });
        Self { keys }
    }
}

impl Drop for ScopedFontRegistration {
    fn drop(&mut self) {
        GLOBALS.with(|globals| {
            let globals = globals.borrow();
            let mut fonts = globals.fonts.borrow_mut();
            let mut available = globals.available_fonts.borrow_mut();
            for key in &self.keys {
                fonts.remove(key);
                if let Some(weights) = available.get_mut(&key.0) {
                    weights.retain(|&w| w != key.1);
                    if weights.is_empty() {
                        available.remove(&key.0);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic]
    fn assert_state_eq_fails_on_style_ranges() {
        let mut p = id!();

        let state1 = state! {
            doc {
                @p paragraph {
                    text { "abc" }
                }
            }
            selection { (p, 0) }
        };

        let state2 = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "abc" }
                }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(state1, state2);
    }
}
