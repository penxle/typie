use editor_model::{Modifier, PlainNode};
use scraper::ElementRef;
use std::cmp::Reverse;
use std::sync::OnceLock;

pub struct NodeParseRule {
    pub tag: &'static str,
    pub priority: u8,
    pub matches: fn(&ElementRef) -> bool,
    pub parse: fn(&ElementRef) -> Option<PlainNode>,
}

impl NodeParseRule {
    pub const fn simple(tag: &'static str, parse: fn(&ElementRef) -> Option<PlainNode>) -> Self {
        Self {
            tag,
            priority: 50,
            matches: |_| true,
            parse,
        }
    }
}

pub struct ModifierParseRule {
    pub matcher: ModifierMatcher,
    pub priority: u8,
    pub parse: fn(&ElementRef, ModifierMatchContext<'_>) -> Vec<Modifier>,
}

pub enum ModifierMatcher {
    Tag(&'static str),
    StyleProperty(&'static str),
    DataAttr(&'static str),
}

pub struct ModifierMatchContext<'a> {
    pub value: Option<&'a str>,
}

impl<'a> ModifierMatchContext<'a> {
    pub fn none() -> Self {
        Self { value: None }
    }
    pub fn with_value(v: &'a str) -> Self {
        Self { value: Some(v) }
    }
}

pub fn node_parse_rules() -> &'static [NodeParseRule] {
    static RULES: OnceLock<Vec<NodeParseRule>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut v = build_node_rules();
            v.sort_by_key(|r| Reverse(r.priority));
            v
        })
        .as_slice()
}

pub fn modifier_parse_rules() -> &'static [ModifierParseRule] {
    static RULES: OnceLock<Vec<ModifierParseRule>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut v = build_modifier_rules();
            v.sort_by_key(|r| Reverse(r.priority));
            v
        })
        .as_slice()
}

fn build_node_rules() -> Vec<NodeParseRule> {
    use editor_model::{
        PlainBlockquoteNode, PlainBulletListNode, PlainCalloutNode, PlainFoldNode,
        PlainFoldTitleNode, PlainHardBreakNode, PlainHorizontalRuleNode, PlainListItemNode,
        PlainOrderedListNode, PlainParagraphNode, PlainTableCellNode, PlainTableNode,
        PlainTableRowNode,
    };
    vec![
        NodeParseRule::simple("p", |_| {
            Some(PlainNode::Paragraph(PlainParagraphNode::default()))
        }),
        NodeParseRule::simple("blockquote", |_| {
            Some(PlainNode::Blockquote(PlainBlockquoteNode::default()))
        }),
        NodeParseRule::simple("ul", |_| {
            Some(PlainNode::BulletList(PlainBulletListNode::default()))
        }),
        NodeParseRule::simple("ol", |_| {
            Some(PlainNode::OrderedList(PlainOrderedListNode::default()))
        }),
        NodeParseRule::simple("li", |_| {
            Some(PlainNode::ListItem(PlainListItemNode::default()))
        }),
        NodeParseRule::simple("table", |_| {
            Some(PlainNode::Table(PlainTableNode::default()))
        }),
        NodeParseRule::simple("tr", |_| {
            Some(PlainNode::TableRow(PlainTableRowNode::default()))
        }),
        NodeParseRule::simple("td", |_| {
            Some(PlainNode::TableCell(PlainTableCellNode::default()))
        }),
        NodeParseRule::simple("th", |_| {
            Some(PlainNode::TableCell(PlainTableCellNode::default()))
        }),
        NodeParseRule::simple("hr", |_| {
            Some(PlainNode::HorizontalRule(PlainHorizontalRuleNode::default()))
        }),
        NodeParseRule::simple("br", |elem| {
            if elem.value().attr("class").is_some_and(|class| {
                class
                    .split_ascii_whitespace()
                    .any(|v| v == "Apple-interchange-newline")
            }) {
                None
            } else {
                Some(PlainNode::HardBreak(PlainHardBreakNode::default()))
            }
        }),
        NodeParseRule::simple("details", |_| {
            Some(PlainNode::Fold(PlainFoldNode::default()))
        }),
        NodeParseRule::simple("summary", |_| {
            Some(PlainNode::FoldTitle(PlainFoldTitleNode::default()))
        }),
        NodeParseRule {
            tag: "aside",
            priority: 100,
            matches: |elem| elem.value().attr("data-callout").is_some(),
            parse: |elem| {
                let variant_s = elem.value().attr("data-variant").unwrap_or("info");
                let variant: editor_model::CalloutVariant =
                    serde_json::from_value(serde_json::Value::String(variant_s.to_string()))
                        .unwrap_or_default();
                Some(PlainNode::Callout(PlainCalloutNode { variant }))
            },
        },
    ]
}

fn build_modifier_rules() -> Vec<ModifierParseRule> {
    use crate::html::parse::value::{
        parse_font_weight, parse_length_to_pt_hundredths, parse_letter_spacing_to_em_hundredths,
        text_decoration_tokens,
    };

    let mut rules = vec![];

    for tag in ["strong", "b"] {
        rules.push(ModifierParseRule {
            matcher: ModifierMatcher::Tag(tag),
            priority: 100,
            parse: |_, _| vec![Modifier::Bold],
        });
    }
    for tag in ["em", "i"] {
        rules.push(ModifierParseRule {
            matcher: ModifierMatcher::Tag(tag),
            priority: 100,
            parse: |_, _| vec![Modifier::Italic],
        });
    }
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::Tag("u"),
        priority: 100,
        parse: |_, _| vec![Modifier::Underline],
    });
    for tag in ["s", "strike", "del"] {
        rules.push(ModifierParseRule {
            matcher: ModifierMatcher::Tag(tag),
            priority: 100,
            parse: |_, _| vec![Modifier::Strikethrough],
        });
    }
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::Tag("a"),
        priority: 100,
        parse: |elem, _| {
            let href = elem.value().attr("href").unwrap_or("").to_string();
            vec![Modifier::Link { href }]
        },
    });

    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("font-weight"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            let Some(w) = parse_font_weight(v) else {
                return vec![];
            };
            vec![Modifier::FontWeight { value: w }]
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("font-style"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            if v.trim().eq_ignore_ascii_case("italic") {
                vec![Modifier::Italic]
            } else {
                vec![]
            }
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("text-decoration"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            let t = text_decoration_tokens(v);
            let mut out = vec![];
            if t.iter().any(|x| x == "underline") {
                out.push(Modifier::Underline);
            }
            if t.iter().any(|x| x == "line-through") {
                out.push(Modifier::Strikethrough);
            }
            out
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("color"),
        priority: 50,
        parse: |_, ctx| {
            ctx.value
                .map(|v| {
                    vec![Modifier::TextColor {
                        value: v.trim().to_string(),
                    }]
                })
                .unwrap_or_default()
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("background-color"),
        priority: 50,
        parse: |_, ctx| {
            ctx.value
                .map(|v| {
                    vec![Modifier::BackgroundColor {
                        value: v.trim().to_string(),
                    }]
                })
                .unwrap_or_default()
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("font-size"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            parse_length_to_pt_hundredths(v)
                .map(|pt| vec![Modifier::FontSize { value: pt }])
                .unwrap_or_default()
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("font-family"),
        priority: 50,
        parse: |_, ctx| {
            ctx.value
                .map(|v| {
                    vec![Modifier::FontFamily {
                        value: v.trim().to_string(),
                    }]
                })
                .unwrap_or_default()
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("letter-spacing"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            parse_letter_spacing_to_em_hundredths(v)
                .map(|em| vec![Modifier::LetterSpacing { value: em }])
                .unwrap_or_default()
        },
    });
    rules.push(ModifierParseRule {
        matcher: ModifierMatcher::StyleProperty("text-align"),
        priority: 50,
        parse: |_, ctx| {
            let Some(v) = ctx.value else { return vec![] };
            let align = match v.trim().to_lowercase().as_str() {
                "left" => editor_model::Alignment::Left,
                "center" => editor_model::Alignment::Center,
                "right" => editor_model::Alignment::Right,
                "justify" => editor_model::Alignment::Justify,
                _ => return vec![],
            };
            vec![Modifier::Alignment { value: align }]
        },
    });

    rules
}

pub fn try_parse_node(elem: &ElementRef, rules: &[NodeParseRule]) -> Option<PlainNode> {
    let tag = elem.value().name();
    for rule in rules {
        if rule.tag == tag
            && (rule.matches)(elem)
            && let Some(n) = (rule.parse)(elem)
        {
            return Some(n);
        }
    }
    None
}

pub fn compute_modifiers_for_element(
    elem: &ElementRef,
    decls: &[(String, String)],
    rules: &[ModifierParseRule],
) -> Vec<Modifier> {
    let mut out: Vec<Modifier> = vec![];
    let tag = elem.value().name();
    let mut push_unique = |m: Modifier| {
        let t = m.as_type();
        if !out.iter().any(|x| x.as_type() == t) {
            out.push(m);
        }
    };
    for rule in rules {
        match &rule.matcher {
            ModifierMatcher::Tag(t) if *t == tag => {
                for m in (rule.parse)(elem, ModifierMatchContext::none()) {
                    push_unique(m);
                }
            }
            ModifierMatcher::StyleProperty(p) => {
                if let Some((_, v)) = decls.iter().find(|(k, _)| k == p) {
                    for m in (rule.parse)(elem, ModifierMatchContext::with_value(v)) {
                        push_unique(m);
                    }
                }
            }
            ModifierMatcher::DataAttr(a) if elem.value().attr(a).is_some() => {
                for m in (rule.parse)(elem, ModifierMatchContext::none()) {
                    push_unique(m);
                }
            }
            _ => {}
        }
    }
    out
}
