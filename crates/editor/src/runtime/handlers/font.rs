use crate::model::{Annotation, Node, NodeId, Style};
use crate::runtime::{Effect, Runtime};
use loro::Frontiers;
use rustc_hash::FxHashSet;

impl Runtime {
    pub(crate) fn collect_doc_fonts(&self) -> Vec<(String, u16, FxHashSet<u32>)> {
        let mut fonts: rustc_hash::FxHashMap<(String, u16), FxHashSet<u32>> =
            rustc_hash::FxHashMap::default();

        self.collect_from_node(NodeId::ROOT, &mut fonts);

        fonts
            .into_iter()
            .map(|((family, weight), cps)| (family, weight, cps))
            .collect()
    }

    pub(crate) fn collect_doc_fonts_from_changed_text_diff(
        &self,
        old_state_frontiers: &Frontiers,
        new_state_frontiers: &Frontiers,
    ) -> Option<Vec<(String, u16, FxHashSet<u32>)>> {
        let diff = self
            .state
            .doc
            .loro_doc()
            .diff(old_state_frontiers, new_state_frontiers)
            .ok()?;

        let mut changed_text_nodes = FxHashSet::default();
        for (container_id, container_diff) in diff.iter() {
            if !matches!(container_diff, loro::event::Diff::Text(_)) {
                return None;
            }

            let path = self
                .state
                .doc
                .loro_doc()
                .get_path_to_container(container_id)?;
            let node_id = Self::extract_text_node_id_from_container_path(&path)?;
            changed_text_nodes.insert(node_id);
        }

        let mut fonts: rustc_hash::FxHashMap<(String, u16), FxHashSet<u32>> =
            rustc_hash::FxHashMap::default();
        for node_id in changed_text_nodes {
            self.collect_from_node(node_id, &mut fonts);
        }

        Some(
            fonts
                .into_iter()
                .map(|((family, weight), cps)| (family, weight, cps))
                .collect(),
        )
    }

    fn extract_text_node_id_from_container_path(
        path: &[(loro::ContainerID, loro::Index)],
    ) -> Option<NodeId> {
        const NODES_KEY: &str = "nodes";
        const TEXT_KEY: &str = "text";

        let mut inside_nodes_map = false;
        let mut node_id: Option<NodeId> = None;

        for (_, index) in path {
            let key = match index {
                loro::Index::Key(key) => key.to_string(),
                _ => continue,
            };

            if key == NODES_KEY {
                inside_nodes_map = true;
                node_id = None;
                continue;
            }

            if inside_nodes_map && node_id.is_none() {
                if let Some(parsed) = NodeId::from_string(&key) {
                    node_id = Some(parsed);
                    continue;
                }
                inside_nodes_map = false;
                continue;
            }

            if key == TEXT_KEY {
                return node_id;
            }
        }

        None
    }

    fn collect_from_node(
        &self,
        node_id: NodeId,
        fonts: &mut rustc_hash::FxHashMap<(String, u16), FxHashSet<u32>>,
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
