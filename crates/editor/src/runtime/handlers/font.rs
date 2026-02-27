use crate::model::{Annotation, Attr, Node, NodeId, Style};
use crate::runtime::{Effect, Runtime};
use rustc_hash::{FxHashMap, FxHashSet};

impl Runtime {
    pub(crate) fn collect_doc_fonts_from_nodes<I>(
        &self,
        node_ids: I,
    ) -> Vec<(String, u16, FxHashSet<u32>)>
    where
        I: IntoIterator<Item = NodeId>,
    {
        let mut fonts: FxHashMap<(String, u16), FxHashSet<u32>> = FxHashMap::default();
        for node_id in node_ids {
            self.collect_from_node(node_id, &mut fonts);
        }

        fonts
            .into_iter()
            .map(|((family, weight), cps)| (family, weight, cps))
            .collect()
    }

    fn collect_from_node(
        &self,
        node_id: NodeId,
        fonts: &mut FxHashMap<(String, u16), FxHashSet<u32>>,
    ) {
        let Some(node_ref) = self.doc().node(node_id) else {
            return;
        };

        if let Some(cascade_attrs) = node_ref.cascade_attrs() {
            let mut family = None;
            let mut weight = None;
            for attr in &cascade_attrs {
                match attr {
                    Attr::Style(Style::FontFamily(f)) => family = Some(f.family.clone()),
                    Attr::Style(Style::FontWeight(w)) => weight = Some(w.weight),
                    _ => {}
                }
            }
            if let Some(family) = family {
                let weight = weight.unwrap_or_else(|| self.doc().default_attrs().font_weight());
                fonts
                    .entry((family, weight))
                    .or_default()
                    .insert('\u{200B}' as u32);
            }
        }

        if let Some(Node::Text(text_node)) = node_ref.node() {
            let defaults = self.doc().default_attrs();
            let overrides = node_ref
                .parent()
                .and_then(|p| p.node().map(|n| n.style_overrides()))
                .unwrap_or_default();
            let ruby_family = defaults.font_family().to_string();
            let ruby_weight = defaults.font_weight();

            for seg in text_node.text.get_segments() {
                let family = seg
                    .styles
                    .iter()
                    .find_map(|s| match s {
                        Style::FontFamily(f) => Some(f.family.clone()),
                        _ => None,
                    })
                    .or_else(|| {
                        overrides.iter().find_map(|s| match s {
                            Style::FontFamily(f) => Some(f.family.clone()),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| defaults.font_family().to_string());
                let weight = seg
                    .styles
                    .iter()
                    .find_map(|s| match s {
                        Style::FontWeight(w) => Some(w.weight),
                        _ => None,
                    })
                    .or_else(|| {
                        overrides.iter().find_map(|s| match s {
                            Style::FontWeight(w) => Some(w.weight),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| defaults.font_weight());

                let font_cps = fonts.entry((family, weight)).or_default();
                for ch in seg.text.chars() {
                    font_cps.insert(ch as u32);
                }

                for annotation in &seg.annotations {
                    if let Annotation::Ruby(ruby_ann) = annotation {
                        let ruby_font_cps =
                            fonts.entry((ruby_family.clone(), ruby_weight)).or_default();
                        for ch in ruby_ann.text.chars() {
                            ruby_font_cps.insert(ch as u32);
                        }
                    }
                }
            }
        }

        for child in node_ref.children() {
            self.collect_from_node(child.node_id(), fonts);
        }
    }

    #[cfg(test)]
    fn collect_doc_fonts_as_map<I>(&self, node_ids: I) -> FxHashMap<(String, u16), FxHashSet<u32>>
    where
        I: IntoIterator<Item = NodeId>,
    {
        let mut fonts: FxHashMap<(String, u16), FxHashSet<u32>> = FxHashMap::default();
        for node_id in node_ids {
            self.collect_from_node(node_id, &mut fonts);
        }
        fonts
    }

    pub(crate) fn handle_fonts_loaded(
        &mut self,
        family: String,
        weight: u16,
        codepoints: Vec<u32>,
    ) -> Vec<Effect> {
        let key = (family.clone(), weight);

        // Remove loaded codepoints from the embedded pending set
        if let Some((_, pending_cps)) = self.missing_font_nodes.get_mut(&key) {
            for cp in &codepoints {
                pending_cps.remove(cp);
            }
        }

        let all_loaded = self
            .missing_font_nodes
            .get(&key)
            .map_or(true, |(_, cps)| cps.is_empty());

        if all_loaded {
            // All codepoints loaded — consume missing_font_nodes
            if let Some((nodes, _)) = self.missing_font_nodes.remove(&key) {
                if nodes.contains(&NodeId::ROOT) {
                    return vec![Effect::FullLayoutInvalidation, Effect::LayoutChanged];
                }
                if !nodes.is_empty() {
                    return nodes
                        .into_iter()
                        .map(|node_id| Effect::NodeChanged { node_id })
                        .collect();
                }
            }
        } else {
            // More codepoints pending — return effects but keep missing_font_nodes
            if let Some((nodes, _)) = self.missing_font_nodes.get(&key) {
                if nodes.contains(&NodeId::ROOT) {
                    return vec![Effect::FullLayoutInvalidation, Effect::LayoutChanged];
                }
                if !nodes.is_empty() {
                    return nodes
                        .iter()
                        .copied()
                        .map(|node_id| Effect::NodeChanged { node_id })
                        .collect();
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;

    const ZWSP: u32 = '\u{200B}' as u32;

    #[test]
    fn collect_fonts_includes_root_cascade_attrs_font() {
        let state = state! {
            doc {
                paragraph { text { "a" } }
            }
        };
        let rt = crate::runtime::Runtime::new(800.0, 1.0, state);
        let fonts = rt.collect_doc_fonts_as_map([NodeId::ROOT]);

        let default_family = rt.doc().default_attrs().font_family().to_string();
        let default_weight = rt.doc().default_attrs().font_weight();
        let key = (default_family, default_weight);

        assert!(
            fonts.contains_key(&key),
            "ROOT cascade_attrs font must be collected, got keys: {:?}",
            fonts.keys().collect::<Vec<_>>()
        );
        assert!(
            fonts[&key].contains(&ZWSP),
            "cascade_attrs font codepoints must include ZWSP"
        );
    }

    #[test]
    fn collect_fonts_includes_paragraph_cascade_attrs_font() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "CustomFont".to_string(),
                })]),
            )
            .unwrap();
        });
        let rt = crate::runtime::Runtime::new(800.0, 1.0, state);
        let fonts = rt.collect_doc_fonts_as_map([NodeId::ROOT]);

        assert!(
            fonts.keys().any(|(f, _)| f == "CustomFont"),
            "paragraph cascade_attrs font must be collected"
        );
    }

    #[test]
    fn collect_fonts_from_empty_paragraph_with_cascade_attrs() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "EmptyParaFont".to_string(),
                })]),
            )
            .unwrap();
        });
        let rt = crate::runtime::Runtime::new(800.0, 1.0, state);

        // Collect from the paragraph node directly (no text children)
        let fonts = rt.collect_doc_fonts_as_map([p]);

        assert!(
            fonts.keys().any(|(f, _)| f == "EmptyParaFont"),
            "empty paragraph's cascade_attrs font must be collected"
        );
    }

    #[test]
    fn collect_fonts_cascade_uses_default_weight_when_only_family_specified() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "WeightTest".to_string(),
                })]),
            )
            .unwrap();
        });
        let rt = crate::runtime::Runtime::new(800.0, 1.0, state);
        let default_weight = rt.doc().default_attrs().font_weight();
        let fonts = rt.collect_doc_fonts_as_map([p]);

        assert!(
            fonts.contains_key(&("WeightTest".to_string(), default_weight)),
            "cascade font without explicit weight must use default weight ({})",
            default_weight
        );
    }

    #[test]
    fn collect_fonts_cascade_respects_explicit_weight() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[
                    Style::FontFamily(FontFamilyStyle {
                        family: "BoldFont".to_string(),
                    }),
                    Style::FontWeight(FontWeightStyle { weight: 700 }),
                ]),
            )
            .unwrap();
        });
        let rt = crate::runtime::Runtime::new(800.0, 1.0, state);
        let fonts = rt.collect_doc_fonts_as_map([p]);

        assert!(
            fonts.contains_key(&("BoldFont".to_string(), 700)),
            "cascade font with explicit weight=700 must be collected with that weight"
        );
    }
}
