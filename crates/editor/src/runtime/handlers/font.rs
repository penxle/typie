use crate::model::{Annotation, Node, NodeId, Style};
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

        if let Node::Text(text_node) = node_ref.node() {
            let defaults = self.doc().default_attrs();
            let overrides = node_ref
                .parent()
                .map(|p| p.node().style_overrides())
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

    pub(crate) fn handle_fonts_loaded(&mut self, family: String, weight: u16) -> Vec<Effect> {
        if let Some(nodes) = self.missing_font_nodes.remove(&(family.clone(), weight)) {
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
        Vec::new()
    }
}
