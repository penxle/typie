use editor_crdt::Dot;
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::materialize_target;

pub fn materialize_block(tr: &mut Transaction, block: Dot) -> Result<Dot, CommandError> {
    materialize_target(tr, block)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;

    use super::*;

    #[test]
    fn synthetic_block_becomes_real_and_a_real_block_passes_through() {
        let (state, ..) = state! {
            doc { root { fold paragraph {} } }
            selection: none
        };
        let (title, authored) = {
            let view = state.view();
            let root = view.root().expect("root");
            let fold = root
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Fold)
                .expect("fold");
            let title = fold
                .child_blocks()
                .find(|block| block.node_type() == NodeType::FoldTitle)
                .expect("fold title")
                .id();
            let authored = root
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Paragraph)
                .expect("paragraph")
                .id();
            (title, authored)
        };
        assert!(title.is_synthetic());
        assert!(!authored.is_synthetic());

        let mut tr = Transaction::new(&state);
        let materialized = materialize_block(&mut tr, title).expect("materialize");
        assert!(!materialized.is_synthetic());
        assert_ne!(materialized, title);
        assert_eq!(
            materialize_block(&mut tr, authored).expect("pass through"),
            authored
        );
    }
}
