use proc_macro2::Ident;
use syn::braced;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Result, Token, bracketed, parenthesized, token};

pub struct DocTree {
    pub styles: Vec<StyleDef>,
    pub root: NodeDef,
}

pub struct StyleDef {
    pub id: Ident,
    pub name: Option<LitStr>,
    pub modifiers: Vec<DecorationDef>,
}

pub struct NodeDef {
    pub binding: Option<Ident>,
    pub node_type: Ident,
    pub params: Vec<FieldValue>,
    pub content: NodeContent,
    pub modifiers: Option<Vec<DecorationDef>>,
    pub style: Option<Ident>,
    pub marker: Option<MarkerDef>,
}

pub struct MarkerDef {
    pub style: Option<Ident>,
    pub modifiers: Vec<DecorationDef>,
}

pub enum NodeContent {
    Children(Vec<NodeDef>),
    Text(LitStr),
    Leaf,
}

pub struct DecorationDef {
    pub name: Ident,
    pub params: DecorationParams,
}

pub enum DecorationParams {
    None,
    Named(Vec<FieldValue>),
    Positional(Vec<Expr>),
}

pub struct FieldValue {
    pub name: Ident,
    pub value: Expr,
}

impl Parse for DocTree {
    fn parse(input: ParseStream) -> Result<Self> {
        let styles = if peek_styles_kw(input) {
            parse_styles_block(input)?
        } else {
            Vec::new()
        };

        let root = parse_node_def(input)?;
        if root.node_type != "root" {
            return Err(syn::Error::new(root.node_type.span(), "expected `root`"));
        }

        let tree = DocTree { styles, root };
        validate_styles(&tree)?;
        Ok(tree)
    }
}

fn validate_styles(tree: &DocTree) -> Result<()> {
    let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();
    for s in &tree.styles {
        if !declared.insert(s.id.to_string()) {
            return Err(syn::Error::new(
                s.id.span(),
                format!("duplicate style `{}`", s.id),
            ));
        }
    }
    validate_node_styles(&tree.root, &declared)
}

fn validate_node_styles(
    node: &NodeDef,
    declared: &std::collections::HashSet<String>,
) -> Result<()> {
    if let Some(style) = &node.style
        && !declared.contains(&style.to_string())
    {
        return Err(syn::Error::new(
            style.span(),
            format!("unknown style `{}`", style),
        ));
    }
    if let Some(marker) = &node.marker
        && let Some(style) = &marker.style
        && !declared.contains(&style.to_string())
    {
        return Err(syn::Error::new(
            style.span(),
            format!("unknown style `{}`", style),
        ));
    }
    if let NodeContent::Children(children) = &node.content {
        for child in children {
            validate_node_styles(child, declared)?;
        }
    }
    Ok(())
}

fn peek_styles_kw(input: ParseStream) -> bool {
    let fork = input.fork();
    matches!(fork.parse::<Ident>(), Ok(id) if id == "styles") && fork.peek(token::Brace)
}

fn parse_styles_block(input: ParseStream) -> Result<Vec<StyleDef>> {
    let _kw: Ident = input.parse()?;
    let content;
    braced!(content in input);

    let mut styles = Vec::new();
    while !content.is_empty() {
        styles.push(parse_style_def(&content)?);
    }
    Ok(styles)
}

fn parse_style_def(input: ParseStream) -> Result<StyleDef> {
    let id: Ident = input.parse()?;
    input.parse::<Token![:]>()?;

    let name = if input.peek(LitStr) {
        Some(input.parse::<LitStr>()?)
    } else {
        None
    };

    let modifiers = if input.peek(token::Bracket) {
        parse_modifier_list(input)?
    } else {
        Vec::new()
    };

    if name.is_none() && modifiers.is_empty() {
        return Err(syn::Error::new(
            id.span(),
            "style definition must have a display name or modifiers",
        ));
    }

    Ok(StyleDef {
        id,
        name,
        modifiers,
    })
}

fn parse_node_list(input: ParseStream) -> Result<Vec<NodeDef>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        nodes.push(parse_node_def(input)?);
    }
    Ok(nodes)
}

fn parse_node_def(input: ParseStream) -> Result<NodeDef> {
    let (binding, node_type) = if input.peek2(Token![:]) && !input.peek2(Token![::]) {
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty: Ident = input.parse()?;
        (Some(name), ty)
    } else {
        let ty: Ident = input.parse()?;
        (None, ty)
    };

    let is_text = node_type == "text";

    if is_text {
        let text = parse_text_content(input)?;
        let style = parse_optional_style(input)?;
        let modifiers = if input.peek(token::Bracket) {
            Some(parse_modifier_list(input)?)
        } else {
            None
        };
        let marker = parse_optional_marker(input)?;
        Ok(NodeDef {
            binding,
            node_type,
            params: vec![],
            content: NodeContent::Text(text),
            modifiers,
            style,
            marker,
        })
    } else {
        let style = parse_optional_style(input)?;

        let params = if input.peek(token::Paren) {
            parse_field_values_in_parens(input)?
        } else {
            vec![]
        };

        let modifiers = if input.peek(token::Bracket) {
            Some(parse_modifier_list(input)?)
        } else {
            None
        };

        let marker = parse_optional_marker(input)?;

        let content = if input.peek(token::Brace) {
            let inner;
            braced!(inner in input);
            NodeContent::Children(parse_node_list(&inner)?)
        } else {
            NodeContent::Leaf
        };

        Ok(NodeDef {
            binding,
            node_type,
            params,
            content,
            modifiers,
            style,
            marker,
        })
    }
}

fn parse_optional_marker(input: ParseStream) -> Result<Option<MarkerDef>> {
    let is_marker = matches!(input.fork().parse::<Ident>(), Ok(id) if id == "marker")
        && input.peek2(token::Paren);
    if !is_marker {
        return Ok(None);
    }

    let kw: Ident = input.parse()?;
    let content;
    parenthesized!(content in input);

    let style = parse_optional_style(&content)?;
    let modifiers = if content.peek(token::Bracket) {
        parse_modifier_list(&content)?
    } else {
        Vec::new()
    };

    if style.is_none() && modifiers.is_empty() {
        return Err(syn::Error::new(
            kw.span(),
            "marker must have a style or modifiers",
        ));
    }

    Ok(Some(MarkerDef { style, modifiers }))
}

fn parse_optional_style(input: ParseStream) -> Result<Option<Ident>> {
    if input.peek(Token![@]) {
        input.parse::<Token![@]>()?;
        Ok(Some(input.parse::<Ident>()?))
    } else {
        Ok(None)
    }
}

fn parse_text_content(input: ParseStream) -> Result<LitStr> {
    let content;
    parenthesized!(content in input);
    content.parse()
}

pub(crate) fn parse_modifier_list(input: ParseStream) -> Result<Vec<DecorationDef>> {
    let content;
    bracketed!(content in input);

    let mut modifiers = Vec::new();

    loop {
        if content.is_empty() {
            break;
        }

        let name: Ident = Ident::parse_any(&content)?;

        let params = if content.peek(token::Paren) {
            parse_decoration_params(&content)?
        } else {
            DecorationParams::None
        };

        modifiers.push(DecorationDef { name, params });

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }

    Ok(modifiers)
}

pub(crate) fn parse_decoration_params(input: ParseStream) -> Result<DecorationParams> {
    let content;
    parenthesized!(content in input);

    if content.is_empty() {
        return Ok(DecorationParams::None);
    }

    let is_named =
        content.peek(syn::Ident) && content.peek2(Token![:]) && !content.peek2(Token![::]);

    if is_named {
        let mut fields = Vec::new();
        loop {
            if content.is_empty() {
                break;
            }
            let name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let value: Expr = content.parse()?;
            fields.push(FieldValue { name, value });
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(DecorationParams::Named(fields))
    } else {
        let mut exprs = Vec::new();
        loop {
            if content.is_empty() {
                break;
            }
            let value: Expr = content.parse()?;
            exprs.push(value);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(DecorationParams::Positional(exprs))
    }
}

fn parse_field_values_in_parens(input: ParseStream) -> Result<Vec<FieldValue>> {
    let content;
    parenthesized!(content in input);

    let mut fields = Vec::new();
    loop {
        if content.is_empty() {
            break;
        }

        let name: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let value: Expr = content.parse()?;

        fields.push(FieldValue { name, value });

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }

    Ok(fields)
}
