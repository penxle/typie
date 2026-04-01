use crate::model::{BlockquoteNode, BlockquoteVariant, Node};
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn toggle_blockquote(&mut self, variant: BlockquoteVariant) -> Result<bool> {
        if self.lift_from_ancestor(|parent, _blocks| matches!(parent, Node::Blockquote(_)))? {
            return Ok(true);
        }

        if self.wrap_in_ancestor(Node::Blockquote(BlockquoteNode { variant }))? {
            return Ok(true);
        }

        Ok(false)
    }

    pub fn set_blockquote(&mut self, variant: BlockquoteVariant) -> Result<bool> {
        self.lift_from_ancestor(|parent, _blocks| matches!(parent, Node::Blockquote(_)))?;
        self.wrap_in_ancestor(Node::Blockquote(BlockquoteNode { variant }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Effect;

    #[test]
    fn toggle_wraps_when_outside() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "hello" } }
                    @p2 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_lifts_when_inside() {
        let mut p = id!();

        let initial = state! {
            doc {
                blockquote {
                    @p paragraph { text { "hello" } }
                }
                paragraph { text { "world" } }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph { text { "hello" } }
                paragraph { text { "world" } }
            }
            selection { (p, 0) -> (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_blockquote_not_applicable() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "hello" } }
                }
                @p2 paragraph { text { "hello" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        let mut tr = Transaction::new(&state);
        let result = tr.toggle_blockquote(BlockquoteVariant::default()).unwrap();

        assert!(!result);
    }

    #[test]
    fn toggle_blockquote_to_wrap_in_blockquote_in_list_item() {
        let mut p = id!();

        let state = state! {
            doc {
                bullet_list {
                    list_item {
                        @p paragraph { text { "t" } }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(state, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                blockquote {
                    bullet_list {
                        list_item {
                            @p paragraph { text { "t" } }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) -> (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_blockquote_to_wrap_lists_in_blockquote() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "t" } }
                    }
                }
                ordered_list {
                    list_item {
                        @p2 paragraph { text { "t" } }
                    }
                    list_item {
                        paragraph { text { "t" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(state, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                blockquote {
                    bullet_list {
                        list_item {
                            @p1 paragraph { text { "t" } }
                        }
                    }
                    ordered_list {
                        list_item {
                            @p2 paragraph { text { "t" } }
                        }
                        list_item {
                            paragraph { text { "t" } }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_blockquote_to_lift_blockquote_in_list_item() {
        let mut p = id!();

        let state = state! {
            doc {
                blockquote {
                    bullet_list {
                        list_item {
                            @p paragraph { text { "t" } }
                        }
                    }
                }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(state, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p paragraph { text { "t" } }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) -> (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn wrap_in_blockquote_emits_node_changed_for_children() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let has_paragraph_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeMutated { node_id, kind: crate::runtime::MutationKind::Attr } if *node_id == p));
        assert!(
            has_paragraph_changed,
            "Effect::NodeChanged should be emitted for the paragraph when wrapping in blockquote"
        );
    }

    #[test]
    fn toggle_blockquote_to_lift_blockquote_has_lists() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                blockquote {
                    bullet_list {
                        list_item {
                            @p1 paragraph { text { "t" } }
                        }
                    }
                    ordered_list {
                        list_item {
                            @p2 paragraph { text { "t" } }
                        }
                        list_item {
                            paragraph { text { "t" } }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(state, |tr| tr
            .toggle_blockquote(BlockquoteVariant::default())
            .unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "t" } }
                    }
                }
                ordered_list {
                    list_item {
                        @p2 paragraph { text { "t" } }
                    }
                    list_item {
                        paragraph { text { "t" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_blockquote_rewraps_with_requested_variant_when_inside_blockquote() {
        let mut p = id!();

        let initial = state! {
            doc {
                blockquote {
                    @p paragraph { text { "hello" } }
                }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .set_blockquote(BlockquoteVariant::MessageSent)
            .unwrap());

        let expected = state! {
            doc {
                blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p paragraph { text { "hello" } }
                }
                paragraph {}
            }
            selection { (p, 0) -> (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_blockquote_wraps_when_outside_blockquote() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .set_blockquote(BlockquoteVariant::LeftQuote)
            .unwrap());

        let expected = state! {
            doc {
                blockquote(variant: BlockquoteVariant::LeftQuote,) {
                    @p paragraph { text { "hello" } }
                }
                paragraph {}
            }
            selection { (p, 0) -> (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }
}
