use crate::model::*;
use crate::transaction::Transaction;
use anyhow::{Context, Result};

impl Transaction {
    pub fn add_remark(&self, node_id: NodeId, remark: &Remark) -> Result<()> {
        let node = self.node(node_id).context("Node not found")?;
        node.as_mut().add_remark(remark)?;
        Ok(())
    }

    pub fn update_remark(&self, node_id: NodeId, remark_id: RemarkId, text: &str) -> Result<()> {
        let node = self.node(node_id).context("Node not found")?;
        node.as_mut().update_remark(remark_id, text)?;
        Ok(())
    }

    pub fn remove_remark(&self, node_id: NodeId, remark_id: RemarkId) -> Result<()> {
        let node = self.node(node_id).context("Node not found")?;
        node.as_mut().remove_remark(remark_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_remark(text: &str) -> Remark {
        Remark {
            id: NodeId::new(),
            user_id: "user1".to_string(),
            text: text.to_string(),
            created_at: 1700000000000,
        }
    }

    #[test]
    fn add_remark_to_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let state = transact!(initial, |tr| {
            let remark = make_remark("a comment");
            tr.add_remark(p, &remark).unwrap();
        });

        let node = state.doc.node(p).unwrap();
        let remarks = node.remarks();
        assert_eq!(remarks.len(), 1);
        assert_eq!(remarks[0].text, "a comment");
        assert_eq!(remarks[0].user_id, "user1");
    }

    #[test]
    fn add_multiple_remarks() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let r1 = Remark {
            id: NodeId::new(),
            user_id: "user1".to_string(),
            text: "first".to_string(),
            created_at: 1000,
        };
        let r2 = Remark {
            id: NodeId::new(),
            user_id: "user2".to_string(),
            text: "second".to_string(),
            created_at: 2000,
        };

        let state = transact!(initial, |tr| {
            tr.add_remark(p, &r1).unwrap();
            tr.add_remark(p, &r2).unwrap();
        });

        let node = state.doc.node(p).unwrap();
        let remarks = node.remarks();
        assert_eq!(remarks.len(), 2);
        assert_eq!(remarks[0].text, "first");
        assert_eq!(remarks[1].text, "second");
    }

    #[test]
    fn update_remark_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let remark = make_remark("original");
        let remark_id = remark.id;

        let state = transact!(initial, |tr| {
            tr.add_remark(p, &remark).unwrap();
        });

        let state = transact!(state, |tr| {
            tr.update_remark(p, remark_id, "updated").unwrap();
        });

        let node = state.doc.node(p).unwrap();
        let remarks = node.remarks();
        assert_eq!(remarks.len(), 1);
        assert_eq!(remarks[0].text, "updated");
        assert_eq!(remarks[0].id, remark_id);
    }

    #[test]
    fn remove_remark() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let remark = make_remark("to be removed");
        let remark_id = remark.id;

        let state = transact!(initial, |tr| {
            tr.add_remark(p, &remark).unwrap();
        });

        let state = transact!(state, |tr| {
            tr.remove_remark(p, remark_id).unwrap();
        });

        let node = state.doc.node(p).unwrap();
        assert!(node.remarks().is_empty());
    }

    #[test]
    fn all_remarks_across_nodes() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "first" } }
                @p2 paragraph { text { "second" } }
            }
            selection { (p1, 0) }
        };

        let r1 = Remark {
            id: NodeId::new(),
            user_id: "u1".to_string(),
            text: "on first".to_string(),
            created_at: 2000,
        };
        let r2 = Remark {
            id: NodeId::new(),
            user_id: "u2".to_string(),
            text: "on second".to_string(),
            created_at: 1000,
        };

        let state = transact!(initial, |tr| {
            tr.add_remark(p1, &r1).unwrap();
            tr.add_remark(p2, &r2).unwrap();
        });

        let all = state.doc.all_remarks();
        assert_eq!(all.len(), 2);
        // sorted by node document order, not created_at
        assert_eq!(all[0].0, p1);
        assert_eq!(all[0].1.text, "on first");
        assert_eq!(all[1].0, p2);
        assert_eq!(all[1].1.text, "on second");
    }

    #[test]
    fn add_remark_on_inline_node_fails() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let state: crate::runtime::State = initial;
        let tr = Transaction::new(&state);

        let text_id = state.doc.node(p).unwrap().first_child().unwrap().node_id();
        let remark = make_remark("bad");
        let result = tr.add_remark(text_id, &remark);
        assert!(result.is_err());
    }
}
