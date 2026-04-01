use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_escape(&mut self) -> Vec<Effect> {
        if !self.state.selection.is_collapsed() {
            return self.transact(|tr| {
                tr.collapse_selection()?;
                Ok(true)
            });
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Message;

    #[test]
    fn test_escape_collapses_selection() {
        let mut p1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello" } }
            }
            selection { (p1, 0) -> (p1, 5) }
        };
        rt.layout();

        rt.update(Message::Escape);

        let selection = &rt.state().selection;
        assert!(selection.is_collapsed());
        assert_eq!(selection.anchor.node_id, p1);
        assert_eq!(selection.anchor.offset, 5);
    }

    #[test]
    fn test_escape_does_nothing_if_collapsed() {
        let mut p1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello" } }
            }
            selection { (p1, 2) }
        };
        rt.layout();

        let initial_selection = rt.state().selection;
        rt.update(Message::Escape);

        assert_eq!(rt.state().selection, initial_selection);
    }
}
