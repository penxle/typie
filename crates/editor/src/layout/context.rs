use crate::layout::{Layout, LayoutCache, LayoutNode};
use crate::model::{Decorations, DocumentSettings, NodeRef};
use crate::runtime::ViewStates;
use crate::types::BoxConstraints;
use std::cell::RefCell;
use std::rc::Rc;

pub struct LayoutContext<'a> {
    pub node: &'a NodeRef<'a>,
    pub settings: &'a DocumentSettings,
    pub decorations: &'a Decorations,
    pub scale_factor: f64,
    pub view_states: &'a ViewStates,
    cache: &'a RefCell<LayoutCache>,
}

impl<'a> LayoutContext<'a> {
    pub fn new(
        node: &'a NodeRef<'a>,
        settings: &'a DocumentSettings,
        decorations: &'a Decorations,
        scale_factor: f64,
        view_states: &'a ViewStates,
        cache: &'a RefCell<LayoutCache>,
    ) -> Self {
        Self {
            node,
            settings,
            decorations,
            scale_factor,
            view_states,
            cache,
        }
    }

    pub fn with_node(&self, node: &'a NodeRef<'a>) -> Self {
        Self {
            node,
            settings: self.settings,
            decorations: self.decorations,
            scale_factor: self.scale_factor,
            view_states: self.view_states,
            cache: self.cache,
        }
    }

    pub fn layout(&self, child: &'a NodeRef<'a>, constraints: BoxConstraints) -> Rc<LayoutNode> {
        let node_id = child.node_id();

        if let Some(cached) = self.cache.borrow().get(node_id) {
            return cached;
        }

        let child_ctx = self.with_node(child);
        let layout = child.node().layout(&child_ctx, constraints);
        let rc = Rc::new(layout);

        self.cache.borrow_mut().insert(node_id, Rc::clone(&rc));
        rc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::elements::LineElement;
    use crate::layout::{Element, Layout};
    use crate::model::{Decorations, NodeId};
    use crate::types::BoxConstraints;
    use std::cell::RefCell;

    fn first_line(layout: &LayoutNode) -> Option<&LineElement> {
        if let Some(Element::Line(line)) = layout.element.as_ref() {
            return Some(line);
        }

        layout
            .children
            .as_ref()
            .and_then(|children| children.iter().find_map(|child| first_line(&child.node)))
    }

    #[test]
    fn layout_cache_accounts_for_preedit_in_list_item_paragraph() {
        let mut p = id!();
        let state = state! {
            doc {
                bullet_list {
                    list_item {
                        @p paragraph {
                            text { "hello" }
                        }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let root = doc.node(crate::model::NodeId::ROOT).unwrap();
        let settings = doc.settings();
        let constraints = BoxConstraints::new(0.0, 400.0, 0.0, f32::INFINITY);
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();

        let decorations = Decorations::default();
        let ctx = LayoutContext::new(&root, &settings, &decorations, 1.0, &view_states, &cache);
        let layout_without_preedit = root.node().layout(&ctx, constraints);
        let line_without_preedit =
            first_line(&layout_without_preedit).expect("라인을 찾을 수 있어야 함");
        assert!(
            line_without_preedit.preedit.is_none(),
            "초기 레이아웃에는 preedit가 없어야 함"
        );

        let decorations_with_preedit = Decorations {
            preedit: Some(crate::model::PreeditDecor {
                node_id: p,
                offset: 1,
                text: "가".into(),
                marks: None,
            }),
            pending_marks: None,
        };

        let p_node = doc.node(p).unwrap();
        let ancestors: Vec<NodeId> = p_node.ancestors().map(|n| n.node_id()).collect();
        cache
            .borrow_mut()
            .invalidate_with_ancestors(p, ancestors.into_iter());

        let ctx_with_preedit = LayoutContext::new(
            &root,
            &settings,
            &decorations_with_preedit,
            1.0,
            &view_states,
            &cache,
        );
        let layout_with_preedit = root.node().layout(&ctx_with_preedit, constraints);
        let line_with_preedit =
            first_line(&layout_with_preedit).expect("preedit 적용 후에도 라인이 필요함");

        let preedit = line_with_preedit
            .preedit
            .as_ref()
            .expect("리스트 아이템 내 문단에도 preedit가 반영되어야 함");
        assert_eq!(preedit.node_id, p, "preedit 대상 노드가 일치해야 함");
        assert_eq!(preedit.offset, 1, "preedit 오프셋이 유지되어야 함");
    }
}
