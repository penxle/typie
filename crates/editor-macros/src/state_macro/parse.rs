use proc_macro2::Ident;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{LitInt, Result, Token, braced, bracketed, parenthesized, token};

use crate::doc_macro::parse::{DecorationDef, DecorationParams, DocTree, parse_decoration_params};

pub struct StateInput {
    pub doc_tree: DocTree,
    pub selection: SelectionExpr,
    pub pending_modifiers: Vec<PendingModifierDef>,
}

pub enum PendingModifierDef {
    Set(DecorationDef),
    Unset(Ident),
}

pub enum SelectionExpr {
    Collapsed(PositionExpr),
    Range(PositionExpr, PositionExpr),
}

pub struct PositionExpr {
    pub node_ident: Ident,
    pub offset: LitInt,
    pub affinity: Option<AffinityKind>,
}

pub enum AffinityKind {
    Upstream,
    Downstream,
}

impl Parse for StateInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let doc_kw: Ident = input.parse()?;
        if doc_kw != "doc" {
            return Err(syn::Error::new(doc_kw.span(), "expected `doc`"));
        }

        let doc_content;
        braced!(doc_content in input);
        let doc_tree: DocTree = doc_content.parse()?;

        let sel_kw: Ident = input.parse()?;
        if sel_kw != "selection" {
            return Err(syn::Error::new(sel_kw.span(), "expected `selection`"));
        }

        input.parse::<Token![:]>()?;
        let selection = parse_selection(input)?;

        let pending_modifiers = if !input.is_empty() {
            let ps_kw: Ident = input.parse()?;
            if ps_kw != "pending_modifiers" {
                return Err(syn::Error::new(
                    ps_kw.span(),
                    "expected `pending_modifiers`",
                ));
            }
            input.parse::<Token![:]>()?;
            parse_pending_modifier_list(input)?
        } else {
            vec![]
        };

        Ok(StateInput {
            doc_tree,
            selection,
            pending_modifiers,
        })
    }
}

fn parse_selection(input: ParseStream) -> Result<SelectionExpr> {
    let anchor = parse_position(input)?;

    if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        let head = parse_position(input)?;
        Ok(SelectionExpr::Range(anchor, head))
    } else {
        Ok(SelectionExpr::Collapsed(anchor))
    }
}

fn parse_position(input: ParseStream) -> Result<PositionExpr> {
    let content;
    parenthesized!(content in input);

    let node_ident: Ident = content.parse()?;
    content.parse::<Token![,]>()?;
    let offset: LitInt = content.parse()?;

    let affinity = if content.peek(Token![,]) {
        content.parse::<Token![,]>()?;
        Some(parse_affinity(&content)?)
    } else {
        None
    };

    Ok(PositionExpr {
        node_ident,
        offset,
        affinity,
    })
}

fn parse_affinity(input: ParseStream) -> Result<AffinityKind> {
    if input.peek(Token![<]) {
        input.parse::<Token![<]>()?;
        Ok(AffinityKind::Upstream)
    } else if input.peek(Token![>]) {
        input.parse::<Token![>]>()?;
        Ok(AffinityKind::Downstream)
    } else {
        Err(input.error("expected `<` (Upstream) or `>` (Downstream)"))
    }
}

fn parse_pending_modifier_list(input: ParseStream) -> Result<Vec<PendingModifierDef>> {
    let content;
    bracketed!(content in input);

    let mut modifiers = Vec::new();

    loop {
        if content.is_empty() {
            break;
        }

        if content.peek(Token![!]) {
            content.parse::<Token![!]>()?;
            let name: Ident = Ident::parse_any(&content)?;
            modifiers.push(PendingModifierDef::Unset(name));
        } else {
            let name: Ident = Ident::parse_any(&content)?;
            let params = if content.peek(token::Paren) {
                parse_decoration_params(&content)?
            } else {
                DecorationParams::None
            };
            modifiers.push(PendingModifierDef::Set(DecorationDef { name, params }));
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }

    Ok(modifiers)
}
