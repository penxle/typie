use crate::model::html::builder::{DomSpec, HtmlBuilder};
use crate::model::{Fragment, Mark, Node, NodeId};
use scraper::ElementRef;

pub struct HtmlContext<'a> {
    pub fragment: &'a Fragment,
}

impl<'a> HtmlContext<'a> {
    pub fn new(fragment: &'a Fragment) -> Self {
        Self { fragment }
    }

    pub fn write_children(&self, id: NodeId, b: &mut HtmlBuilder) {
        for (child_id, child) in self.fragment.children_of_node(id) {
            if let Some(spec) = child.data().to_dom() {
                render_node_spec(&spec, self, child_id, b);
            }
        }
    }
}

pub trait NodeHtmlCodec {
    fn to_dom(&self) -> Option<DomSpec> {
        None
    }

    fn parse_rules() -> Vec<NodeParseRule>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub fn render_node_spec(spec: &DomSpec, ctx: &HtmlContext, id: NodeId, b: &mut HtmlBuilder) {
    match spec {
        DomSpec::Hole => ctx.write_children(id, b),
        DomSpec::Text(t) => b.text(t),
        DomSpec::Element {
            tag,
            attrs,
            children,
        } => {
            let mut tb = b.open(tag);
            for (k, v) in attrs {
                tb = tb.attr(k, v);
            }
            tb.children(|b| {
                for child in children {
                    render_node_spec(child, ctx, id, b);
                }
            });
        }
        DomSpec::Void { tag, attrs } => {
            let mut tb = b.open(tag);
            for (k, v) in attrs {
                tb = tb.attr(k, v);
            }
            tb.void();
        }
        DomSpec::Fragment(specs) => {
            for spec in specs {
                render_node_spec(spec, ctx, id, b);
            }
        }
    }
}

pub trait MarkHtmlCodec {
    fn to_dom(&self) -> DomSpec;
    fn parse_rules() -> Vec<MarkParseRule>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub struct NodeParseRule {
    pub tag: &'static str,
    pub priority: u8,
    pub matches: fn(&ElementRef) -> bool,
    pub parse: fn(&ElementRef) -> Option<Node>,
}

impl NodeParseRule {
    pub fn new(
        tag: &'static str,
        priority: u8,
        matches: fn(&ElementRef) -> bool,
        parse: fn(&ElementRef) -> Option<Node>,
    ) -> Self {
        Self {
            tag,
            priority,
            matches,
            parse,
        }
    }

    pub fn simple(tag: &'static str, parse: fn(&ElementRef) -> Option<Node>) -> Self {
        Self {
            tag,
            priority: 50,
            matches: |_| true,
            parse,
        }
    }
}

pub struct MarkParseRule {
    pub tag: Option<&'static str>,
    pub style_key: Option<&'static str>,
    pub data_attr: Option<&'static str>,
    pub priority: u8,
    pub parse: fn(&ElementRef) -> Option<Mark>,
    pub get_content: Option<fn(&ElementRef) -> String>,
}

impl MarkParseRule {
    pub fn from_tag(tag: &'static str, parse: fn(&ElementRef) -> Option<Mark>) -> Self {
        Self {
            tag: Some(tag),
            style_key: None,
            data_attr: None,
            priority: 50,
            parse,
            get_content: None,
        }
    }

    pub fn from_tag_with_content(
        tag: &'static str,
        parse: fn(&ElementRef) -> Option<Mark>,
        get_content: fn(&ElementRef) -> String,
    ) -> Self {
        Self {
            tag: Some(tag),
            style_key: None,
            data_attr: None,
            priority: 50,
            parse,
            get_content: Some(get_content),
        }
    }

    pub fn from_style(style_key: &'static str, parse: fn(&ElementRef) -> Option<Mark>) -> Self {
        Self {
            tag: None,
            style_key: Some(style_key),
            data_attr: None,
            priority: 30,
            parse,
            get_content: None,
        }
    }

    pub fn from_data(data_attr: &'static str, parse: fn(&ElementRef) -> Option<Mark>) -> Self {
        Self {
            tag: None,
            style_key: None,
            data_attr: Some(data_attr),
            priority: 60,
            parse,
            get_content: None,
        }
    }
}

use crate::model::marks::*;
use crate::model::nodes::*;

pub fn collect_node_parse_rules() -> Vec<NodeParseRule> {
    let mut rules = Vec::new();
    rules.extend(ParagraphNode::parse_rules());
    rules.extend(BlockquoteNode::parse_rules());
    rules.extend(BulletListNode::parse_rules());
    rules.extend(OrderedListNode::parse_rules());
    rules.extend(ListItemNode::parse_rules());
    rules.extend(FoldNode::parse_rules());
    rules.extend(FoldTitleNode::parse_rules());
    rules.extend(FoldContentNode::parse_rules());
    rules.extend(ImageNode::parse_rules());
    rules.extend(EmbedNode::parse_rules());
    rules.extend(HorizontalRuleNode::parse_rules());
    rules.extend(HardBreakNode::parse_rules());
    rules.extend(PageBreakNode::parse_rules());
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    rules
}

pub fn collect_mark_parse_rules() -> Vec<MarkParseRule> {
    let mut rules = Vec::new();
    rules.extend(ItalicMark::parse_rules());
    rules.extend(UnderlineMark::parse_rules());
    rules.extend(StrikethroughMark::parse_rules());
    rules.extend(FontWeightMark::parse_rules());
    rules.extend(FontSizeMark::parse_rules());
    rules.extend(FontFamilyMark::parse_rules());
    rules.extend(LetterSpacingMark::parse_rules());
    rules.extend(TextColorMark::parse_rules());
    rules.extend(BackgroundColorMark::parse_rules());
    rules.extend(RubyMark::parse_rules());
    rules.extend(LinkMark::parse_rules());
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    rules
}

pub fn try_parse_node(elem: &ElementRef, rules: &[NodeParseRule]) -> Option<Node> {
    let tag = elem.value().name();
    for rule in rules {
        if rule.tag == tag && (rule.matches)(elem) {
            if let Some(node) = (rule.parse)(elem) {
                return Some(node);
            }
        }
    }
    None
}

pub struct ParsedMarks {
    pub marks: Vec<Mark>,
    pub custom_content: Option<String>,
}

pub fn try_parse_marks(elem: &ElementRef, rules: &[MarkParseRule]) -> ParsedMarks {
    let tag = elem.value().name();
    let mut marks = Vec::new();
    let mut custom_content = None;

    for rule in rules {
        let matches = if let Some(rule_tag) = rule.tag {
            rule_tag == tag
        } else if rule.style_key.is_some() || rule.data_attr.is_some() {
            true
        } else {
            false
        };

        if matches {
            if let Some(mark) = (rule.parse)(elem) {
                marks.push(mark);
                if let Some(get_content) = rule.get_content {
                    custom_content = Some(get_content(elem));
                }
            }
        }
    }

    ParsedMarks {
        marks,
        custom_content,
    }
}
