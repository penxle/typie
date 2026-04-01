use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_insert_newline(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            if tr.insert_paragraph_on_nontextblock_selection()? {
                return Ok(true);
            }
            tr.delete_selection()?;
            tr.normalize()?;

            if tr.split_list_item()? {
                return Ok(true);
            }

            if tr.lift_on_empty_paragraph()? {
                return Ok(true);
            }
            tr.split_paragraph()
        })
    }

    pub fn handle_insert_hard_break(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.insert_hard_break()
        })
    }

    pub fn handle_insert_page_break(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_paragraph_on_nontextblock_selection()?;
            tr.delete_selection()?;
            tr.normalize()?;
            tr.insert_page_break()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::NodeId;
    use crate::runtime::Message;

    #[test]
    fn insert_page_break_on_first_block() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                image() {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        rt.update(Message::InsertPageBreak);
        rt.layout();

        let expected = state! {
            doc {
                paragraph {
                    page_break {}
                }
                image() {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn insert_page_break_on_block() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "Hello" }
                }
                image() {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        rt.update(Message::InsertPageBreak);
        rt.layout();

        let expected = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                image() {}
                paragraph {
                    page_break {}
                }
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn insert_newline_on_empty_paragraph_in_fold() {
        let mut n1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {}
                fold {
                    fold_title {
                        text { "title" }
                    }
                    fold_content {
                        paragraph {
                            text { "dd" }
                        }
                        @n1 paragraph {}
                        paragraph {
                            text { "d" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        rt.layout();
        rt.update(Message::InsertNewline);
        rt.tick();

        let expected = state! {
            doc {
                paragraph {}
                fold {
                    fold_title {
                        text { "title" }
                    }
                    fold_content {
                        paragraph {
                            text { "dd" }
                        }
                        paragraph {}
                        @n1 paragraph {}
                        paragraph {
                            text { "d" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn insert_newline_after_select_all() {
        let mut p1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "Hello" }
                }
                paragraph {
                    text { "World" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 2) }
        };

        rt.update(Message::InsertNewline);
        rt.tick();

        let expected = state! {
            doc {
                paragraph {}
                @p1 paragraph {}
            }
            selection { (p1, 0) -> (p1, 0) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn insert_hard_break_after_select_all() {
        let mut p1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "Hello" }
                }
                paragraph {
                    text { "World" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 2) }
        };

        rt.update(Message::InsertHardBreak);
        rt.tick();

        let expected = state! {
            doc {
                @p1 paragraph {
                  hard_break {}
                }
            }
            selection { (p1, 1) -> (p1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn insert_page_break_after_select_all() {
        let mut p1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "Hello" }
                }
                paragraph {
                    text { "World" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 2) }
        };

        rt.update(Message::InsertPageBreak);
        rt.tick();

        let expected = state! {
            doc {
                paragraph {
                  page_break {}
                }
                @p1 paragraph {}
            }
            selection { (p1, 0) -> (p1, 0) }
        };

        assert_state_eq!(*rt.state(), expected);
    }
}
