use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token, token};

pub enum ContentAst {
    Empty,
    Single(Ident),
    Seq(Vec<ContentAst>),
    Choice(Vec<Ident>),
    ZeroOrMore(Box<ContentAst>),
    OneOrMore(Box<ContentAst>),
    Optional(Box<ContentAst>),
}

impl Parse for ContentAst {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(ContentAst::Empty);
        }
        let first = parse_item(input)?;
        if input.peek(Token![,]) {
            let mut items = vec![first];
            while input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
                items.push(parse_item(input)?);
            }
            Ok(ContentAst::Seq(items))
        } else {
            Ok(first)
        }
    }
}

fn parse_item(input: ParseStream) -> Result<ContentAst> {
    let base = parse_base(input)?;
    if input.peek(Token![+]) {
        input.parse::<Token![+]>()?;
        Ok(ContentAst::OneOrMore(Box::new(base)))
    } else if input.peek(Token![*]) {
        input.parse::<Token![*]>()?;
        Ok(ContentAst::ZeroOrMore(Box::new(base)))
    } else if input.peek(Token![?]) {
        input.parse::<Token![?]>()?;
        Ok(ContentAst::Optional(Box::new(base)))
    } else {
        Ok(base)
    }
}

fn parse_base(input: ParseStream) -> Result<ContentAst> {
    if input.peek(token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let first: Ident = content.parse()?;
        if content.peek(Token![|]) {
            let mut choices = vec![first];
            while content.peek(Token![|]) {
                content.parse::<Token![|]>()?;
                choices.push(content.parse()?);
            }
            Ok(ContentAst::Choice(choices))
        } else {
            Ok(ContentAst::Single(first))
        }
    } else {
        let ident: Ident = input.parse()?;
        Ok(ContentAst::Single(ident))
    }
}
