use crate::html::parse::shorthand::expand_shorthands;
use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, parse_important,
};
use scraper::Html;
use scraper::selector::{Parser as ScraperParser, Simple};
use selectors::SelectorList;
use selectors::matching::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches, matches_selector,
};
use selectors::parser::{ParseRelative, Selector as SelectorsSelector};

#[derive(Clone, Debug)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

struct Rule {
    selector: SelectorsSelector<Simple>,
    declarations: Vec<Declaration>,
    source_order: u32,
}

#[derive(Default)]
pub struct ComputedStylesheet {
    rules: Vec<Rule>,
}

impl ComputedStylesheet {
    pub fn from_html(doc: &Html) -> Self {
        let style_sel = scraper::Selector::parse("style").expect("style selector");
        let mut rules: Vec<Rule> = vec![];
        let mut order: u32 = 0;
        for style_elem in doc.select(&style_sel) {
            let text: String = style_elem.text().collect();
            for (sel_str, mut decls) in parse_stylesheet_text(&text) {
                expand_shorthands(&mut decls);

                let mut input = ParserInput::new(&sel_str);
                let mut parser = Parser::new(&mut input);
                let Ok(list) =
                    SelectorList::<Simple>::parse(&ScraperParser, &mut parser, ParseRelative::No)
                else {
                    continue;
                };
                for selector in list.slice() {
                    rules.push(Rule {
                        selector: selector.clone(),
                        declarations: decls.clone(),
                        source_order: order,
                    });
                    order += 1;
                }
            }
        }
        Self { rules }
    }

    pub fn matched_for(&self, elem: &scraper::ElementRef<'_>) -> Vec<Declaration> {
        let mut matched: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| matches_one(&r.selector, elem))
            .collect();
        matched.sort_by(|a, b| {
            b.selector
                .specificity()
                .cmp(&a.selector.specificity())
                .then_with(|| b.source_order.cmp(&a.source_order))
        });
        let mut out: Vec<Declaration> = vec![];
        for rule in matched {
            for decl in &rule.declarations {
                if decl.important {
                    continue;
                }
                if !out.iter().any(|o| o.property == decl.property) {
                    out.push(decl.clone());
                }
            }
        }
        out
    }
}

fn matches_one(sel: &SelectorsSelector<Simple>, elem: &scraper::ElementRef<'_>) -> bool {
    let mut caches = SelectorCaches::default();
    let mut ctx = MatchingContext::<Simple>::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    matches_selector(sel, 0, None, elem, &mut ctx)
}

pub fn parse_inline_style(style: &str) -> Vec<Declaration> {
    let mut input = ParserInput::new(style);
    let mut parser = Parser::new(&mut input);
    let mut decl_parser = DeclParser;
    let iter: RuleBodyParser<'_, '_, '_, _, Declaration, ()> =
        RuleBodyParser::new(&mut parser, &mut decl_parser);
    let mut out: Vec<Declaration> = vec![];
    for d in iter.flatten() {
        if d.important {
            continue;
        }
        if let Some(existing) = out
            .iter_mut()
            .find(|x: &&mut Declaration| x.property == d.property)
        {
            *existing = d;
        } else {
            out.push(d);
        }
    }
    expand_shorthands(&mut out);
    out
}

fn parse_stylesheet_text(text: &str) -> Vec<(String, Vec<Declaration>)> {
    let mut input = ParserInput::new(text);
    let mut parser = Parser::new(&mut input);
    let mut rp = RulesetParser;
    let iter = StyleSheetParser::new(&mut parser, &mut rp);
    let mut out = vec![];
    for rule in iter.flatten() {
        out.push(rule);
    }
    out
}

struct RulesetParser;

impl<'i> QualifiedRuleParser<'i> for RulesetParser {
    type Prelude = String;
    type QualifiedRule = (String, Vec<Declaration>);
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let start = input.position();
        while input.next_including_whitespace_and_comments().is_ok() {}
        Ok(input.slice_from(start).trim().to_string())
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut decl_parser = DeclParser;
        let iter: RuleBodyParser<'_, '_, '_, _, Declaration, ()> =
            RuleBodyParser::new(input, &mut decl_parser);
        let mut decls: Vec<Declaration> = vec![];
        for d in iter.flatten() {
            if let Some(existing) = decls
                .iter_mut()
                .find(|x: &&mut Declaration| x.property == d.property)
            {
                *existing = d;
            } else {
                decls.push(d);
            }
        }
        Ok((prelude, decls))
    }
}

impl<'i> AtRuleParser<'i> for RulesetParser {
    type Prelude = ();
    type AtRule = (String, Vec<Declaration>);
    type Error = ();
}

struct DeclParser;

impl<'i> DeclarationParser<'i> for DeclParser {
    type Declaration = Declaration;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _decl_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        let value_start = input.position();
        let mut important_at: Option<cssparser::SourcePosition> = None;
        loop {
            let before = input.position();
            if input.try_parse(parse_important).is_ok() {
                important_at = Some(before);
                break;
            }
            if input.next_including_whitespace_and_comments().is_err() {
                break;
            }
        }

        let raw = match important_at {
            Some(end) => input.slice(value_start..end).trim().to_string(),
            None => input.slice_from(value_start).trim().to_string(),
        };

        Ok(Declaration {
            property: name.to_string().to_lowercase(),
            value: raw,
            important: important_at.is_some(),
        })
    }
}

impl<'i> AtRuleParser<'i> for DeclParser {
    type Prelude = ();
    type AtRule = Declaration;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for DeclParser {
    type Prelude = ();
    type QualifiedRule = Declaration;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Declaration, ()> for DeclParser {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Selector;

    #[test]
    fn tag_selector() {
        let html = r#"<style>p { color: red; }</style><p>x</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        let d = s.matched_for(&p);
        assert!(d.iter().any(|x| x.property == "color" && x.value == "red"));
    }

    #[test]
    fn class_selector() {
        let html = r#"<style>.foo { color: red; }</style><p class="foo">x</p><p>y</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p_sel = Selector::parse("p").unwrap();
        let mut it = doc.select(&p_sel);
        let with_class = it.next().unwrap();
        let without = it.next().unwrap();
        assert!(
            s.matched_for(&with_class)
                .iter()
                .any(|x| x.property == "color")
        );
        assert!(s.matched_for(&without).is_empty());
    }

    #[test]
    fn id_selector() {
        let html = r#"<style>#x { color: red; }</style><p id="x">a</p><p>b</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p_sel = Selector::parse("p").unwrap();
        let mut it = doc.select(&p_sel);
        let with_id = it.next().unwrap();
        let without = it.next().unwrap();
        assert!(
            s.matched_for(&with_id)
                .iter()
                .any(|x| x.property == "color")
        );
        assert!(s.matched_for(&without).is_empty());
    }

    #[test]
    fn compound_selector() {
        let html = r#"<style>p.c { color: red; }</style><p>plain</p><p class="c">classed</p><div class="c">div</div>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p_sel = Selector::parse("p").unwrap();
        let mut p_it = doc.select(&p_sel);
        let p_plain = p_it.next().unwrap();
        let p_classed = p_it.next().unwrap();
        let div_sel = Selector::parse("div").unwrap();
        let div_classed = doc.select(&div_sel).next().unwrap();
        assert!(
            s.matched_for(&p_classed)
                .iter()
                .any(|x| x.property == "color")
        );
        assert!(s.matched_for(&p_plain).is_empty());
        assert!(s.matched_for(&div_classed).is_empty());
    }

    #[test]
    fn descendant_combinator() {
        let html = r#"<style>div p { color: red; }</style><div><p>nested</p></div><p>top</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p_sel = Selector::parse("p").unwrap();
        let mut it = doc.select(&p_sel);
        let nested = it.next().unwrap();
        let top = it.next().unwrap();
        assert!(s.matched_for(&nested).iter().any(|x| x.property == "color"));
        assert!(s.matched_for(&top).is_empty());
    }

    #[test]
    fn important_excluded() {
        let html = r#"<style>p { color: red !important; }</style><p>x</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        let d = s.matched_for(&p);
        assert!(
            d.iter().find(|x| x.property == "color").is_none(),
            "!important declaration should be skipped"
        );
    }

    #[test]
    fn specificity_class_beats_tag() {
        let html = r#"<style>p { color: blue; } .c { color: red; }</style><p class="c">x</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        let d = s.matched_for(&p);
        assert_eq!(
            d.iter().find(|x| x.property == "color").unwrap().value,
            "red"
        );
    }

    #[test]
    fn comma_group_expanded() {
        let html = r#"<style>h1, p { color: red; }</style><h1>a</h1><p>b</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let h1 = doc.select(&Selector::parse("h1").unwrap()).next().unwrap();
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        assert!(s.matched_for(&h1).iter().any(|x| x.property == "color"));
        assert!(s.matched_for(&p).iter().any(|x| x.property == "color"));
    }

    #[test]
    fn child_combinator() {
        let html = r#"<style>div > p { color: red; }</style><div><p>a</p></div><div><span><p>b</p></span></div>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p_sel = Selector::parse("p").unwrap();
        let mut it = doc.select(&p_sel);
        let direct = it.next().unwrap();
        let indirect = it.next().unwrap();
        assert!(s.matched_for(&direct).iter().any(|x| x.property == "color"));
        assert!(s.matched_for(&indirect).is_empty());
    }

    #[test]
    fn at_rule_skipped() {
        let html = r#"<style>@media (max-width:100px) { p { color: red; } } p { color: blue; }</style><p>x</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        assert_eq!(
            s.matched_for(&p)
                .iter()
                .find(|x| x.property == "color")
                .unwrap()
                .value,
            "blue"
        );
    }

    #[test]
    fn inline_parse_drops_important() {
        let d = parse_inline_style("color: red !important; font-size: 16px");
        assert!(d.iter().find(|x| x.property == "color").is_none());
        assert!(d.iter().any(|x| x.property == "font-size"));
    }

    #[test]
    fn important_quoted_string_false_positive_avoided() {
        let html = r#"<style>p { font-family: "foo !important", serif; }</style><p>x</p>"#;
        let doc = Html::parse_fragment(html);
        let s = ComputedStylesheet::from_html(&doc);
        let p = doc.select(&Selector::parse("p").unwrap()).next().unwrap();
        let d = s.matched_for(&p);
        let ff = d
            .iter()
            .find(|x| x.property == "font-family")
            .expect("font-family present");
        assert!(
            ff.value.contains("foo !important"),
            "quoted !important should remain in value, got: {:?}",
            ff.value
        );
    }
}
