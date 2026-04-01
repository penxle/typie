use crate::diagnostics::LayoutPassRecorder;
use crate::layout::{Layout, LayoutCache, LayoutNode};
use crate::model::{Attr, Decorations, DefaultAttrs, DocumentSettings, NodeRef, Style};
use crate::runtime::ViewStates;
use crate::types::BoxConstraints;
use std::cell::RefCell;
use std::rc::Rc;

pub struct LayoutContext<'a> {
    pub node: &'a NodeRef<'a>,
    pub settings: &'a DocumentSettings,
    pub default_attrs: &'a DefaultAttrs,
    pub decorations: &'a Decorations,
    pub view_states: &'a ViewStates,
    cache: &'a RefCell<LayoutCache>,
    trace: Option<&'a RefCell<LayoutPassRecorder>>,
}

impl<'a> LayoutContext<'a> {
    pub fn new(
        node: &'a NodeRef<'a>,
        settings: &'a DocumentSettings,
        default_attrs: &'a DefaultAttrs,
        decorations: &'a Decorations,
        view_states: &'a ViewStates,
        cache: &'a RefCell<LayoutCache>,
    ) -> Self {
        Self::new_with_trace(
            node,
            settings,
            default_attrs,
            decorations,
            view_states,
            cache,
            None,
        )
    }

    pub fn new_with_trace(
        node: &'a NodeRef<'a>,
        settings: &'a DocumentSettings,
        default_attrs: &'a DefaultAttrs,
        decorations: &'a Decorations,
        view_states: &'a ViewStates,
        cache: &'a RefCell<LayoutCache>,
        trace: Option<&'a RefCell<LayoutPassRecorder>>,
    ) -> Self {
        Self {
            node,
            settings,
            default_attrs,
            decorations,
            view_states,
            cache,
            trace,
        }
    }

    /// Resolve font defaults from style_overrides → cascade_attrs chain → root default_attrs.
    pub fn resolve_cascade_font(&self) -> (String, u16, u32) {
        let mut family: Option<String> = None;
        let mut weight: Option<u16> = None;
        let mut font_size: Option<u32> = None;

        // cascade_attrs: node → parent → ... → root (first occurrence wins)
        for ancestor in self.node.ancestors() {
            if let Some(cascade) = ancestor.cascade_attrs() {
                for attr in &cascade {
                    match attr {
                        Attr::Style(Style::FontFamily(f)) if family.is_none() => {
                            family = Some(f.family.clone());
                        }
                        Attr::Style(Style::FontWeight(w)) if weight.is_none() => {
                            weight = Some(w.weight);
                        }
                        Attr::Style(Style::FontSize(s)) if font_size.is_none() => {
                            font_size = Some(s.size);
                        }
                        _ => {}
                    }
                }
                if family.is_some() && weight.is_some() && font_size.is_some() {
                    break;
                }
            }
        }

        let mut family = family.unwrap_or_else(|| self.default_attrs.font_family().to_string());
        let mut weight = weight.unwrap_or_else(|| self.default_attrs.font_weight());
        let mut font_size = font_size.unwrap_or(1200);

        // style_overrides: highest priority (node-type-specific hardcoded overrides)
        let style_overrides = self
            .node
            .node()
            .map(|n| n.style_overrides())
            .unwrap_or_default();
        for style in style_overrides {
            match &style {
                Style::FontFamily(f) => family = f.family.clone(),
                Style::FontWeight(w) => weight = w.weight,
                Style::FontSize(s) => font_size = s.size,
                _ => {}
            }
        }

        (family, weight, font_size)
    }

    pub fn with_node(&self, node: &'a NodeRef<'a>) -> Self {
        Self {
            node,
            settings: self.settings,
            default_attrs: self.default_attrs,
            decorations: self.decorations,
            view_states: self.view_states,
            cache: self.cache,
            trace: self.trace,
        }
    }

    pub fn layout(&self, child: &'a NodeRef<'a>, constraints: BoxConstraints) -> Rc<LayoutNode> {
        let node_id = child.node_id();

        if let Some(cached) = self.cache.borrow().get(node_id) {
            return cached;
        }

        if let Some(trace) = self.trace {
            trace.borrow_mut().record_recomputed(node_id);
        }

        let prev = self.cache.borrow_mut().take_prev(node_id);

        let child_ctx = self.with_node(child);
        let Some(node) = child.node() else {
            // Undecodable node: return a minimal placeholder (1px height to avoid zero-height layout issues)
            let empty = Rc::new(LayoutNode {
                size: crate::types::Size::new(constraints.max_width, 1.0),
                element: None,
                children: None,
                page_break_policy: Default::default(),
                render_hints: Default::default(),
                scope_id: None,
            });
            self.cache.borrow_mut().insert(node_id, Rc::clone(&empty));
            return empty;
        };
        let layout = node.layout(&child_ctx, constraints);
        let rc = Rc::new(layout);

        let result = if let Some(prev_layout) = prev {
            if is_layout_equal(&rc, &prev_layout) {
                prev_layout
            } else {
                rc
            }
        } else {
            rc
        };

        self.cache.borrow_mut().insert(node_id, Rc::clone(&result));
        result
    }
}

fn is_layout_equal(new: &LayoutNode, old: &LayoutNode) -> bool {
    if new.element != old.element {
        return false;
    }

    match (&new.children, &old.children) {
        (Some(new_children), Some(old_children)) => {
            new_children.len() == old_children.len()
                && new_children
                    .iter()
                    .zip(old_children.iter())
                    .all(|(n, o)| Rc::ptr_eq(&n.node, &o.node))
        }
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Element;
    use crate::layout::elements::{LineElement, TableBorderElement};
    use crate::model::{NodeId, PreeditDecor, TableAlign};
    use crate::runtime::{Message, Runtime};
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
        let root = doc.node(NodeId::ROOT).unwrap();
        let settings = doc.settings();
        let constraints = BoxConstraints::new(0.0, 400.0, 0.0, f32::INFINITY);
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let default_attrs = DefaultAttrs::default();

        let decorations = Decorations::default();
        let ctx = LayoutContext::new(
            &root,
            &settings,
            &default_attrs,
            &decorations,
            &view_states,
            &cache,
        );
        let layout_without_preedit = root.node().unwrap().layout(&ctx, constraints);
        let line_without_preedit =
            first_line(&layout_without_preedit).expect("라인을 찾을 수 있어야 함");
        assert!(
            line_without_preedit.preedit.is_none(),
            "초기 레이아웃에는 preedit가 없어야 함"
        );

        let decorations_with_preedit = Decorations {
            preedit: Some(PreeditDecor {
                node_id: p,
                offset: 1,
                text: "가".into(),
            }),
            pending_styles: Default::default(),
        };

        let p_node = doc.node(p).unwrap();
        let ancestors: Vec<NodeId> = p_node.ancestors().map(|n| n.node_id()).collect();
        cache
            .borrow_mut()
            .invalidate_with_ancestors(p, ancestors.into_iter());

        let ctx_with_preedit = LayoutContext::new(
            &root,
            &settings,
            &default_attrs,
            &decorations_with_preedit,
            &view_states,
            &cache,
        );
        let layout_with_preedit = root.node().unwrap().layout(&ctx_with_preedit, constraints);
        let line_with_preedit =
            first_line(&layout_with_preedit).expect("preedit 적용 후에도 라인이 필요함");

        let preedit = line_with_preedit
            .preedit
            .as_ref()
            .expect("리스트 아이템 내 문단에도 preedit가 반영되어야 함");
        assert_eq!(preedit.node_id, p, "preedit 대상 노드가 일치해야 함");
        assert_eq!(preedit.offset, 1, "preedit 오프셋이 유지되어야 함");
    }

    fn find_table_border(layout: &LayoutNode) -> Option<&TableBorderElement> {
        if let Some(Element::TableBorder(ref t)) = layout.element {
            return Some(t);
        }
        if let Some(ref children) = layout.children {
            for child in children {
                if let Some(t) = find_table_border(&child.node) {
                    return Some(t);
                }
            }
        }
        None
    }

    #[test]
    fn layout_cache_respects_element_change_with_same_children() {
        let mut t = id!();
        let state = state! {
            doc {
                @t table(align: TableAlign::Left,) {
                   table_row {
                       table_cell {
                           paragraph { text { "cell" } }
                       }
                   }
                }
            }
            selection { (t, 0) }
        };

        let mut rt = Runtime::new(400.0, 1.0, state);

        rt.layout();

        let page = &rt.pages()[0];
        let t_elem =
            find_table_border(&page.root.node).expect("Should find table border in first layout");
        assert_eq!(
            t_elem.align,
            TableAlign::Left,
            "Initial align should be Left"
        );

        rt.update(Message::SetTableAlign {
            table_id: t.to_string(),
            align: TableAlign::Right,
        });
        rt.flush();
        rt.tick();

        rt.layout();

        let page = &rt.pages()[0];
        let t_elem =
            find_table_border(&page.root.node).expect("Should find table border in second layout");

        assert_eq!(
            t_elem.align,
            TableAlign::Right,
            "Align should be updated to Right"
        );
    }

    #[test]
    fn resolve_cascade_font_returns_root_defaults_when_no_cascade() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let node = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(
            &node,
            &settings,
            &default_attrs,
            &decorations,
            &view_states,
            &cache,
        );

        let (family, weight, _) = ctx.resolve_cascade_font();
        assert_eq!(family, default_attrs.font_family());
        assert_eq!(weight, default_attrs.font_weight());
    }

    #[test]
    fn resolve_cascade_font_paragraph_overrides_root() {
        let mut p = id!();
        let state = state! {
            doc { @p paragraph {} }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &crate::model::Attr::from_styles(&[crate::model::Style::FontFamily(
                    crate::model::FontFamilyStyle {
                        family: "ParagraphFont".to_string(),
                    },
                )]),
            )
            .unwrap();
        });

        let doc = &state.doc;
        let node = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(
            &node,
            &settings,
            &default_attrs,
            &decorations,
            &view_states,
            &cache,
        );

        let (family, _, _) = ctx.resolve_cascade_font();
        assert_eq!(
            family, "ParagraphFont",
            "paragraph cascade_attrs must override root default"
        );
    }

    #[test]
    fn resolve_cascade_font_includes_style_overrides() {
        // FoldTitle has style_overrides() -> FontWeight(500)
        let mut f = id!();
        let state = state! {
            doc {
                fold {
                    @f fold_title {}
                }
            }
            selection { (f, 0) }
        };

        let doc = &state.doc;
        let node = doc.node(f).unwrap();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(
            &node,
            &settings,
            &default_attrs,
            &decorations,
            &view_states,
            &cache,
        );

        let (_, weight, _) = ctx.resolve_cascade_font();
        // FOLD_TITLE_FONT_WEIGHT = 500
        assert_eq!(
            weight, 500,
            "style_overrides FontWeight must be reflected in resolve_cascade_font"
        );
    }
}
