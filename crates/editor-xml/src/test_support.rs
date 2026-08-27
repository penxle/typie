use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    Alignment, BlockquoteVariant, CalloutVariant, HorizontalRuleVariant, LayoutMode, Modifier,
    ModifierType, NodeType, PlainBlockquoteNode, PlainBulletListNode, PlainCalloutNode, PlainDoc,
    PlainFoldContentNode, PlainFoldNode, PlainFoldTitleNode, PlainHardBreakNode,
    PlainHorizontalRuleNode, PlainImageNode, PlainListItemNode, PlainNode, PlainNodeEntry,
    PlainOrderedListNode, PlainPageBreakNode, PlainParagraphNode, PlainRootNode, PlainTabNode,
    PlainTableCellNode, PlainTableNode, PlainTableRowNode, PlainTextNode, TableBorderStyle,
};
use editor_state::State;
use proptest::prelude::*;

use crate::error::Pos;
use crate::names::is_textblock;
use crate::tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};

pub fn live_heads(state: &State) -> Vec<Dot> {
    let mut heads: Vec<Dot> = state.graph().current_heads().copied().collect();
    heads.sort();
    heads
}

const ALPHABET: &[char] = &[
    '가', '나', '다', 'a', 'b', ' ', ' ', '<', '&', '"', '\'', '>', '한', '😀',
];

/// How deep a generated block may nest containers. A container built at depth
/// `d` builds its own containers at `d - 1`, so the strategy tree the
/// constructor walks is finite.
const BLOCK_DEPTH: u32 = 2;

/// How often a root-direct paragraph ends in a page break.
const PAGE_BREAK_WEIGHT: f64 = 0.1;

fn arb_text() -> BoxedStrategy<String> {
    prop::collection::vec(prop::sample::select(ALPHABET.to_vec()), 1..12)
        .prop_map(|v| v.into_iter().collect())
        .boxed()
}

/// One inline modifier a leaf can own. `on_text` tells the two kinds the
/// schema keeps to `Paragraph > Text` apart from the ten it also allows on
/// `Tab` and `HardBreak`.
fn arb_inline_modifier(on_text: bool) -> BoxedStrategy<Modifier> {
    let shared = prop_oneof![
        Just(Modifier::Bold),
        Just(Modifier::Italic),
        Just(Modifier::Underline),
        Just(Modifier::Strikethrough),
        prop::sample::select(vec![400u32, 1200, 12800])
            .prop_map(|value| Modifier::FontSize { value }),
        prop::sample::select(vec!["Pretendard".to_string(), "Noto Serif KR".to_string()])
            .prop_map(|value| Modifier::FontFamily { value }),
        prop::sample::select(vec![100u16, 400, 900])
            .prop_map(|value| Modifier::FontWeight { value }),
        prop::sample::select(vec!["#ff0000".to_string(), "#00ff00".to_string()])
            .prop_map(|value| Modifier::TextColor { value }),
        prop::sample::select(vec!["#0000ff".to_string(), "#ffff00".to_string()])
            .prop_map(|value| Modifier::BackgroundColor { value }),
        prop::sample::select(vec![-50i32, 0, 200])
            .prop_map(|value| Modifier::LetterSpacing { value }),
    ];
    if !on_text {
        return shared.boxed();
    }
    prop_oneof![
        8 => shared,
        1 => Just(Modifier::Link { href: "https://typie.co/?a=1&b=2".to_string() }),
        1 => Just(Modifier::Ruby { text: "ruby".to_string() }),
    ]
    .boxed()
}

fn arb_own_modifiers(on_text: bool) -> BoxedStrategy<BTreeMap<ModifierType, Modifier>> {
    prop_oneof![
        2 => Just(BTreeMap::new()),
        1 => prop::collection::vec(arb_inline_modifier(on_text), 1..4)
            .prop_map(|ms| ms.into_iter().map(|m| (m.as_type(), m)).collect()),
    ]
    .boxed()
}

fn text_entry(text: String, modifiers: BTreeMap<ModifierType, Modifier>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers,
        carry: Vec::new(),
        children: Vec::new(),
    }
}

fn atom_entry(node: PlainNode) -> PlainNodeEntry {
    atom_entry_with(node, BTreeMap::new())
}

fn atom_entry_with(node: PlainNode, modifiers: BTreeMap<ModifierType, Modifier>) -> PlainNodeEntry {
    PlainNodeEntry {
        node,
        modifiers,
        carry: Vec::new(),
        children: Vec::new(),
    }
}

/// A paragraph. `root_direct` carries the two kinds the schema allows only
/// under the root: the `paragraph_indent` modifier and a trailing page break.
fn arb_paragraph(root_direct: bool) -> BoxedStrategy<PlainNodeEntry> {
    let inline = prop_oneof![
        6 => (arb_text(), arb_own_modifiers(true)).prop_map(|(t, m)| text_entry(t, m)),
        1 => arb_own_modifiers(false)
            .prop_map(|m| atom_entry_with(PlainNode::HardBreak(PlainHardBreakNode {}), m)),
        1 => arb_own_modifiers(false)
            .prop_map(|m| atom_entry_with(PlainNode::Tab(PlainTabNode {}), m)),
    ];
    let indent: BoxedStrategy<Option<u32>> = if root_direct {
        prop::option::weighted(0.2, prop::sample::select(vec![0u32, 200, 400])).boxed()
    } else {
        Just(None).boxed()
    };
    let page_break: BoxedStrategy<bool> = if root_direct && PAGE_BREAK_WEIGHT > 0.0 {
        prop::bool::weighted(PAGE_BREAK_WEIGHT).boxed()
    } else {
        Just(false).boxed()
    };
    (
        prop::collection::vec(inline, 0..6),
        prop::option::of(prop::sample::select(vec![
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
            Alignment::Justify,
        ])),
        prop::option::of(prop::sample::select(vec![120u32, 160, 200])),
        prop::bool::weighted(0.15),
        indent,
        page_break,
    )
        .prop_map(
            |(mut children, align, lh, carry_bold, indent, page_break)| {
                let mut modifiers = BTreeMap::new();
                if let Some(a) = align {
                    modifiers.insert(ModifierType::Alignment, Modifier::Alignment { value: a });
                }
                if let Some(v) = lh {
                    modifiers.insert(ModifierType::LineHeight, Modifier::LineHeight { value: v });
                }
                if let Some(v) = indent {
                    modifiers.insert(
                        ModifierType::ParagraphIndent,
                        Modifier::ParagraphIndent { value: v },
                    );
                }
                if page_break {
                    children.push(atom_entry(PlainNode::PageBreak(PlainPageBreakNode {})));
                }
                PlainNodeEntry {
                    node: PlainNode::Paragraph(PlainParagraphNode {}),
                    modifiers,
                    carry: if carry_bold {
                        vec![Modifier::Bold]
                    } else {
                        Vec::new()
                    },
                    children,
                }
            },
        )
        .boxed()
}

/// A fold title. Its content rule is `Text*`, so it is one of the three
/// elements that can stand empty, and no inline modifier reaches it — the
/// schema puts every one of them under `Paragraph`.
fn arb_fold_title() -> BoxedStrategy<PlainNodeEntry> {
    (prop::option::of(arb_text()), prop::bool::weighted(0.15))
        .prop_map(|(text, carry_bold)| PlainNodeEntry {
            node: PlainNode::FoldTitle(PlainFoldTitleNode {}),
            modifiers: BTreeMap::new(),
            carry: if carry_bold {
                vec![Modifier::Bold]
            } else {
                Vec::new()
            },
            children: text
                .into_iter()
                .map(|t| text_entry(t, BTreeMap::new()))
                .collect(),
        })
        .boxed()
}

fn container(node: PlainNode, children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node,
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn arb_list(depth: u32) -> BoxedStrategy<PlainNodeEntry> {
    let item: BoxedStrategy<PlainNodeEntry> = if depth == 0 {
        arb_paragraph(false)
            .prop_map(|p| container(PlainNode::ListItem(PlainListItemNode {}), vec![p]))
            .boxed()
    } else {
        (
            arb_paragraph(false),
            prop::option::weighted(0.3, arb_list(depth - 1)),
        )
            .prop_map(|(p, sub)| {
                let mut children = vec![p];
                children.extend(sub);
                container(PlainNode::ListItem(PlainListItemNode {}), children)
            })
            .boxed()
    };
    (prop::bool::ANY, prop::collection::vec(item, 1..4))
        .prop_map(|(ordered, items)| {
            if ordered {
                container(PlainNode::OrderedList(PlainOrderedListNode {}), items)
            } else {
                container(PlainNode::BulletList(PlainBulletListNode {}), items)
            }
        })
        .boxed()
}

/// What a blockquote or a callout may hold: `(Paragraph | BulletList |
/// OrderedList)+`.
fn arb_quote_children(depth: u32) -> BoxedStrategy<Vec<PlainNodeEntry>> {
    let child: BoxedStrategy<PlainNodeEntry> = if depth == 0 {
        arb_paragraph(false)
    } else {
        prop_oneof![3 => arb_paragraph(false), 1 => arb_list(depth - 1)].boxed()
    };
    prop::collection::vec(child, 1..3).boxed()
}

fn arb_fold(depth: u32) -> BoxedStrategy<PlainNodeEntry> {
    (
        arb_fold_title(),
        prop::collection::vec(arb_block_at(depth - 1, false), 1..3),
    )
        .prop_map(|(title, blocks)| PlainNodeEntry {
            node: PlainNode::Fold(PlainFoldNode {}),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![
                title,
                container(PlainNode::FoldContent(PlainFoldContentNode {}), blocks),
            ],
        })
        .boxed()
}

/// A rectangular table. Its cells hold paragraphs only, which is also what
/// keeps the `!Table > ** > &` context rule satisfied without a second guard.
fn arb_table() -> BoxedStrategy<PlainNodeEntry> {
    let cell = (
        prop::option::weighted(0.4, prop::sample::select(vec![80u32, 160])),
        prop::option::weighted(0.2, Just("#eeeeee".to_string())),
        prop::collection::vec(arb_paragraph(false), 1..3),
    )
        .prop_map(|(col_width, background_color, children)| PlainNodeEntry {
            node: PlainNode::TableCell(PlainTableCellNode {
                col_width,
                background_color,
            }),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children,
        })
        .boxed();
    (
        prop::sample::select(vec![
            TableBorderStyle::Solid,
            TableBorderStyle::Dashed,
            TableBorderStyle::None,
        ]),
        prop::sample::select(vec![50u32, 100]),
        prop::option::weighted(
            0.3,
            prop::sample::select(vec![Alignment::Center, Alignment::Right]),
        ),
        (1..4usize, 1..4usize).prop_flat_map(move |(rows, cols)| {
            prop::collection::vec(
                prop::collection::vec(cell.clone(), cols..=cols),
                rows..=rows,
            )
        }),
    )
        .prop_map(|(border_style, proportion, align, grid)| {
            let mut modifiers = BTreeMap::new();
            if let Some(a) = align {
                modifiers.insert(ModifierType::Alignment, Modifier::Alignment { value: a });
            }
            PlainNodeEntry {
                node: PlainNode::Table(PlainTableNode {
                    border_style,
                    proportion,
                }),
                modifiers,
                carry: Vec::new(),
                children: grid
                    .into_iter()
                    .map(|row| container(PlainNode::TableRow(PlainTableRowNode {}), row))
                    .collect(),
            }
        })
        .boxed()
}

fn arb_block_at(depth: u32, root_direct: bool) -> BoxedStrategy<PlainNodeEntry> {
    let rule = prop::sample::select(vec![
        HorizontalRuleVariant::Line,
        HorizontalRuleVariant::Zigzag,
    ])
    .prop_map(|variant| {
        atom_entry(PlainNode::HorizontalRule(PlainHorizontalRuleNode {
            variant,
        }))
    });
    if depth == 0 {
        return prop_oneof![
            6 => arb_paragraph(root_direct),
            1 => rule,
            1 => arb_image(),
        ]
        .boxed();
    }
    prop_oneof![
        8 => arb_paragraph(root_direct),
        1 => rule,
        1 => arb_image(),
        1 => (prop::sample::select(vec![BlockquoteVariant::LeftLine, BlockquoteVariant::LeftQuote]), arb_quote_children(depth))
            .prop_map(|(variant, cs)| container(PlainNode::Blockquote(PlainBlockquoteNode { variant }), cs)),
        1 => (prop::sample::select(vec![CalloutVariant::Info, CalloutVariant::Warning]), arb_quote_children(depth))
            .prop_map(|(variant, cs)| container(PlainNode::Callout(PlainCalloutNode { variant }), cs)),
        1 => arb_list(depth - 1),
        2 => arb_fold(depth),
        2 => arb_table(),
    ]
    .boxed()
}

fn arb_image() -> BoxedStrategy<PlainNodeEntry> {
    (
        prop::sample::select(vec!["IMG1".to_string(), "IMG2".to_string()]),
        prop::sample::select(vec![50u32, 100]),
        prop::option::weighted(
            0.3,
            prop::sample::select(vec![Alignment::Center, Alignment::Right]),
        ),
    )
        .prop_map(|(id, proportion, align)| {
            let mut modifiers = BTreeMap::new();
            if let Some(a) = align {
                modifiers.insert(ModifierType::Alignment, Modifier::Alignment { value: a });
            }
            atom_entry_with(
                PlainNode::Image(PlainImageNode {
                    id: Some(id),
                    proportion,
                }),
                modifiers,
            )
        })
        .boxed()
}

fn arb_layout_mode() -> BoxedStrategy<LayoutMode> {
    prop_oneof![
        9 => prop::sample::select(vec![400u32, 600, 900])
            .prop_map(|max_width| LayoutMode::Continuous { max_width }),
        1 => prop::sample::select(vec![40u32, 80]).prop_map(|margin| LayoutMode::Paginated {
            page_width: 794,
            page_height: 1123,
            page_margin_top: margin,
            page_margin_bottom: margin,
            page_margin_left: margin,
            page_margin_right: margin,
        }),
    ]
    .boxed()
}

/// The block modifiers the schema puts in the root's context.
fn arb_root_modifiers() -> BoxedStrategy<BTreeMap<ModifierType, Modifier>> {
    (
        prop::option::weighted(0.2, prop::sample::select(vec![0u32, 200, 400])),
        prop::option::weighted(0.2, prop::sample::select(vec![0u32, 100, 400])),
        prop::option::weighted(
            0.2,
            prop::sample::select(vec!["Pretendard".to_string(), "Noto Serif KR".to_string()]),
        ),
        prop::option::weighted(0.2, prop::sample::select(vec![50u32, 400])),
    )
        .prop_map(|(gap, indent, family, line_height)| {
            let mut m = BTreeMap::new();
            if let Some(value) = gap {
                m.insert(ModifierType::BlockGap, Modifier::BlockGap { value });
            }
            if let Some(value) = indent {
                m.insert(
                    ModifierType::ParagraphIndent,
                    Modifier::ParagraphIndent { value },
                );
            }
            if let Some(value) = family {
                m.insert(ModifierType::FontFamily, Modifier::FontFamily { value });
            }
            if let Some(value) = line_height {
                m.insert(ModifierType::LineHeight, Modifier::LineHeight { value });
            }
            m
        })
        .boxed()
}

pub fn arb_plain_doc() -> BoxedStrategy<PlainDoc> {
    (
        arb_layout_mode(),
        arb_root_modifiers(),
        prop::collection::vec(arb_block_at(BLOCK_DEPTH, true), 1..6),
    )
        .prop_map(|(layout_mode, modifiers, children)| PlainDoc {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode { layout_mode }),
                modifiers,
                carry: Vec::new(),
                children,
            },
        })
        .boxed()
}

#[derive(Debug, Clone)]
pub enum Mutation {
    ReplaceChar {
        paragraph: usize,
        at: usize,
        with: char,
    },
    InsertText {
        paragraph: usize,
        at: usize,
        text: String,
    },
    DeleteRange {
        paragraph: usize,
        from: usize,
        len: usize,
    },
    ToggleModifier {
        paragraph: usize,
        from: usize,
        len: usize,
        modifier: Modifier,
    },
    SplitParagraph {
        paragraph: usize,
        at: usize,
    },
    MergeWithNext {
        paragraph: usize,
    },
    InsertParagraphAfter {
        block: usize,
        text: String,
    },
    DeleteBlock {
        block: usize,
    },
    MoveBlockToFront {
        block: usize,
    },
    MoveBlockInto {
        block: usize,
        container: usize,
    },
    UnwrapBlock {
        block: usize,
    },
    WrapInBlockquote {
        block: usize,
    },
    WrapWithLeadParagraph {
        block: usize,
        text: String,
        atom: bool,
    },
    ChangeBlockType {
        block: usize,
    },
    AddTableRow {
        table: usize,
    },
    DeleteTableRow {
        table: usize,
        row: usize,
    },
    AddListItem {
        list: usize,
        text: String,
    },
    DeleteListItem {
        list: usize,
        item: usize,
    },
    SetAlignment {
        paragraph: usize,
        value: Alignment,
    },
    ReplaceAll {
        paragraphs: Vec<String>,
    },
}

fn arb_replace_char() -> impl Strategy<Value = Mutation> {
    (
        0..8usize,
        0..12usize,
        prop::sample::select(ALPHABET.to_vec()),
    )
        .prop_map(|(p, at, with)| Mutation::ReplaceChar {
            paragraph: p,
            at,
            with,
        })
}

pub fn arb_mutation() -> BoxedStrategy<Mutation> {
    prop_oneof![
        arb_replace_char(),
        (0..8usize, 0..12usize, arb_text()).prop_map(|(p, at, text)| Mutation::InsertText {
            paragraph: p,
            at,
            text
        }),
        (0..8usize, 0..12usize, 1..4usize).prop_map(|(p, from, len)| Mutation::DeleteRange {
            paragraph: p,
            from,
            len
        }),
        (0..8usize, 0..12usize, 1..4usize, arb_inline_modifier(true)).prop_map(
            |(p, from, len, modifier)| Mutation::ToggleModifier {
                paragraph: p,
                from,
                len,
                modifier
            }
        ),
        (0..8usize, 0..12usize).prop_map(|(p, at)| Mutation::SplitParagraph { paragraph: p, at }),
        (0..8usize).prop_map(|p| Mutation::MergeWithNext { paragraph: p }),
        (0..8usize, arb_text())
            .prop_map(|(b, text)| Mutation::InsertParagraphAfter { block: b, text }),
        (0..8usize).prop_map(|b| Mutation::DeleteBlock { block: b }),
        (0..8usize).prop_map(|b| Mutation::MoveBlockToFront { block: b }),
        (0..6usize, 0..3usize).prop_map(|(b, c)| Mutation::MoveBlockInto {
            block: b,
            container: c
        }),
        (0..8usize).prop_map(|b| Mutation::UnwrapBlock { block: b }),
        (0..6usize).prop_map(|b| Mutation::WrapInBlockquote { block: b }),
        (0..6usize, arb_text(), any::<bool>()).prop_map(|(b, text, atom)| {
            Mutation::WrapWithLeadParagraph {
                block: b,
                text,
                atom,
            }
        }),
        (0..4usize).prop_map(|b| Mutation::ChangeBlockType { block: b }),
        (0..2usize).prop_map(|t| Mutation::AddTableRow { table: t }),
        (0..2usize, 0..3usize).prop_map(|(t, row)| Mutation::DeleteTableRow { table: t, row }),
        (0..2usize, arb_text()).prop_map(|(l, text)| Mutation::AddListItem { list: l, text }),
        (0..2usize, 0..3usize).prop_map(|(l, item)| Mutation::DeleteListItem { list: l, item }),
        (
            0..8usize,
            prop::sample::select(vec![Alignment::Center, Alignment::Right])
        )
            .prop_map(|(p, value)| Mutation::SetAlignment {
                paragraph: p,
                value
            }),
        prop::collection::vec(arb_text(), 1..4)
            .prop_map(|paragraphs| Mutation::ReplaceAll { paragraphs }),
    ]
    .boxed()
}

/// Applies `m` to a parsed tree, or reports `None` when the mutation has no
/// valid landing on this document — an out-of-range index, or a shape the
/// reader would refuse and the diff could never reach.
pub fn apply_mutation(tree: &XmlTree, m: &Mutation) -> Option<XmlTree> {
    let mut out = tree.clone();
    let replaces_root = matches!(m, Mutation::ReplaceAll { .. });
    match m {
        Mutation::ReplaceChar {
            paragraph,
            at,
            with,
        } => {
            let block = authored_textblock_at(&mut out.root, *paragraph)?;
            let entry = inline_at(block, *at)?;
            match entry.leaf {
                InlineLeaf::Char(_) => entry.leaf = InlineLeaf::Char(*with),
                InlineLeaf::Atom(_) => return None,
            }
        }
        Mutation::InsertText {
            paragraph,
            at,
            text,
        } => {
            let block = authored_textblock_at(&mut out.root, *paragraph)?;
            if *at > block.children.len() {
                return None;
            }
            let pos = block.pos;
            let inserted: Vec<XmlChild> = text
                .chars()
                .map(|ch| {
                    XmlChild::Inline(InlineEntry {
                        pos,
                        leaf: InlineLeaf::Char(ch),
                        own: BTreeMap::new(),
                    })
                })
                .collect();
            block.children.splice(*at..*at, inserted);
        }
        Mutation::DeleteRange {
            paragraph,
            from,
            len,
        } => {
            let block = authored_textblock_at(&mut out.root, *paragraph)?;
            let end = from.checked_add(*len)?;
            if end > block.children.len() {
                return None;
            }
            block.children.drain(*from..end);
        }
        Mutation::ToggleModifier {
            paragraph,
            from,
            len,
            modifier,
        } => {
            let block = authored_textblock_at(&mut out.root, *paragraph)?;
            if block.node.as_type() != NodeType::Paragraph {
                return None;
            }
            let end = from.checked_add(*len)?;
            if end > block.children.len() {
                return None;
            }
            let ty = modifier.as_type();
            for child in &mut block.children[*from..end] {
                let XmlChild::Inline(entry) = child else {
                    return None;
                };
                if entry.own.remove(&ty).is_none() {
                    entry.own.insert(ty, modifier.clone());
                }
            }
        }
        Mutation::SplitParagraph { paragraph, at } => {
            let path = authored_paragraph_path(&out.root, *paragraph)?;
            let (index, parent_path) = path.split_last()?;
            let (index, parent_path) = (*index, parent_path.to_vec());
            let block = node_at_path_mut(&mut out.root, &path)?;
            if *at > block.children.len() {
                return None;
            }
            let tail = block.children.split_off(*at);
            let (pos, modifiers) = (block.pos, block.modifiers.clone());
            let parent = node_at_path_mut(&mut out.root, &parent_path)?;
            parent.children.insert(
                index + 1,
                XmlChild::Block(XmlNode {
                    dot: None,
                    pos,
                    node: PlainNode::Paragraph(PlainParagraphNode {}),
                    modifiers,
                    carry: BTreeMap::new(),
                    children: tail,
                }),
            );
        }
        Mutation::MergeWithNext { paragraph } => {
            let path = authored_paragraph_path(&out.root, *paragraph)?;
            let (index, parent_path) = path.split_last()?;
            let (index, parent_path) = (*index, parent_path.to_vec());
            let parent = node_at_path_mut(&mut out.root, &parent_path)?;
            let next = match parent.children.get(index + 1)? {
                XmlChild::Block(block) => block,
                XmlChild::Inline(_) => return None,
            };
            if next.node.as_type() != NodeType::Paragraph
                || next.dot.is_some_and(|d| d.is_synthetic())
            {
                return None;
            }
            let XmlChild::Block(next) = parent.children.remove(index + 1) else {
                return None;
            };
            let XmlChild::Block(first) = &mut parent.children[index] else {
                return None;
            };
            first.children.extend(next.children);
        }
        Mutation::InsertParagraphAfter { block, text } => {
            if *block >= out.root.children.len() {
                return None;
            }
            let child = new_paragraph(out.root.pos, text);
            out.root.children.insert(block + 1, XmlChild::Block(child));
        }
        Mutation::DeleteBlock { block } => {
            let target = root_child(&out.root, *block)?;
            if target.dot.is_some_and(|d| d.is_synthetic()) {
                return None;
            }
            out.root.children.remove(*block);
        }
        Mutation::MoveBlockToFront { block } => {
            if *block == 0 {
                return None;
            }
            let target = root_child(&out.root, *block)?;
            if target.dot.is_some_and(|d| d.is_synthetic()) {
                return None;
            }
            let child = out.root.children.remove(*block);
            out.root.children.insert(0, child);
        }
        Mutation::MoveBlockInto { block, container } => {
            let target = root_child(&out.root, *block)?;
            if target.dot.is_some_and(|d| d.is_synthetic()) {
                return None;
            }
            let path = block_paths(&out.root, &is_open_container)
                .into_iter()
                .filter(|p| p.first() != Some(block))
                .nth(*container)?;
            let moved = out.root.children.remove(*block);
            let mut path = path;
            if path[0] > *block {
                path[0] -= 1;
            }
            node_at_path_mut(&mut out.root, &path)?.children.push(moved);
        }
        Mutation::UnwrapBlock { block } => {
            let target = root_child(&out.root, *block)?;
            if target.dot.is_some_and(|d| d.is_synthetic()) {
                return None;
            }
            let lifted = lifted_children(target)?;
            out.root.children.splice(*block..*block + 1, lifted);
        }
        Mutation::WrapInBlockquote { block } => {
            let path = authored_paragraph_path(&out.root, *block)?;
            let (index, parent_path) = path.split_last()?;
            let (index, parent_path) = (*index, parent_path.to_vec());
            let parent = node_at_path_mut(&mut out.root, &parent_path)?;
            let pos = parent.pos;
            let inner = parent.children.remove(index);
            parent.children.insert(
                index,
                XmlChild::Block(XmlNode {
                    dot: None,
                    pos,
                    node: PlainNode::Blockquote(PlainBlockquoteNode {
                        variant: BlockquoteVariant::LeftLine,
                    }),
                    modifiers: BTreeMap::new(),
                    carry: BTreeMap::new(),
                    children: vec![inner],
                }),
            );
        }
        Mutation::WrapWithLeadParagraph { block, text, atom } => {
            let path = authored_paragraph_path(&out.root, *block)?;
            let (index, parent_path) = path.split_last()?;
            let (index, parent_path) = (*index, parent_path.to_vec());
            let parent = node_at_path_mut(&mut out.root, &parent_path)?;
            let pos = parent.pos;
            let inner = parent.children.remove(index);
            let mut lead = new_paragraph(pos, text);
            if *atom {
                lead.children.push(XmlChild::Inline(InlineEntry {
                    pos,
                    leaf: InlineLeaf::Atom(PlainNode::HardBreak(PlainHardBreakNode {})),
                    own: BTreeMap::new(),
                }));
            }
            parent.children.insert(
                index,
                XmlChild::Block(XmlNode {
                    dot: None,
                    pos,
                    node: PlainNode::Blockquote(PlainBlockquoteNode {
                        variant: BlockquoteVariant::LeftLine,
                    }),
                    modifiers: BTreeMap::new(),
                    carry: BTreeMap::new(),
                    children: vec![XmlChild::Block(lead), inner],
                }),
            );
        }
        Mutation::ChangeBlockType { block } => {
            let path = block_paths(&out.root, &|node| {
                matches!(
                    node.node.as_type(),
                    NodeType::BulletList
                        | NodeType::OrderedList
                        | NodeType::Blockquote
                        | NodeType::Callout
                )
            })
            .into_iter()
            .nth(*block)?;
            let target = node_at_path_mut(&mut out.root, &path)?;
            if target.dot.is_some_and(|d| d.is_synthetic()) {
                return None;
            }
            target.node = match target.node.as_type() {
                NodeType::BulletList => PlainNode::OrderedList(PlainOrderedListNode {}),
                NodeType::OrderedList => PlainNode::BulletList(PlainBulletListNode {}),
                NodeType::Blockquote => PlainNode::Callout(PlainCalloutNode {
                    variant: CalloutVariant::Info,
                }),
                _ => PlainNode::Blockquote(PlainBlockquoteNode {
                    variant: BlockquoteVariant::LeftLine,
                }),
            };
        }
        Mutation::AddTableRow { table } => {
            let path = nth_path(&out.root, NodeType::Table, *table)?;
            let node = node_at_path_mut(&mut out.root, &path)?;
            let cols = node
                .block_children()
                .next()
                .map(|row| row.block_children().count())
                .unwrap_or_default();
            if cols == 0 {
                return None;
            }
            let pos = node.pos;
            let cells: Vec<XmlChild> = (0..cols)
                .map(|_| {
                    XmlChild::Block(XmlNode {
                        dot: None,
                        pos,
                        node: PlainNode::TableCell(PlainTableCellNode {
                            col_width: None,
                            background_color: None,
                        }),
                        modifiers: BTreeMap::new(),
                        carry: BTreeMap::new(),
                        children: vec![XmlChild::Block(new_paragraph(pos, ""))],
                    })
                })
                .collect();
            node.children.push(XmlChild::Block(XmlNode {
                dot: None,
                pos,
                node: PlainNode::TableRow(PlainTableRowNode {}),
                modifiers: BTreeMap::new(),
                carry: BTreeMap::new(),
                children: cells,
            }));
        }
        Mutation::DeleteTableRow { table, row } => {
            let path = nth_path(&out.root, NodeType::Table, *table)?;
            remove_nth_block_child(node_at_path_mut(&mut out.root, &path)?, *row)?;
        }
        Mutation::AddListItem { list, text } => {
            let path = nth_list_path(&out.root, *list)?;
            let node = node_at_path_mut(&mut out.root, &path)?;
            let pos = node.pos;
            node.children.push(XmlChild::Block(XmlNode {
                dot: None,
                pos,
                node: PlainNode::ListItem(PlainListItemNode {}),
                modifiers: BTreeMap::new(),
                carry: BTreeMap::new(),
                children: vec![XmlChild::Block(new_paragraph(pos, text))],
            }));
        }
        Mutation::DeleteListItem { list, item } => {
            let path = nth_list_path(&out.root, *list)?;
            remove_nth_block_child(node_at_path_mut(&mut out.root, &path)?, *item)?;
        }
        Mutation::SetAlignment { paragraph, value } => {
            let block = authored_textblock_at(&mut out.root, *paragraph)?;
            if block.node.as_type() != NodeType::Paragraph {
                return None;
            }
            block.modifiers.insert(
                ModifierType::Alignment,
                Modifier::Alignment { value: *value },
            );
        }
        Mutation::ReplaceAll { paragraphs } => {
            let pos = out.root.pos;
            out.root.children = paragraphs
                .iter()
                .map(|text| XmlChild::Block(new_paragraph(pos, text)))
                .collect();
        }
    }
    if !replaces_root && synthetic_dots(&tree.root) != synthetic_dots(&out.root) {
        return None;
    }
    if writes_behind_the_trailing_scaffold(&tree.root, &out.root) {
        return None;
    }
    crate::reader::validate_schema(&out.root, &[]).ok()?;
    Some(out)
}

/// The scaffold paragraph the projection put at the end of a root whose last
/// paragraph ends in a page break. Unknown children are skipped, the way the
/// projection reads the last child.
fn trailing_scaffold_after_a_page_break(root: &XmlNode) -> Option<Dot> {
    fn ends_in_a_page_break(node: &XmlNode) -> bool {
        let last = node.children.iter().rev().find(|child| match child {
            XmlChild::Block(block) => block.node.as_type() != NodeType::Unknown,
            XmlChild::Inline(entry) => match &entry.leaf {
                InlineLeaf::Char(_) => true,
                InlineLeaf::Atom(node) => node.as_type() != NodeType::Unknown,
            },
        });
        node.node.as_type() == NodeType::Paragraph
            && matches!(
                last,
                Some(XmlChild::Inline(entry))
                    if matches!(&entry.leaf, InlineLeaf::Atom(node) if node.as_type() == NodeType::PageBreak)
            )
    }
    let known: Vec<&XmlNode> = root
        .block_children()
        .filter(|block| block.node.as_type() != NodeType::Unknown)
        .collect();
    let [.., before, last] = known.as_slice() else {
        return None;
    };
    let scaffold = last.dot.filter(Dot::is_synthetic)?;
    (last.node.as_type() == NodeType::Paragraph && ends_in_a_page_break(before)).then_some(scaffold)
}

/// A target that writes a block behind that scaffold. The incremental
/// projection keeps a scaffold a fresh projection drops, so the document the
/// edit lands on is not the document the file describes.
fn writes_behind_the_trailing_scaffold(base: &XmlNode, out: &XmlNode) -> bool {
    let Some(scaffold) = trailing_scaffold_after_a_page_break(base) else {
        return false;
    };
    let Some(index) = out
        .children
        .iter()
        .position(|child| matches!(child, XmlChild::Block(block) if block.dot == Some(scaffold)))
    else {
        return false;
    };
    index + 1 < out.children.len()
}

/// The containers a block can be moved into: everything whose content rule is
/// an open list of blocks. `Fold`, `Table` and `TableRow` are left out — their
/// content is a fixed sequence of roles, not a place for authored blocks.
fn is_open_container(node: &XmlNode) -> bool {
    matches!(
        node.node.as_type(),
        NodeType::Blockquote
            | NodeType::Callout
            | NodeType::ListItem
            | NodeType::FoldContent
            | NodeType::TableCell
    )
}

/// Paths, in document order, to every block the predicate accepts. A path is
/// the chain of child indices that reaches it from `root`.
fn block_paths(root: &XmlNode, want: &dyn Fn(&XmlNode) -> bool) -> Vec<Vec<usize>> {
    fn walk(
        node: &XmlNode,
        want: &dyn Fn(&XmlNode) -> bool,
        prefix: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if want(node) {
            out.push(prefix.clone());
        }
        for (i, child) in node.children.iter().enumerate() {
            if let XmlChild::Block(block) = child {
                prefix.push(i);
                walk(block, want, prefix, out);
                prefix.pop();
            }
        }
    }
    let mut out = Vec::new();
    walk(root, want, &mut Vec::new(), &mut out);
    out
}

fn node_at_path_mut<'a>(root: &'a mut XmlNode, path: &[usize]) -> Option<&'a mut XmlNode> {
    let mut node = root;
    for i in path {
        node = match node.children.get_mut(*i)? {
            XmlChild::Block(block) => block,
            XmlChild::Inline(_) => return None,
        };
    }
    Some(node)
}

fn nth_path(root: &XmlNode, node_type: NodeType, n: usize) -> Option<Vec<usize>> {
    block_paths(root, &|node| node.node.as_type() == node_type)
        .into_iter()
        .nth(n)
}

fn nth_list_path(root: &XmlNode, n: usize) -> Option<Vec<usize>> {
    block_paths(root, &|node| {
        matches!(
            node.node.as_type(),
            NodeType::BulletList | NodeType::OrderedList
        )
    })
    .into_iter()
    .nth(n)
}

fn authored_paragraph_path(root: &XmlNode, n: usize) -> Option<Vec<usize>> {
    block_paths(root, &|node| {
        node.node.as_type() == NodeType::Paragraph && !node.dot.is_some_and(|d| d.is_synthetic())
    })
    .into_iter()
    .nth(n)
}

/// Drops the nth block child, refused when it is the only one: a content rule
/// of `+` cannot be emptied, so the reader would refuse the file and the diff
/// could never reach it.
fn remove_nth_block_child(node: &mut XmlNode, n: usize) -> Option<()> {
    let slots: Vec<usize> = node
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, child)| matches!(child, XmlChild::Block(_)).then_some(i))
        .collect();
    if slots.len() <= 1 {
        return None;
    }
    node.children.remove(*slots.get(n)?);
    Some(())
}

/// The blocks a container gives up when its element lines are removed: its own
/// block children, or, when those cannot stand at the root as a list item
/// cannot, theirs. Whether the root still reads as a root once the run is
/// spliced in is `validate_schema`'s answer at the end of the mutation.
fn lifted_children(node: &XmlNode) -> Option<Vec<XmlChild>> {
    let direct: Vec<XmlNode> = node.block_children().cloned().collect();
    if direct.is_empty() || direct.len() != node.children.len() {
        return None;
    }
    if root_takes_each(&direct) {
        return Some(direct.into_iter().map(XmlChild::Block).collect());
    }
    let mut deeper: Vec<XmlNode> = Vec::new();
    for child in &direct {
        let grand: Vec<XmlNode> = child.block_children().cloned().collect();
        if grand.is_empty() || grand.len() != child.children.len() {
            return None;
        }
        deeper.extend(grand);
    }
    root_takes_each(&deeper).then(|| deeper.into_iter().map(XmlChild::Block).collect())
}

fn root_takes_each(children: &[XmlNode]) -> bool {
    let allowed = NodeType::Root.spec().content.allowed_types();
    children
        .iter()
        .all(|child| allowed.contains(&child.node.as_type()))
}

fn new_paragraph(pos: Pos, text: &str) -> XmlNode {
    XmlNode {
        dot: None,
        pos,
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: BTreeMap::new(),
        carry: BTreeMap::new(),
        children: text
            .chars()
            .map(|ch| {
                XmlChild::Inline(InlineEntry {
                    pos,
                    leaf: InlineLeaf::Char(ch),
                    own: BTreeMap::new(),
                })
            })
            .collect(),
    }
}

fn root_child(root: &XmlNode, index: usize) -> Option<&XmlNode> {
    match root.children.get(index)? {
        XmlChild::Block(b) => Some(b),
        XmlChild::Inline(_) => None,
    }
}

fn synthetic_dots(node: &XmlNode) -> Vec<Dot> {
    let mut out = Vec::new();
    fn walk(node: &XmlNode, out: &mut Vec<Dot>) {
        if let Some(d) = node.dot.filter(Dot::is_synthetic) {
            out.push(d);
        }
        for child in node.block_children() {
            walk(child, out);
        }
    }
    walk(node, &mut out);
    out
}

fn textblock_paths(node: &XmlNode, prefix: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if is_textblock(node.node.as_type()) {
        out.push(prefix.clone());
        return;
    }
    for (i, child) in node.children.iter().enumerate() {
        if let XmlChild::Block(block) = child {
            prefix.push(i);
            textblock_paths(block, prefix, out);
            prefix.pop();
        }
    }
}

/// The nth textblock, refused when it is a projection-owned scaffold: a
/// scaffold has no authored identity to edit, and touching one materializes it
/// into a different dot.
fn authored_textblock_at(root: &mut XmlNode, n: usize) -> Option<&mut XmlNode> {
    let block = textblock_at(root, n)?;
    (!block.dot.is_some_and(|d| d.is_synthetic())).then_some(block)
}

fn textblock_at(root: &mut XmlNode, n: usize) -> Option<&mut XmlNode> {
    let mut paths = Vec::new();
    textblock_paths(root, &mut Vec::new(), &mut paths);
    let path = paths.into_iter().nth(n)?;
    let mut node = root;
    for i in path {
        node = match &mut node.children[i] {
            XmlChild::Block(block) => block,
            XmlChild::Inline(_) => return None,
        };
    }
    Some(node)
}

fn inline_at(block: &mut XmlNode, at: usize) -> Option<&mut InlineEntry> {
    match block.children.get_mut(at)? {
        XmlChild::Inline(entry) => Some(entry),
        XmlChild::Block(_) => None,
    }
}
