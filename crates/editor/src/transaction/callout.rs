use crate::model::{CalloutNode, Node};
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn toggle_callout(&mut self) -> Result<bool> {
        if self.lift_from_ancestor(|parent, _blocks| matches!(parent, Node::Callout(_)))? {
            return Ok(true);
        }

        if self.wrap_in_ancestor(Node::Callout(CalloutNode::default()))? {
            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction::Transaction;

    #[test]
    fn toggle_callout_wraps_when_outside() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr.toggle_callout().unwrap());

        let expected = state! {
            doc {
                callout {
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
    fn toggle_callout_lifts_when_inside() {
        let mut p = id!();

        let initial = state! {
            doc {
                callout {
                    @p paragraph { text { "hello" } }
                }
                paragraph { text { "world" } }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.toggle_callout().unwrap());

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
    fn toggle_callout_not_applicable() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                callout {
                    @p1 paragraph { text { "hello" } }
                }
                @p2 paragraph { text { "hello" } }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        let mut tr = Transaction::new(&state);
        let result = tr.toggle_callout().unwrap();

        assert!(!result);
    }

    #[test]
    fn toggle_callout_to_wrap_in_callout_in_list_item() {
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

        let actual = transact!(state, |tr| tr.toggle_callout().unwrap());

        let expected = state! {
            doc {
                callout {
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
    fn toggle_callout_to_lift_callout_in_list_item() {
        let mut p = id!();

        let state = state! {
            doc {
                callout {
                    bullet_list {
                        list_item {
                            @p paragraph { text { "t" } }
                        }
                    }
                }
            }
            selection { (p, 0) -> (p, 1) }
        };

        let actual = transact!(state, |tr| tr.toggle_callout().unwrap());

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
}
