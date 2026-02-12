pub struct HtmlBuilder {
    buf: String,
}

impl HtmlBuilder {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    pub fn into_string(self) -> String {
        self.buf
    }

    pub fn text(&mut self, s: &str) {
        self.buf.push_str(&escape(s));
    }

    pub fn open(&mut self, tag: &str) -> TagBuilder<'_> {
        TagBuilder {
            b: self,
            tag: tag.into(),
            attrs: Vec::new(),
        }
    }
}

pub struct TagBuilder<'a> {
    b: &'a mut HtmlBuilder,
    tag: String,
    attrs: Vec<(String, String)>,
}

impl<'a> TagBuilder<'a> {
    pub fn attr(mut self, name: &str, value: impl ToString) -> Self {
        self.attrs.push((name.into(), value.to_string()));
        self
    }

    pub fn data(self, name: &str, value: impl ToString) -> Self {
        self.attr(&format!("data-{}", name), value)
    }

    pub fn void(self) {
        self.b.buf.push('<');
        self.b.buf.push_str(&self.tag);
        for (k, v) in &self.attrs {
            self.b.buf.push(' ');
            self.b.buf.push_str(k);
            self.b.buf.push_str("=\"");
            self.b.buf.push_str(&escape_attr(v));
            self.b.buf.push('"');
        }
        self.b.buf.push('>');
    }

    pub fn children(self, f: impl FnOnce(&mut HtmlBuilder)) {
        self.b.buf.push('<');
        self.b.buf.push_str(&self.tag);
        for (k, v) in &self.attrs {
            self.b.buf.push(' ');
            self.b.buf.push_str(k);
            self.b.buf.push_str("=\"");
            self.b.buf.push_str(&escape_attr(v));
            self.b.buf.push('"');
        }
        self.b.buf.push('>');
        f(self.b);
        self.b.buf.push_str("</");
        self.b.buf.push_str(&self.tag);
        self.b.buf.push('>');
    }
}

fn escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}

fn escape_attr(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            _ => result.push(c),
        }
    }
    result
}

#[derive(Clone, Debug)]
pub enum DomSpec {
    Hole,
    Text(String),
    Element {
        tag: &'static str,
        attrs: Vec<(&'static str, String)>,
        children: Vec<DomSpec>,
    },
    Void {
        tag: &'static str,
        attrs: Vec<(&'static str, String)>,
    },
    Fragment(Vec<DomSpec>),
}

impl DomSpec {
    pub fn el(tag: &'static str) -> ElementBuilder {
        ElementBuilder::new(tag)
    }

    pub fn substitute_hole(self, replacement: DomSpec) -> DomSpec {
        match self {
            DomSpec::Hole => replacement,
            DomSpec::Element {
                tag,
                attrs,
                children,
            } => {
                let new_children = children
                    .into_iter()
                    .map(|c| c.substitute_hole(replacement.clone()))
                    .collect();
                DomSpec::Element {
                    tag,
                    attrs,
                    children: new_children,
                }
            }
            DomSpec::Fragment(specs) => {
                let new_specs = specs
                    .into_iter()
                    .map(|s| s.substitute_hole(replacement.clone()))
                    .collect();
                DomSpec::Fragment(new_specs)
            }
            other => other,
        }
    }

    pub fn wrap_with_styles(text: String, styles: Vec<DomSpec>) -> DomSpec {
        let mut result = DomSpec::Text(text);
        for style_spec in styles.into_iter().rev() {
            result = style_spec.substitute_hole(result);
        }
        result
    }
}

pub struct ElementBuilder {
    tag: &'static str,
    attrs: Vec<(&'static str, String)>,
    children: Vec<DomSpec>,
}

impl ElementBuilder {
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.attrs.push((key, value.into()));
        self
    }

    pub fn style(self, value: impl Into<String>) -> Self {
        self.attr("style", value)
    }

    pub fn data(self, key: &'static str, value: impl Into<String>) -> Self {
        let data_key = Box::leak(format!("data-{}", key).into_boxed_str());
        self.attr(data_key, value)
    }

    pub fn child(mut self, node: DomSpec) -> Self {
        self.children.push(node);
        self
    }

    pub fn hole(self) -> DomSpec {
        self.child(DomSpec::Hole).build()
    }

    pub fn text(self, s: impl Into<String>) -> DomSpec {
        self.child(DomSpec::Text(s.into())).build()
    }

    pub fn void(self) -> DomSpec {
        DomSpec::Void {
            tag: self.tag,
            attrs: self.attrs,
        }
    }

    pub fn empty(self) -> DomSpec {
        self.build()
    }

    pub fn build(self) -> DomSpec {
        DomSpec::Element {
            tag: self.tag,
            attrs: self.attrs,
            children: self.children,
        }
    }
}
