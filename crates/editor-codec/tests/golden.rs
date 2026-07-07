use editor_codec::ctx::{DecCtx, EncCtx};
use editor_codec::durable::Durable;
use editor_codec::framing::{UnknownPayload, UnknownTail};
use editor_codec::registry::all_type_schemas;
use editor_codec::schema::SchemaKind;
use editor_codec::types::*;
use editor_crdt::Dot;

fn enc() -> EncCtx {
    EncCtx::from_parts(&[3, 9], vec![10, 0]).unwrap()
}

fn dec() -> DecCtx {
    DecCtx {
        actors: vec![3, 9],
        baselines: vec![10, 0],
    }
}

fn bytes<T: Durable>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.encode(&enc(), &mut out).unwrap();
    out
}

/// Decodes `bytes` as `T` under the corpus's fixed ctx, then re-encodes —
/// every golden fixture is decode-reencode-identical.
fn redecode<T: Durable>(bytes: &[u8]) -> Vec<u8> {
    let d = dec();
    let mut slice = bytes;
    let value = T::decode(&d, &mut slice).unwrap();
    assert!(slice.is_empty(), "trailing bytes after decode");
    let mut out = Vec::new();
    value.encode(&enc(), &mut out).unwrap();
    out
}

type Fixture = (&'static str, Vec<u8>, fn(&[u8]) -> Vec<u8>);

fn corpus() -> Vec<Fixture> {
    let anchor3 = Dot::new(3, 12);
    let anchor9 = Dot::new(9, 0);

    vec![
        // ----- DurableBias (closed) -----
        (
            "DurableBias::Before",
            bytes(&DurableBias::Before),
            redecode::<DurableBias>,
        ),
        (
            "DurableBias::After",
            bytes(&DurableBias::After),
            redecode::<DurableBias>,
        ),
        // ----- DurableAnchor (frozen struct) -----
        (
            "DurableAnchor",
            bytes(&DurableAnchor {
                id: anchor3,
                bias: DurableBias::Before,
            }),
            redecode::<DurableAnchor>,
        ),
        // ----- DurableAlignment (open) -----
        (
            "DurableAlignment::Left",
            bytes(&DurableAlignment::Left),
            redecode::<DurableAlignment>,
        ),
        (
            "DurableAlignment::Center",
            bytes(&DurableAlignment::Center),
            redecode::<DurableAlignment>,
        ),
        (
            "DurableAlignment::Right",
            bytes(&DurableAlignment::Right),
            redecode::<DurableAlignment>,
        ),
        (
            "DurableAlignment::Justify",
            bytes(&DurableAlignment::Justify),
            redecode::<DurableAlignment>,
        ),
        (
            "DurableAlignment::Unknown",
            bytes(&DurableAlignment::Unknown(UnknownPayload {
                tag: 77,
                bytes: vec![1, 2],
            })),
            redecode::<DurableAlignment>,
        ),
        // ----- DurableBlockquoteVariant (open) -----
        (
            "DurableBlockquoteVariant::LeftLine",
            bytes(&DurableBlockquoteVariant::LeftLine),
            redecode::<DurableBlockquoteVariant>,
        ),
        (
            "DurableBlockquoteVariant::LeftQuote",
            bytes(&DurableBlockquoteVariant::LeftQuote),
            redecode::<DurableBlockquoteVariant>,
        ),
        (
            "DurableBlockquoteVariant::MessageSent",
            bytes(&DurableBlockquoteVariant::MessageSent),
            redecode::<DurableBlockquoteVariant>,
        ),
        (
            "DurableBlockquoteVariant::MessageReceived",
            bytes(&DurableBlockquoteVariant::MessageReceived),
            redecode::<DurableBlockquoteVariant>,
        ),
        (
            "DurableBlockquoteVariant::Unknown",
            bytes(&DurableBlockquoteVariant::Unknown(UnknownPayload {
                tag: 78,
                bytes: vec![3, 4],
            })),
            redecode::<DurableBlockquoteVariant>,
        ),
        // ----- DurableCalloutVariant (open) -----
        (
            "DurableCalloutVariant::Info",
            bytes(&DurableCalloutVariant::Info),
            redecode::<DurableCalloutVariant>,
        ),
        (
            "DurableCalloutVariant::Success",
            bytes(&DurableCalloutVariant::Success),
            redecode::<DurableCalloutVariant>,
        ),
        (
            "DurableCalloutVariant::Warning",
            bytes(&DurableCalloutVariant::Warning),
            redecode::<DurableCalloutVariant>,
        ),
        (
            "DurableCalloutVariant::Danger",
            bytes(&DurableCalloutVariant::Danger),
            redecode::<DurableCalloutVariant>,
        ),
        (
            "DurableCalloutVariant::Unknown",
            bytes(&DurableCalloutVariant::Unknown(UnknownPayload {
                tag: 79,
                bytes: vec![5, 6],
            })),
            redecode::<DurableCalloutVariant>,
        ),
        // ----- DurableHorizontalRuleVariant (open) -----
        (
            "DurableHorizontalRuleVariant::Line",
            bytes(&DurableHorizontalRuleVariant::Line),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::DashedLine",
            bytes(&DurableHorizontalRuleVariant::DashedLine),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::CircleLine",
            bytes(&DurableHorizontalRuleVariant::CircleLine),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::DiamondLine",
            bytes(&DurableHorizontalRuleVariant::DiamondLine),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::Circle",
            bytes(&DurableHorizontalRuleVariant::Circle),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::Diamond",
            bytes(&DurableHorizontalRuleVariant::Diamond),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::ThreeCircles",
            bytes(&DurableHorizontalRuleVariant::ThreeCircles),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::ThreeDiamonds",
            bytes(&DurableHorizontalRuleVariant::ThreeDiamonds),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::Zigzag",
            bytes(&DurableHorizontalRuleVariant::Zigzag),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        (
            "DurableHorizontalRuleVariant::Unknown",
            bytes(&DurableHorizontalRuleVariant::Unknown(UnknownPayload {
                tag: 80,
                bytes: vec![7, 8],
            })),
            redecode::<DurableHorizontalRuleVariant>,
        ),
        // ----- DurableLayoutMode (open) -----
        (
            "DurableLayoutMode::Paginated",
            bytes(&DurableLayoutMode::Paginated {
                page_width: 800,
                page_height: 1200,
                page_margin_top: 40,
                page_margin_bottom: 40,
                page_margin_left: 30,
                page_margin_right: 30,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableLayoutMode>,
        ),
        (
            "DurableLayoutMode::Continuous",
            bytes(&DurableLayoutMode::Continuous {
                max_width: 1400,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableLayoutMode>,
        ),
        (
            "DurableLayoutMode::Unknown",
            bytes(&DurableLayoutMode::Unknown(UnknownPayload {
                tag: 81,
                bytes: vec![9, 10],
            })),
            redecode::<DurableLayoutMode>,
        ),
        // ----- DurableTableBorderStyle (open) -----
        (
            "DurableTableBorderStyle::Solid",
            bytes(&DurableTableBorderStyle::Solid),
            redecode::<DurableTableBorderStyle>,
        ),
        (
            "DurableTableBorderStyle::Dashed",
            bytes(&DurableTableBorderStyle::Dashed),
            redecode::<DurableTableBorderStyle>,
        ),
        (
            "DurableTableBorderStyle::Dotted",
            bytes(&DurableTableBorderStyle::Dotted),
            redecode::<DurableTableBorderStyle>,
        ),
        (
            "DurableTableBorderStyle::None",
            bytes(&DurableTableBorderStyle::None),
            redecode::<DurableTableBorderStyle>,
        ),
        (
            "DurableTableBorderStyle::Unknown",
            bytes(&DurableTableBorderStyle::Unknown(UnknownPayload {
                tag: 82,
                bytes: vec![11, 12],
            })),
            redecode::<DurableTableBorderStyle>,
        ),
        // ----- DurableModifier (open) -----
        (
            "DurableModifier::Bold",
            bytes(&DurableModifier::Bold),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Italic",
            bytes(&DurableModifier::Italic),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Underline",
            bytes(&DurableModifier::Underline),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Strikethrough",
            bytes(&DurableModifier::Strikethrough),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::FontSize",
            bytes(&DurableModifier::FontSize(1400)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::FontFamily",
            bytes(&DurableModifier::FontFamily("golden".to_owned())),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::FontWeight",
            bytes(&DurableModifier::FontWeight(400)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::TextColor",
            bytes(&DurableModifier::TextColor("golden".to_owned())),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::BackgroundColor",
            bytes(&DurableModifier::BackgroundColor("golden".to_owned())),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::LetterSpacing",
            bytes(&DurableModifier::LetterSpacing(-25)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Link",
            bytes(&DurableModifier::Link("golden".to_owned())),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Ruby",
            bytes(&DurableModifier::Ruby("golden".to_owned())),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::LineHeight",
            bytes(&DurableModifier::LineHeight(1400)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::BlockGap",
            bytes(&DurableModifier::BlockGap(1400)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::ParagraphIndent",
            bytes(&DurableModifier::ParagraphIndent(1400)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Alignment",
            bytes(&DurableModifier::Alignment(DurableAlignment::Center)),
            redecode::<DurableModifier>,
        ),
        (
            "DurableModifier::Unknown",
            bytes(&DurableModifier::Unknown(UnknownPayload {
                tag: 83,
                bytes: vec![13, 14],
            })),
            redecode::<DurableModifier>,
        ),
        // ----- DurableModifierKind (open) -----
        (
            "DurableModifierKind::Bold",
            bytes(&DurableModifierKind::Bold),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Italic",
            bytes(&DurableModifierKind::Italic),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Underline",
            bytes(&DurableModifierKind::Underline),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Strikethrough",
            bytes(&DurableModifierKind::Strikethrough),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::FontSize",
            bytes(&DurableModifierKind::FontSize),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::FontFamily",
            bytes(&DurableModifierKind::FontFamily),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::FontWeight",
            bytes(&DurableModifierKind::FontWeight),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::TextColor",
            bytes(&DurableModifierKind::TextColor),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::BackgroundColor",
            bytes(&DurableModifierKind::BackgroundColor),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::LetterSpacing",
            bytes(&DurableModifierKind::LetterSpacing),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Link",
            bytes(&DurableModifierKind::Link),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Ruby",
            bytes(&DurableModifierKind::Ruby),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::LineHeight",
            bytes(&DurableModifierKind::LineHeight),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::BlockGap",
            bytes(&DurableModifierKind::BlockGap),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::ParagraphIndent",
            bytes(&DurableModifierKind::ParagraphIndent),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Alignment",
            bytes(&DurableModifierKind::Alignment),
            redecode::<DurableModifierKind>,
        ),
        (
            "DurableModifierKind::Unknown",
            bytes(&DurableModifierKind::Unknown(UnknownPayload {
                tag: 84,
                bytes: vec![15, 16],
            })),
            redecode::<DurableModifierKind>,
        ),
        // ----- DurableAttr (open) -----
        (
            "DurableAttr::RootLayoutMode",
            bytes(&DurableAttr::RootLayoutMode(
                DurableLayoutMode::Continuous {
                    max_width: 1400,
                    tail: UnknownTail(Vec::new()),
                },
            )),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::BlockquoteVariant",
            bytes(&DurableAttr::BlockquoteVariant(
                DurableBlockquoteVariant::LeftLine,
            )),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::CalloutVariant",
            bytes(&DurableAttr::CalloutVariant(DurableCalloutVariant::Info)),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::TableBorderStyle",
            bytes(&DurableAttr::TableBorderStyle(
                DurableTableBorderStyle::Solid,
            )),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::TableProportion",
            bytes(&DurableAttr::TableProportion(1400)),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::TableCellColWidth",
            bytes(&DurableAttr::TableCellColWidth(Some(1400))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::TableCellBackgroundColor",
            bytes(&DurableAttr::TableCellBackgroundColor(Some(
                "golden".to_owned(),
            ))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::ImageId",
            bytes(&DurableAttr::ImageId(Some("golden".to_owned()))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::ImageProportion",
            bytes(&DurableAttr::ImageProportion(1400)),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::FileId",
            bytes(&DurableAttr::FileId(Some("golden".to_owned()))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::EmbedId",
            bytes(&DurableAttr::EmbedId(Some("golden".to_owned()))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::ArchivedId",
            bytes(&DurableAttr::ArchivedId(Some("golden".to_owned()))),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::HorizontalRuleVariant",
            bytes(&DurableAttr::HorizontalRuleVariant(
                DurableHorizontalRuleVariant::Line,
            )),
            redecode::<DurableAttr>,
        ),
        (
            "DurableAttr::Unknown",
            bytes(&DurableAttr::Unknown(UnknownPayload {
                tag: 85,
                bytes: vec![17, 18],
            })),
            redecode::<DurableAttr>,
        ),
        // ----- DurableNodeType (open) -----
        (
            "DurableNodeType::Root",
            bytes(&DurableNodeType::Root),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Paragraph",
            bytes(&DurableNodeType::Paragraph),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Blockquote",
            bytes(&DurableNodeType::Blockquote),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Callout",
            bytes(&DurableNodeType::Callout),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Text",
            bytes(&DurableNodeType::Text),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::BulletList",
            bytes(&DurableNodeType::BulletList),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::OrderedList",
            bytes(&DurableNodeType::OrderedList),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::ListItem",
            bytes(&DurableNodeType::ListItem),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Fold",
            bytes(&DurableNodeType::Fold),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::FoldTitle",
            bytes(&DurableNodeType::FoldTitle),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::FoldContent",
            bytes(&DurableNodeType::FoldContent),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Table",
            bytes(&DurableNodeType::Table),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::TableRow",
            bytes(&DurableNodeType::TableRow),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::TableCell",
            bytes(&DurableNodeType::TableCell),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Image",
            bytes(&DurableNodeType::Image),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::File",
            bytes(&DurableNodeType::File),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Embed",
            bytes(&DurableNodeType::Embed),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Archived",
            bytes(&DurableNodeType::Archived),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::HardBreak",
            bytes(&DurableNodeType::HardBreak),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::HorizontalRule",
            bytes(&DurableNodeType::HorizontalRule),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::PageBreak",
            bytes(&DurableNodeType::PageBreak),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Tab",
            bytes(&DurableNodeType::Tab),
            redecode::<DurableNodeType>,
        ),
        (
            "DurableNodeType::Unknown",
            bytes(&DurableNodeType::Unknown(UnknownPayload {
                tag: 86,
                bytes: vec![19, 20],
            })),
            redecode::<DurableNodeType>,
        ),
        // ----- DurableItem (open) -----
        (
            "DurableItem::Char",
            bytes(&DurableItem::Char('a')),
            redecode::<DurableItem>,
        ),
        (
            "DurableItem::Atom",
            bytes(&DurableItem::Atom {
                node_type: DurableNodeType::HardBreak,
                init: vec![],
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableItem>,
        ),
        (
            "DurableItem::Block",
            bytes(&DurableItem::Block {
                node_type: DurableNodeType::Paragraph,
                parents: vec![anchor3],
                init: vec![],
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableItem>,
        ),
        (
            "DurableItem::BlockAtom",
            bytes(&DurableItem::BlockAtom {
                node_type: DurableNodeType::Image,
                parents: vec![anchor9],
                init: vec![],
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableItem>,
        ),
        (
            "DurableItem::Unknown",
            bytes(&DurableItem::Unknown(UnknownPayload {
                tag: 87,
                bytes: vec![21, 22],
            })),
            redecode::<DurableItem>,
        ),
        // ----- DurableOp (open) -----
        (
            "DurableOp::SeqIns",
            bytes(&DurableOp::SeqIns {
                pos: 1400,
                item: DurableItem::Char('a'),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::SeqDel",
            bytes(&DurableOp::SeqDel {
                pos: 1400,
                len: 1400,
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::SeqUndel",
            bytes(&DurableOp::SeqUndel { del: anchor3 }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::AddSpan",
            bytes(&DurableOp::AddSpan {
                start: DurableAnchor {
                    id: anchor3,
                    bias: DurableBias::Before,
                },
                end: DurableAnchor {
                    id: anchor9,
                    bias: DurableBias::After,
                },
                modifier: DurableModifier::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::RemoveSpan",
            bytes(&DurableOp::RemoveSpan {
                start: DurableAnchor {
                    id: anchor3,
                    bias: DurableBias::Before,
                },
                end: DurableAnchor {
                    id: anchor9,
                    bias: DurableBias::After,
                },
                kind: DurableModifierKind::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::SetBlockModifier",
            bytes(&DurableOp::SetBlockModifier {
                target: anchor3,
                modifier: DurableModifier::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::ClearBlockModifier",
            bytes(&DurableOp::ClearBlockModifier {
                target: anchor3,
                kind: DurableModifierKind::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::SetNodeAttr",
            bytes(&DurableOp::SetNodeAttr {
                target: anchor3,
                attr: DurableAttr::TableProportion(1400),
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::SetNodeCarry",
            bytes(&DurableOp::SetNodeCarry {
                target: anchor3,
                modifier: DurableModifier::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::ClearNodeCarry",
            bytes(&DurableOp::ClearNodeCarry {
                target: anchor3,
                kind: DurableModifierKind::Bold,
                tail: UnknownTail(Vec::new()),
            }),
            redecode::<DurableOp>,
        ),
        (
            "DurableOp::Unknown",
            bytes(&DurableOp::Unknown(UnknownPayload {
                tag: 88,
                bytes: vec![23, 24],
            })),
            redecode::<DurableOp>,
        ),
    ]
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn golden_bytes_are_stable() {
    let rendered: String = corpus()
        .iter()
        .map(|(label, bytes, _)| format!("{label}\t{}\n", hex(bytes)))
        .collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/golden.jsonl");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(path, &rendered).unwrap();
    }
    let on_disk = std::fs::read_to_string(path).expect("golden 픽스처 필요 — UPDATE_GOLDEN=1");
    assert_eq!(
        on_disk, rendered,
        "인코딩 바이트가 변했다 — 스키마/코덱 드리프트"
    );
}

#[test]
fn golden_covers_every_variant() {
    let labels: Vec<&str> = corpus().iter().map(|(l, _, _)| *l).collect();
    for ty in all_type_schemas() {
        match &ty.kind {
            SchemaKind::OpenEnum { variants, .. } | SchemaKind::ClosedEnum { variants } => {
                for v in variants {
                    let expected = format!("{}::{}", ty.name, v.name);
                    assert!(
                        labels.iter().any(|l| *l == expected),
                        "golden 누락: {expected}"
                    );
                }
                if matches!(ty.kind, SchemaKind::OpenEnum { .. }) {
                    let unknown = format!("{}::Unknown", ty.name);
                    assert!(
                        labels.iter().any(|l| *l == unknown),
                        "golden 누락: {unknown}"
                    );
                }
            }
            _ => {
                assert!(
                    labels
                        .iter()
                        .any(|l| *l == ty.name || l.starts_with(&format!("{}::", ty.name))),
                    "golden 누락(struct — 전 Durable 타입이 corpus에 있어야 한다): {}",
                    ty.name
                );
            }
        }
    }
}

#[test]
fn golden_round_trips() {
    for (label, bytes, redecode) in corpus() {
        assert_eq!(redecode(&bytes), bytes, "{label} decode-reencode 항등 실패");
    }
}
