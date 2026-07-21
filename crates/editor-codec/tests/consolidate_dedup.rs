use editor_codec::{
    CodecError, Corruption, ReencodableChangesets, consolidate_stream, decode_changeset_stream,
    decode_changesets, encode_changesets,
};
use editor_crdt::{Changeset, Dot, ListOp, Op, OpGraph};
use editor_model::{EditOp, SeqItem};

fn char_cs(clock: u64, parents: Vec<Dot>, ch: char) -> Changeset<EditOp> {
    Changeset {
        ops: vec![Op {
            id: Dot::new(1, clock),
            parents,
            payload: EditOp::Seq(ListOp::Ins {
                pos: clock as usize,
                item: SeqItem::Char(ch),
            }),
        }],
    }
}

fn bundle(css: Vec<Changeset<EditOp>>) -> Vec<u8> {
    encode_changesets(ReencodableChangesets::from_local_ops(css)).unwrap()
}

fn stream(bundles: &[Vec<u8>]) -> Vec<u8> {
    bundles.concat()
}

fn sorted_ops(g: &OpGraph<EditOp>) -> Vec<Op<EditOp>> {
    let mut ops: Vec<Op<EditOp>> = g.ordered_ops().unwrap().into_iter().cloned().collect();
    ops.sort_by_key(|o| o.id);
    ops
}

fn assert_graph_identity(original: &[Changeset<EditOp>], produced: &[Changeset<EditOp>]) {
    let g_orig = OpGraph::from_changesets(original.to_vec()).unwrap();
    let g_prod = OpGraph::from_changesets(produced.to_vec()).unwrap();

    let mut heads_orig: Vec<Dot> = g_orig.current_heads().copied().collect();
    let mut heads_prod: Vec<Dot> = g_prod.current_heads().copied().collect();
    heads_orig.sort();
    heads_prod.sort();
    assert_eq!(heads_orig, heads_prod);

    assert_eq!(sorted_ops(&g_orig), sorted_ops(&g_prod));
}

#[test]
fn drops_verbatim_duplicates_across_envelopes() {
    let a = char_cs(0, vec![], 'a');
    let b = char_cs(1, vec![Dot::new(1, 0)], 'b');
    let c = char_cs(2, vec![Dot::new(1, 1)], 'c');

    let env1 = bundle(vec![a.clone(), b.clone()]);
    let env2 = bundle(vec![b.clone(), c.clone()]);
    let s = stream(&[env1, env2]);

    let out = consolidate_stream(&s)
        .unwrap()
        .expect("verbatim dedup across envelopes yields Some");
    assert_eq!(out.consumed, 2);
    assert_eq!(out.consumed_bytes, s.len());

    let produced = decode_changesets(&out.payload).unwrap().into_graph_input();
    assert_eq!(produced.len(), 3);

    let original = decode_changeset_stream(&s).unwrap().into_graph_input();
    assert_graph_identity(&original, &produced);
}

#[test]
fn single_envelope_with_internal_duplicates_consolidates() {
    let a = char_cs(0, vec![], 'a');
    let b = char_cs(1, vec![Dot::new(1, 0)], 'b');

    let env1 = bundle(vec![a.clone(), b.clone(), a.clone()]);
    let s = stream(&[env1]);

    let out = consolidate_stream(&s)
        .unwrap()
        .expect("single envelope with internal duplicate yields Some");
    assert_eq!(out.consumed, 1);

    let produced = decode_changesets(&out.payload).unwrap().into_graph_input();
    assert_eq!(produced.len(), 2);

    let original = decode_changeset_stream(&s).unwrap().into_graph_input();
    assert_graph_identity(&original, &produced);
}

#[test]
fn single_clean_envelope_returns_none() {
    let a = char_cs(0, vec![], 'a');
    let b = char_cs(1, vec![Dot::new(1, 0)], 'b');
    let env1 = bundle(vec![a, b]);

    assert!(consolidate_stream(&env1).unwrap().is_none());
}

#[test]
fn same_dots_divergent_content_is_corruption() {
    let a = char_cs(0, vec![], 'a');
    let a_prime = char_cs(0, vec![], 'x');

    let env1 = bundle(vec![a]);
    let env2 = bundle(vec![a_prime]);
    let s = stream(&[env1, env2]);

    assert!(matches!(
        consolidate_stream(&s),
        Err(CodecError::Corruption(
            Corruption::DivergentDuplicate { .. }
        ))
    ));
}
