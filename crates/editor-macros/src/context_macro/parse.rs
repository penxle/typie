use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token};

pub enum ContextAst {
    Any,
    SelfRef,
    GlobStar,
    Node(Ident),
    Child(Box<ContextAst>, Box<ContextAst>),
    AnyOf(Vec<ContextAst>),
    Not(Box<ContextAst>),
}

impl Parse for ContextAst {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_or(input)
    }
}

fn parse_or(input: ParseStream) -> Result<ContextAst> {
    let left = parse_child(input)?;
    let mut alts = vec![];
    while input.peek(Token![|]) {
        input.parse::<Token![|]>()?;
        alts.push(parse_child(input)?);
    }
    if alts.is_empty() {
        Ok(left)
    } else {
        alts.insert(0, left);
        Ok(ContextAst::AnyOf(alts))
    }
}

fn parse_child(input: ParseStream) -> Result<ContextAst> {
    let mut left = parse_unary(input)?;
    while input.peek(Token![>]) {
        input.parse::<Token![>]>()?;
        let right = parse_unary(input)?;
        left = ContextAst::Child(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_unary(input: ParseStream) -> Result<ContextAst> {
    if input.peek(Token![!]) {
        input.parse::<Token![!]>()?;
        let inner = parse_child(input)?;
        Ok(ContextAst::Not(Box::new(inner)))
    } else {
        parse_atom(input)
    }
}

fn parse_atom(input: ParseStream) -> Result<ContextAst> {
    if input.peek(Token![&]) {
        input.parse::<Token![&]>()?;
        Ok(ContextAst::SelfRef)
    } else if input.peek(Token![*]) {
        input.parse::<Token![*]>()?;
        input.parse::<Token![*]>()?;
        Ok(ContextAst::GlobStar)
    } else if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        content.parse()
    } else {
        let ident: Ident = input.parse()?;
        if ident == "Any" {
            Ok(ContextAst::Any)
        } else {
            Ok(ContextAst::Node(ident))
        }
    }
}
