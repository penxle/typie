use proc_macro2::Ident;
use syn::braced;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Result, Token, bracketed, parenthesized, token};

pub struct DocTree {
    pub root: NodeDef,
}

pub struct NodeDef {
    pub binding: Option<Ident>,
    pub node_type: Ident,
    pub params: Vec<FieldValue>,
    pub content: NodeContent,
    pub modifiers: Option<Vec<DecorationDef>>,
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
        let root = parse_node_def(input)?;
        if root.node_type != "root" {
            return Err(syn::Error::new(root.node_type.span(), "expected `root`"));
        }
        Ok(DocTree { root })
    }
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
        let modifiers = if input.peek(token::Bracket) {
            Some(parse_modifier_list(input)?)
        } else {
            None
        };
        Ok(NodeDef {
            binding,
            node_type,
            params: vec![],
            content: NodeContent::Text(text),
            modifiers,
        })
    } else {
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
        })
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
