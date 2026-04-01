use crate::model::annotations::*;
use crate::model::html::builder::{DomSpec, HtmlBuilder};
use crate::model::nodes::*;
use crate::model::styles::*;
use crate::model::{Annotation, Fragment, Node, NodeId, Style};
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
            if let Some(spec) = self.node_to_dom(child.data()) {
                render_node(&spec, self, child_id, b);
            }
        }
    }

    pub fn node_to_dom(&self, node: &Node) -> Option<DomSpec> {
        match node {
            Node::Text(text_node) => {
                let specs: Vec<DomSpec> = text_node
                    .text
                    .get_segments()
                    .into_iter()
                    .map(|seg| {
                        let style_specs: Vec<DomSpec> = seg
                            .styles
                            .iter()
                            .map(|s| StyleHtmlCodec::to_dom(s))
                            .collect();
                        let ann_specs: Vec<DomSpec> = seg
                            .annotations
                            .iter()
                            .map(|a| AnnotationHtmlCodec::to_dom(a))
                            .collect();
                        let mut all_specs = ann_specs;
                        all_specs.extend(style_specs);
                        DomSpec::wrap_with_styles(seg.text, all_specs)
                    })
                    .collect();
                Some(DomSpec::Fragment(specs))
            }
            _ => node.to_dom(),
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

pub fn render_node(spec: &DomSpec, ctx: &HtmlContext, id: NodeId, b: &mut HtmlBuilder) {
    let id_str = id.to_string();
    match spec {
        DomSpec::Element {
            tag,
            attrs,
            children,
        } => {
            let mut tb = b.open(tag);
            tb = tb.attr("data-node-id", &id_str);
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
            tb = tb.attr("data-node-id", &id_str);
            for (k, v) in attrs {
                tb = tb.attr(k, v);
            }
            tb.void();
        }
        _ => {
            b.open("span").attr("data-node-id", &id_str).children(|b| {
                render_node_spec(spec, ctx, id, b);
            });
        }
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

pub trait StyleHtmlCodec {
    fn to_dom(&self) -> DomSpec;
    fn parse_rules() -> Vec<StyleParseRule>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub trait AnnotationHtmlCodec {
    fn to_dom(&self) -> DomSpec;
    fn parse_rules() -> Vec<AnnotationParseRule>
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
        if tag.contains('[') || tag.contains('.') || tag.contains('#') {
            panic!(
                "NodeParseRule::simple expects a simple tag name, not a selector: {}",
                tag
            );
        }

        Self {
            tag,
            priority: 50,
            matches: |_| true,
            parse,
        }
    }
}

pub struct StyleParseRule {
    pub tag: Option<&'static str>,
    pub style_key: Option<&'static str>,
    pub data_attr: Option<&'static str>,
    pub priority: u8,
    pub parse: fn(&ElementRef) -> Option<Style>,
}

impl StyleParseRule {
    pub fn from_tag(tag: &'static str, parse: fn(&ElementRef) -> Option<Style>) -> Self {
        Self {
            tag: Some(tag),
            style_key: None,
            data_attr: None,
            priority: 50,
            parse,
        }
    }

    pub fn from_style(style_key: &'static str, parse: fn(&ElementRef) -> Option<Style>) -> Self {
        Self {
            tag: None,
            style_key: Some(style_key),
            data_attr: None,
            priority: 30,
            parse,
        }
    }

    pub fn from_data(data_attr: &'static str, parse: fn(&ElementRef) -> Option<Style>) -> Self {
        Self {
            tag: None,
            style_key: None,
            data_attr: Some(data_attr),
            priority: 60,
            parse,
        }
    }
}

pub struct AnnotationParseRule {
    pub tag: Option<&'static str>,
    pub data_attr: Option<&'static str>,
    pub priority: u8,
    pub parse: fn(&ElementRef) -> Option<Annotation>,
    pub get_content: Option<fn(&ElementRef) -> String>,
}

impl AnnotationParseRule {
    pub fn from_tag(tag: &'static str, parse: fn(&ElementRef) -> Option<Annotation>) -> Self {
        Self {
            tag: Some(tag),
            data_attr: None,
            priority: 50,
            parse,
            get_content: None,
        }
    }

    pub fn from_tag_with_content(
        tag: &'static str,
        parse: fn(&ElementRef) -> Option<Annotation>,
        get_content: fn(&ElementRef) -> String,
    ) -> Self {
        Self {
            tag: Some(tag),
            data_attr: None,
            priority: 50,
            parse,
            get_content: Some(get_content),
        }
    }
}

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
    rules.extend(ArchivedNode::parse_rules());
    rules.extend(HorizontalRuleNode::parse_rules());
    rules.extend(HardBreakNode::parse_rules());
    rules.extend(PageBreakNode::parse_rules());
    rules.extend(CalloutNode::parse_rules());
    rules.extend(TableNode::parse_rules());
    rules.extend(TableRowNode::parse_rules());
    rules.extend(TableCellNode::parse_rules());
    rules.extend(FileNode::parse_rules());
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    rules
}

pub fn collect_style_parse_rules() -> Vec<StyleParseRule> {
    let mut rules = Vec::new();
    rules.extend(BoldStyle::parse_rules());
    rules.extend(ItalicStyle::parse_rules());
    rules.extend(UnderlineStyle::parse_rules());
    rules.extend(StrikethroughStyle::parse_rules());
    rules.extend(FontWeightStyle::parse_rules());
    rules.extend(FontSizeStyle::parse_rules());
    rules.extend(FontFamilyStyle::parse_rules());
    rules.extend(LetterSpacingStyle::parse_rules());
    rules.extend(TextColorStyle::parse_rules());
    rules.extend(BackgroundColorStyle::parse_rules());
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    rules
}

pub fn collect_annotation_parse_rules() -> Vec<AnnotationParseRule> {
    let mut rules = Vec::new();
    rules.extend(LinkAnnotation::parse_rules());
    rules.extend(RubyAnnotation::parse_rules());
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

pub struct ParsedAnnotations {
    pub annotations: Vec<Annotation>,
    pub custom_content: Option<String>,
}

pub fn parse_inline_styles(elem: &ElementRef, rules: &[StyleParseRule]) -> Vec<Style> {
    let value = elem.value();
    let tag = value.name();
    let has_style = value.attr("style").is_some();
    let mut styles = Vec::new();

    for rule in rules {
        let matches = if let Some(rule_tag) = rule.tag {
            rule_tag == tag
        } else if rule.style_key.is_some() {
            has_style
        } else if let Some(data_attr) = rule.data_attr {
            value.attr(data_attr).is_some()
        } else {
            false
        };

        if matches {
            if let Some(style) = (rule.parse)(elem) {
                styles.push(style);
            }
        }
    }

    styles
}

pub fn parse_inline_annotations(
    elem: &ElementRef,
    rules: &[AnnotationParseRule],
) -> ParsedAnnotations {
    let value = elem.value();
    let tag = value.name();
    let mut annotations = Vec::new();
    let mut custom_content = None;

    for rule in rules {
        let matches = if let Some(rule_tag) = rule.tag {
            rule_tag == tag
        } else if let Some(data_attr) = rule.data_attr {
            value.attr(data_attr).is_some()
        } else {
            false
        };

        if matches {
            if let Some(annotation) = (rule.parse)(elem) {
                annotations.push(annotation);
                if let Some(get_content) = rule.get_content {
                    custom_content = Some(get_content(elem));
                }
            }
        }
    }

    ParsedAnnotations {
        annotations,
        custom_content,
    }
}
