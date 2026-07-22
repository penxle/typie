use editor_crdt::{Dot, ListOp, OpGraph};
use editor_model::{
    Anchor, AtomLeaf, Bias, Child, EditOp, Modifier, ModifierType, Node, NodeType, SeqItem, SpanOp,
};
use hashbrown::HashSet;

use crate::projected_state::ProjectedState;

pub type CorpusStep = (u8, u8, u8, u8, u8);

#[derive(Clone, Debug, Default)]
pub struct CorpusSpans {
    pub a: Vec<(Anchor, Anchor, Dot)>,
    pub b: Vec<(Anchor, Anchor, Dot)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CorpusStats {
    pub partial_deliveries: usize,
    pub undeletes: usize,
    pub b_local_spans: usize,
    pub b_local_edits: usize,
    pub tombstone_anchor_spans: usize,
    pub reversed_spans: usize,
    pub add_spans: usize,
    pub remove_spans: usize,
    pub before_anchors: usize,
    pub after_anchors: usize,
    pub fold_templates: usize,
    pub table_templates: usize,
    pub nested_fold_templates: usize,
    pub nested_attach_templates: usize,
    pub container_kill_steps: usize,
    pub list_templates: usize,
    pub wrap_templates: usize,
    pub redeliveries: usize,
}

pub struct CorpusRun {
    pub a: ProjectedState,
    pub b: ProjectedState,
    pub spans: CorpusSpans,
    pub stats: CorpusStats,
}

fn seq_char(pos: usize, c: char) -> EditOp {
    EditOp::Seq(ListOp::Ins {
        pos,
        item: SeqItem::Char(c),
    })
}

fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
    EditOp::Seq(ListOp::Ins {
        pos,
        item: SeqItem::Block {
            node_type,
            parents,
            attrs: vec![],
        },
    })
}

type OnBoundary<'x> = &'x mut dyn FnMut(&ProjectedState, &ProjectedState, &CorpusSpans);

fn is_char_ins(o: &editor_crdt::Op<EditOp>) -> bool {
    matches!(
        o.payload,
        EditOp::Seq(ListOp::Ins {
            item: SeqItem::Char(_),
            ..
        })
    )
}

fn record_span_into(spans: &mut Vec<(Anchor, Anchor, Dot)>, op: &editor_crdt::Op<EditOp>) {
    if let EditOp::Span(s) = &op.payload {
        let (sa, ea) = s.anchors();
        spans.push((sa, ea, op.id));
    }
}

fn notify(
    active: &ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    spans: &CorpusSpans,
    on_boundary: OnBoundary,
) {
    if active_is_b {
        on_boundary(other, active, spans);
    } else {
        on_boundary(active, other, spans);
    }
}

// Delivers sealed changesets ONE at a time, firing the boundary callback after
// each; received char dots join the receiving replica's anchor candidates so
// cross-actor anchor histories (remote span on a concurrently deleted char)
// are generatable. `partial` delivers only the first half, leaving replicas
// divergent across subsequent edits.
fn deliver(
    dst: &mut ProjectedState,
    src: &ProjectedState,
    dst_is_a: bool,
    dst_live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    stats: &mut CorpusStats,
    partial: bool,
    redeliver: bool,
    on_boundary: OnBoundary,
) {
    let heads: HashSet<Dot> = dst.graph().current_heads().copied().collect();
    let mut css = src.graph().missing_changesets_tolerant(&heads);
    if partial && css.len() > 1 {
        let keep = css.len().div_ceil(2);
        css.truncate(keep);
        stats.partial_deliveries += 1;
    }
    for cs in css.iter().cloned() {
        let (next, ops) = dst
            .receive_changesets(vec![cs])
            .expect("corpus changesets apply");
        *dst = next;
        for o in &ops {
            if is_char_ins(o) {
                dst_live.push(o.id);
            }
            let side = if dst_is_a { &mut spans.a } else { &mut spans.b };
            record_span_into(side, o);
        }
        notify(dst, src, !dst_is_a, spans, on_boundary);
    }
    if redeliver && !css.is_empty() {
        stats.redeliveries += 1;
        for cs in css {
            let (next, ops) = dst
                .receive_changesets(vec![cs])
                .expect("corpus redelivered changesets apply");
            assert!(
                ops.is_empty(),
                "redelivered changeset must apply zero novel ops"
            );
            *dst = next;
            notify(dst, src, !dst_is_a, spans, on_boundary);
        }
    }
}

// One template sub-op: apply on the active replica, update anchor candidates
// and the span record, then fire the boundary callback.
fn template_step(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    op: EditOp,
    on_boundary: OnBoundary,
) -> Dot {
    let applied = active
        .apply(op)
        .expect("corpus template op applies (schema-shaped appends)");
    if is_char_ins(&applied) {
        live.push(applied.id);
    }
    let side = if active_is_b {
        &mut spans.b
    } else {
        &mut spans.a
    };
    record_span_into(side, &applied);
    let id = applied.id;
    notify(active, other, active_is_b, spans, on_boundary);
    id
}

fn apply_fold_template(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    ch: char,
    on_boundary: OnBoundary,
) -> (Dot, Vec<Dot>) {
    let p = active.seq_checkout().visible_len();
    let fold = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::Fold, vec![Dot::ROOT]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::FoldTitle, vec![Dot::ROOT, fold]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_char(p, ch),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let content = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::FoldContent, vec![Dot::ROOT, fold]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::Paragraph, vec![Dot::ROOT, fold, content]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_char(p, ch),
        on_boundary,
    );
    (content, vec![Dot::ROOT, fold])
}

fn apply_table_template(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    ch: char,
    on_boundary: OnBoundary,
) -> (Dot, Vec<Dot>) {
    let p = active.seq_checkout().visible_len();
    let table = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::Table, vec![Dot::ROOT]),
        on_boundary,
    );
    let mut last_row = table;
    let mut last_cell = table;
    for cells in [1usize, 2] {
        let p = active.seq_checkout().visible_len();
        let row = template_step(
            active,
            other,
            active_is_b,
            live,
            spans,
            seq_block(p, NodeType::TableRow, vec![Dot::ROOT, table]),
            on_boundary,
        );
        last_row = row;
        for _ in 0..cells {
            let p = active.seq_checkout().visible_len();
            let cell = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, NodeType::TableCell, vec![Dot::ROOT, table, row]),
                on_boundary,
            );
            last_cell = cell;
            let p = active.seq_checkout().visible_len();
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, NodeType::Paragraph, vec![Dot::ROOT, table, row, cell]),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_char(p, ch),
                on_boundary,
            );
        }
    }
    (last_cell, vec![Dot::ROOT, table, last_row])
}

fn apply_wrap_template(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    y: usize,
    ch: char,
    attach_under: Option<(Dot, Vec<Dot>)>,
    on_boundary: OnBoundary,
) -> (Dot, Vec<Dot>) {
    let base: Vec<Dot> = match attach_under {
        Some((host, mut host_parents)) => {
            host_parents.push(host);
            host_parents
        }
        None => vec![Dot::ROOT],
    };
    let p = active.seq_checkout().visible_len();
    match y % 4 {
        0 | 1 => {
            let list_ty = if y % 4 == 0 {
                NodeType::BulletList
            } else {
                NodeType::OrderedList
            };
            let list = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, list_ty, base.clone()),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let mut item_parents = base.clone();
            item_parents.push(list);
            let item = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, NodeType::ListItem, item_parents),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let mut para_parents = base.clone();
            para_parents.push(list);
            para_parents.push(item);
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, NodeType::Paragraph, para_parents),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_char(p, ch),
                on_boundary,
            );
            (list, base)
        }
        _ => {
            let wrap_ty = if y % 4 == 2 {
                NodeType::Blockquote
            } else {
                NodeType::Callout
            };
            let wrap = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, wrap_ty, base.clone()),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let mut para_parents = base.clone();
            para_parents.push(wrap);
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_block(p, NodeType::Paragraph, para_parents),
                on_boundary,
            );
            let p = active.seq_checkout().visible_len();
            let _ = template_step(
                active,
                other,
                active_is_b,
                live,
                spans,
                seq_char(p, ch),
                on_boundary,
            );
            (wrap, base)
        }
    }
}

fn apply_nested_fold_template(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    ch: char,
    on_boundary: OnBoundary,
) -> (Dot, Vec<Dot>) {
    let p = active.seq_checkout().visible_len();
    let outer = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::Fold, vec![Dot::ROOT]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::FoldTitle, vec![Dot::ROOT, outer]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let content = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::FoldContent, vec![Dot::ROOT, outer]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let inner = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(p, NodeType::Fold, vec![Dot::ROOT, outer, content]),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(
            p,
            NodeType::FoldTitle,
            vec![Dot::ROOT, outer, content, inner],
        ),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let inner_content = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(
            p,
            NodeType::FoldContent,
            vec![Dot::ROOT, outer, content, inner],
        ),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_block(
            p,
            NodeType::Paragraph,
            vec![Dot::ROOT, outer, content, inner, inner_content],
        ),
        on_boundary,
    );
    let p = active.seq_checkout().visible_len();
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        seq_char(p, ch),
        on_boundary,
    );
    (inner_content, vec![Dot::ROOT, outer, content, inner])
}

// Removes either the host's own marker (`x % 2 == 1`, orphaning its
// descendants — a revival-path stressor) or its full content range
// (`x % 2 == 0`, leaving the host childless — a dismantle/filler stressor).
// No-ops (returns `false`, no counter bump) when the host is absent or
// already empty.
fn container_kill_step(
    active: &mut ProjectedState,
    other: &ProjectedState,
    active_is_b: bool,
    live: &mut Vec<Dot>,
    spans: &mut CorpusSpans,
    x: u8,
    on_boundary: OnBoundary,
    last_container: &Option<(Dot, Vec<Dot>)>,
) -> bool {
    let Some((host, _)) = last_container else {
        return false;
    };
    let host = *host;
    if active.projected().tree.get(host).is_none() {
        return false;
    }
    let payload = if x % 2 == 1 {
        let Some(pos) = active
            .seq_checkout()
            .resolve_boundary(host, Bias::Before.into())
            .map(|b| b.position)
        else {
            return false;
        };
        EditOp::Seq(ListOp::Del { pos, len: 1 })
    } else {
        let Some(start) = active
            .seq_checkout()
            .resolve_boundary(host, Bias::After.into())
            .map(|b| b.position)
        else {
            return false;
        };
        let Some(last) = active.subtree_max_seq_pos(host) else {
            return false;
        };
        if last + 1 <= start {
            return false;
        }
        EditOp::Seq(ListOp::Del {
            pos: start,
            len: last + 1 - start,
        })
    };
    let _ = template_step(
        active,
        other,
        active_is_b,
        live,
        spans,
        payload,
        on_boundary,
    );
    true
}

pub fn run_corpus(steps: &[CorpusStep], on_boundary: OnBoundary) -> CorpusRun {
    let mut a = ProjectedState::empty();
    let mut b = ProjectedState::from_graph(OpGraph::with_actor(2)).expect("empty graph projects");
    let mut spans = CorpusSpans::default();
    let mut stats = CorpusStats::default();
    let mut on_b = false;
    let mut live_a: Vec<Dot> = Vec::new();
    let mut live_b: Vec<Dot> = Vec::new();
    let mut dels_a: Vec<Dot> = Vec::new();
    let mut dels_b: Vec<Dot> = Vec::new();
    let mut last_container_a: Option<(Dot, Vec<Dot>)> = None;
    let mut last_container_b: Option<(Dot, Vec<Dot>)> = None;

    a.commit();
    deliver(
        &mut b,
        &a,
        false,
        &mut live_b,
        &mut spans,
        &mut stats,
        false,
        false,
        on_boundary,
    );

    for &(op, x, y, bias, ch) in steps {
        let ch = char::from(b'a' + (ch % 26));
        let bias_s = if bias & 1 == 0 {
            Bias::Before
        } else {
            Bias::After
        };
        let bias_e = if bias & 2 == 0 {
            Bias::Before
        } else {
            Bias::After
        };
        match op % 14 {
            11 => {
                on_b = !on_b;
                a.commit();
                b.commit();
                let partial = bias & 2 != 0;
                let redeliver = bias & 1 != 0;
                if y % 2 == 0 {
                    deliver(
                        &mut a,
                        &b,
                        true,
                        &mut live_a,
                        &mut spans,
                        &mut stats,
                        partial,
                        redeliver,
                        on_boundary,
                    );
                } else {
                    deliver(
                        &mut b,
                        &a,
                        false,
                        &mut live_b,
                        &mut spans,
                        &mut stats,
                        partial,
                        redeliver,
                        on_boundary,
                    );
                }
                continue;
            }
            12 => {
                let host = match x % 3 {
                    1 => {
                        stats.table_templates += 1;
                        if on_b {
                            apply_table_template(
                                &mut b,
                                &a,
                                true,
                                &mut live_b,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        } else {
                            apply_table_template(
                                &mut a,
                                &b,
                                false,
                                &mut live_a,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        }
                    }
                    2 => {
                        stats.nested_fold_templates += 1;
                        if on_b {
                            apply_nested_fold_template(
                                &mut b,
                                &a,
                                true,
                                &mut live_b,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        } else {
                            apply_nested_fold_template(
                                &mut a,
                                &b,
                                false,
                                &mut live_a,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        }
                    }
                    _ => {
                        stats.fold_templates += 1;
                        if on_b {
                            apply_fold_template(
                                &mut b,
                                &a,
                                true,
                                &mut live_b,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        } else {
                            apply_fold_template(
                                &mut a,
                                &b,
                                false,
                                &mut live_a,
                                &mut spans,
                                ch,
                                on_boundary,
                            )
                        }
                    }
                };
                if on_b {
                    last_container_b = Some(host);
                } else {
                    last_container_a = Some(host);
                }
                continue;
            }
            13 => {
                if x >= 200 {
                    let killed = if on_b {
                        container_kill_step(
                            &mut b,
                            &a,
                            true,
                            &mut live_b,
                            &mut spans,
                            x,
                            on_boundary,
                            &last_container_b,
                        )
                    } else {
                        container_kill_step(
                            &mut a,
                            &b,
                            false,
                            &mut live_a,
                            &mut spans,
                            x,
                            on_boundary,
                            &last_container_a,
                        )
                    };
                    if killed {
                        stats.container_kill_steps += 1;
                    }
                    continue;
                }
                if (y as usize) % 4 < 2 {
                    stats.list_templates += 1;
                } else {
                    stats.wrap_templates += 1;
                }
                let nested = bias >= 200;
                let attach = if nested && (y as usize) % 4 >= 2 {
                    let last = if on_b {
                        last_container_b.as_ref()
                    } else {
                        last_container_a.as_ref()
                    };
                    last.and_then(|(host, host_parents)| {
                        let ty = if on_b {
                            b.projected().tree.get(*host)
                        } else {
                            a.projected().tree.get(*host)
                        }
                        .map(|n| n.node_type);
                        matches!(ty, Some(NodeType::FoldContent) | Some(NodeType::TableCell))
                            .then(|| (*host, host_parents.clone()))
                    })
                } else {
                    None
                };
                if attach.is_some() {
                    stats.nested_attach_templates += 1;
                }
                let host = if on_b {
                    apply_wrap_template(
                        &mut b,
                        &a,
                        true,
                        &mut live_b,
                        &mut spans,
                        y as usize,
                        ch,
                        attach,
                        on_boundary,
                    )
                } else {
                    apply_wrap_template(
                        &mut a,
                        &b,
                        false,
                        &mut live_a,
                        &mut spans,
                        y as usize,
                        ch,
                        attach,
                        on_boundary,
                    )
                };
                if on_b {
                    last_container_b = Some(host);
                } else {
                    last_container_a = Some(host);
                }
                continue;
            }
            _ => {}
        }
        let (live, dels) = if on_b {
            (&mut live_b, &mut dels_b)
        } else {
            (&mut live_a, &mut dels_a)
        };
        let visible = if on_b {
            b.seq_checkout().visible_len()
        } else {
            a.seq_checkout().visible_len()
        };
        let pick = |i: u8, v: &[Dot]| v[(i as usize) % v.len()];
        let payload = match op % 14 {
            0..=3 if visible > 0 => Some(seq_char(1 + (x as usize) % visible, ch)),
            4 if visible > 0 => Some(seq_block(
                1 + (x as usize) % visible,
                NodeType::Paragraph,
                vec![Dot::ROOT],
            )),
            5 if visible > 2 => {
                let pos = 1 + (x as usize) % (visible - 1);
                let len = 1 + (y as usize) % (visible - pos).max(1);
                Some(EditOp::Seq(ListOp::Del { pos, len }))
            }
            6 if !dels.is_empty() => {
                let del = dels.swap_remove((x as usize) % dels.len());
                Some(EditOp::Seq(ListOp::Undel { del }))
            }
            7..=9 if !live.is_empty() => {
                let m = match y % 3 {
                    0 => Modifier::Bold,
                    1 => Modifier::Italic,
                    _ => Modifier::FontSize { value: 1400 },
                };
                Some(EditOp::Span(SpanOp::AddSpan {
                    start: Anchor {
                        id: pick(x, live),
                        bias: bias_s,
                    },
                    end: Anchor {
                        id: pick(y, live),
                        bias: bias_e,
                    },
                    modifier: m,
                }))
            }
            10 if !live.is_empty() => Some(EditOp::Span(SpanOp::RemoveSpan {
                start: Anchor {
                    id: pick(x, live),
                    bias: bias_s,
                },
                end: Anchor {
                    id: pick(y, live),
                    bias: bias_e,
                },
                modifier_type: ModifierType::Bold,
            })),
            _ => None,
        };
        let Some(payload) = payload else { continue };
        let o = if on_b {
            b.apply(payload)
        } else {
            a.apply(payload)
        }
        .expect("corpus op applies — arm guards keep generated ops valid");
        if is_char_ins(&o) {
            live.push(o.id);
        }
        if matches!(o.payload, EditOp::Seq(ListOp::Del { .. })) {
            dels.push(o.id);
        }
        if matches!(o.payload, EditOp::Seq(ListOp::Undel { .. })) {
            stats.undeletes += 1;
        }
        if on_b {
            stats.b_local_edits += 1;
        }
        if let EditOp::Span(sop) = &o.payload {
            match sop {
                SpanOp::AddSpan { .. } => stats.add_spans += 1,
                SpanOp::RemoveSpan { .. } => stats.remove_spans += 1,
            }
            for bias in [bias_s, bias_e] {
                match bias {
                    Bias::Before => stats.before_anchors += 1,
                    Bias::After => stats.after_anchors += 1,
                }
            }
            let side = if on_b { &mut spans.b } else { &mut spans.a };
            record_span_into(side, &o);
            if on_b {
                stats.b_local_spans += 1;
            }
            let (sa, ea) = sop.anchors();
            let co = if on_b {
                b.seq_checkout()
            } else {
                a.seq_checkout()
            };
            let s_res = co.resolve_boundary(sa.id, sa.bias.into());
            let e_res = co.resolve_boundary(ea.id, ea.bias.into());
            if s_res.is_some_and(|r| !r.visible) || e_res.is_some_and(|r| !r.visible) {
                stats.tombstone_anchor_spans += 1;
            }
            if let (Some(sr), Some(er)) = (s_res, e_res)
                && sr.position >= er.position
            {
                stats.reversed_spans += 1;
            }
        }
        on_boundary(&a, &b, &spans);
    }

    a.commit();
    b.commit();
    deliver(
        &mut a,
        &b,
        true,
        &mut live_a,
        &mut spans,
        &mut stats,
        false,
        false,
        on_boundary,
    );
    deliver(
        &mut b,
        &a,
        false,
        &mut live_b,
        &mut spans,
        &mut stats,
        false,
        false,
        on_boundary,
    );

    CorpusRun { a, b, spans, stats }
}

pub fn assert_matches_cold_rebuild(warm: &ProjectedState) {
    let cold = ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
    assert_eq!(
        warm.projected(),
        cold.projected(),
        "warm/cold projection diverged"
    );
    warm.assert_seg_index_matches_logs();
    warm.assert_anchor_index_matches_logs();
    cold.assert_anchor_index_matches_logs();
}

pub fn mandatory_prefix() -> Vec<CorpusStep> {
    vec![
        (0, 0, 0, 0, 0),
        (0, 1, 0, 0, 1),
        (0, 2, 0, 0, 2),
        (12, 0, 0, 0, 3),
        (13, 0, 0, 0, 4),
        (13, 0, 2, 0, 5),
        (7, 0, 1, 0, 6),
        (8, 1, 2, 3, 7),
        (8, 3, 0, 0, 8),
        (5, 1, 1, 0, 9),
        (7, 1, 2, 1, 10),
        (10, 0, 1, 2, 11),
        (6, 0, 0, 0, 12),
        (11, 0, 0, 0, 13),
        (11, 0, 0, 0, 14),
        (0, 1, 0, 0, 15),
        (0, 2, 0, 0, 16),
        (11, 0, 1, 2, 17),
        (0, 2, 0, 0, 18),
        (8, 0, 1, 0, 19),
        (11, 0, 0, 0, 20),
    ]
}

/// Decision-test fixture: concrete corpus step sequences known, by direct
/// harvest under instrumentation, to exercise [`ProjectedState`]'s window
/// escalation loop at least once (`window_escalations` observed to increase).
/// Each entry is a full step sequence — [`mandatory_prefix`] plus the
/// harvested tail — ready to hand to [`run_corpus`].
pub fn harvested_escalation_seeds() -> Vec<Vec<CorpusStep>> {
    let tails: [&[CorpusStep]; 3] = [
        &[
            (140, 157, 238, 9, 67),
            (66, 12, 49, 21, 121),
            (141, 244, 114, 51, 207),
            (3, 41, 242, 71, 186),
        ],
        &[
            (156, 43, 245, 31, 55),
            (168, 11, 23, 88, 69),
            (71, 12, 23, 187, 49),
            (149, 189, 184, 186, 235),
            (83, 10, 238, 32, 205),
        ],
        &[
            (235, 31, 198, 236, 113),
            (240, 215, 116, 222, 183),
            (54, 192, 118, 46, 84),
            (104, 23, 166, 56, 52),
            (35, 212, 167, 177, 254),
            (37, 38, 200, 56, 33),
            (246, 167, 155, 37, 50),
            (71, 18, 128, 252, 152),
            (0, 152, 132, 80, 183),
            (12, 81, 205, 234, 117),
            (255, 63, 48, 187, 149),
            (170, 201, 89, 27, 149),
        ],
    ];
    tails
        .into_iter()
        .map(|tail| {
            let mut steps = mandatory_prefix();
            steps.extend_from_slice(tail);
            steps
        })
        .collect()
}

// The prefix walks, in order: three chars on A; fold / bullet-list /
// blockquote templates; spans with both bias combinations plus a strictly
// reversed one; a deterministic delete of live[1..=2] followed by a span
// anchored on those tombstones; RemoveSpan; the consuming undelete; two
// syncs that seal A-cs1 WITHOUT delivering it to B (direction a<-b); a
// second A batch sealing A-cs2; a PARTIAL b<-a delivery of {cs1, cs2}
// (drops cs2 — real partial, replicas stay divergent); B-side edits with a
// span anchored on chars received from A; and a final a<-b hand-back so A
// records B's span while still withholding cs2.

pub fn bold_label_fold_list() -> ProjectedState {
    let mut s = ProjectedState::empty();
    let mut pos = 1;
    let fold = s
        .apply(seq_block(pos, NodeType::Fold, vec![Dot::ROOT]))
        .unwrap()
        .id;
    pos += 1;
    s.apply(seq_block(pos, NodeType::FoldTitle, vec![Dot::ROOT, fold]))
        .unwrap();
    pos += 1;
    s.apply(seq_char(pos, 't')).unwrap();
    pos += 1;
    let content = s
        .apply(seq_block(pos, NodeType::FoldContent, vec![Dot::ROOT, fold]))
        .unwrap()
        .id;
    pos += 1;
    let list = s
        .apply(seq_block(
            pos,
            NodeType::BulletList,
            vec![Dot::ROOT, fold, content],
        ))
        .unwrap()
        .id;
    pos += 1;
    let mut label_chars: Vec<Dot> = Vec::new();
    for i in 0..8 {
        let item = s
            .apply(seq_block(
                pos,
                NodeType::ListItem,
                vec![Dot::ROOT, fold, content, list],
            ))
            .unwrap()
            .id;
        pos += 1;
        s.apply(seq_block(
            pos,
            NodeType::Paragraph,
            vec![Dot::ROOT, fold, content, list, item],
        ))
        .unwrap();
        pos += 1;
        let mut chars = Vec::new();
        for k in 0..6 {
            let d = s
                .apply(seq_char(pos, char::from(b'a' + ((i + k) % 26) as u8)))
                .unwrap()
                .id;
            pos += 1;
            chars.push(d);
        }
        if i % 2 == 0 {
            label_chars = chars;
            s.apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: label_chars[0],
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: *label_chars.last().unwrap(),
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();
        }
    }
    s
}

pub fn span_stress(n: usize) -> ProjectedState {
    span_stress_sized(200, n)
}

pub fn span_stress_sized(chars_len: usize, n: usize) -> ProjectedState {
    let mut s = ProjectedState::empty();
    let mut chars: Vec<Dot> = Vec::new();
    for i in 0..chars_len {
        let d = s
            .apply(seq_char(1 + i, char::from(b'a' + (i % 26) as u8)))
            .unwrap()
            .id;
        chars.push(d);
    }
    for i in 0..n {
        let a = chars[(i * 7) % chars.len()];
        let b = chars[(i * 11 + 3) % chars.len()];
        s.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: a,
                bias: Bias::Before,
            },
            end: Anchor {
                id: b,
                bias: Bias::After,
            },
            modifier: if i % 2 == 0 {
                Modifier::Bold
            } else {
                Modifier::Italic
            },
        }))
        .unwrap();
    }
    s
}

pub fn tombstone_cluster_anchors() -> ProjectedState {
    let mut s = ProjectedState::empty();
    let mut chars: Vec<Dot> = Vec::new();
    for i in 0..12 {
        let d = s
            .apply(seq_char(1 + i, char::from(b'a' + (i % 26) as u8)))
            .unwrap()
            .id;
        chars.push(d);
    }
    s.apply(EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: chars[3],
            bias: Bias::Before,
        },
        end: Anchor {
            id: chars[7],
            bias: Bias::After,
        },
        modifier: Modifier::Bold,
    }))
    .unwrap();
    s.apply(EditOp::Seq(ListOp::Del { pos: 4, len: 4 }))
        .unwrap();
    s.apply(EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: chars[4],
            bias: Bias::After,
        },
        end: Anchor {
            id: chars[9],
            bias: Bias::Before,
        },
        modifier: Modifier::Italic,
    }))
    .unwrap();
    s
}

pub fn concurrent_delete_remote_span() -> ProjectedState {
    let mut a = ProjectedState::empty();
    let mut chars: Vec<Dot> = Vec::new();
    for i in 0..6 {
        chars.push(
            a.apply(seq_char(1 + i, char::from(b'a' + i as u8)))
                .unwrap()
                .id,
        );
    }
    a.commit();
    let mut b = ProjectedState::from_graph(OpGraph::with_actor(2)).expect("empty graph projects");
    let heads: HashSet<Dot> = b.graph().current_heads().copied().collect();
    for cs in a.graph().missing_changesets_tolerant(&heads) {
        let (next, _) = b.receive_changesets(vec![cs]).unwrap();
        b = next;
    }
    a.apply(EditOp::Seq(ListOp::Del { pos: 3, len: 2 }))
        .unwrap();
    a.commit();
    b.apply(EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: chars[2],
            bias: Bias::Before,
        },
        end: Anchor {
            id: chars[4],
            bias: Bias::After,
        },
        modifier: Modifier::Bold,
    }))
    .unwrap();
    b.commit();
    let heads: HashSet<Dot> = a.graph().current_heads().copied().collect();
    for cs in b.graph().missing_changesets_tolerant(&heads) {
        let (next, _) = a.receive_changesets(vec![cs]).unwrap();
        a = next;
    }
    a
}

pub fn mixed_atoms() -> ProjectedState {
    let mut s = ProjectedState::empty();
    let mut chars: Vec<Dot> = Vec::new();
    let mut pos = 1;
    for i in 0..6u8 {
        chars.push(s.apply(seq_char(pos, char::from(b'a' + i))).unwrap().id);
        pos += 1;
    }
    s.apply(EditOp::Seq(ListOp::Ins {
        pos,
        item: SeqItem::Atom(AtomLeaf::HardBreak),
    }))
    .unwrap();
    pos += 1;
    for i in 6..9u8 {
        chars.push(s.apply(seq_char(pos, char::from(b'a' + i))).unwrap().id);
        pos += 1;
    }
    let img = match NodeType::Image.into_node() {
        Node::Image(n) => n,
        _ => unreachable!(),
    };
    s.apply(EditOp::Seq(ListOp::Ins {
        pos,
        item: SeqItem::BlockAtom {
            leaf: AtomLeaf::Image { node: img },
            parents: vec![Dot::ROOT],
        },
    }))
    .unwrap();
    s.apply(EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: chars[4],
            bias: Bias::Before,
        },
        end: Anchor {
            id: chars[7],
            bias: Bias::After,
        },
        modifier: Modifier::Bold,
    }))
    .unwrap();
    s
}

#[cfg(test)]
mod tests {
    use editor_model::{AnchorIntervalIndex, spans_covering};

    use super::*;

    fn index_of(state: &ProjectedState, spans: &[(Anchor, Anchor, Dot)]) -> AnchorIntervalIndex {
        let co = state.seq_checkout();
        let mut idx = AnchorIntervalIndex::build(co, spans.iter().copied());
        idx.flush_pending(co);
        idx
    }

    fn assert_stab_matches_naive(state: &ProjectedState, idx: &AnchorIntervalIndex) {
        let co = state.seq_checkout();
        let visible = co.visible_len();
        let stride = (visible / 64).max(1);
        for p in (0..visible).step_by(stride) {
            let got = idx.stab(co, p);
            let want = spans_covering(p, state.spans(), co);
            assert_eq!(got, want, "stab/naive diverge at pos {p}");
        }
    }

    fn naive_intersecting_spans(state: &ProjectedState, lo: usize, hi: usize) -> Vec<Dot> {
        let co = state.seq_checkout();
        let mut out: Vec<Dot> = state
            .spans()
            .iter()
            .filter_map(|(d, op)| {
                let (sa, ea) = op.anchors();
                let s = co.resolve_boundary(sa.id, sa.bias.into())?.position;
                let e = co.resolve_boundary(ea.id, ea.bias.into())?.position;
                (s < e && s < hi && e > lo).then_some(*d)
            })
            .collect();
        out.sort();
        out
    }

    fn assert_intersecting_matches_naive(state: &ProjectedState, idx: &AnchorIntervalIndex) {
        let co = state.seq_checkout();
        let visible = co.visible_len().max(1);
        for (lo, hi) in [
            (0, visible),
            (visible / 3, (2 * visible) / 3),
            (visible / 2, (visible / 2 + 4).min(visible)),
        ] {
            let got = idx.intersecting(co, lo, hi);
            let want = naive_intersecting_spans(state, lo, hi);
            assert_eq!(got, want, "intersecting/naive diverge on [{lo}, {hi})");
        }
    }

    fn all_span_intervals(state: &ProjectedState) -> Vec<(Anchor, Anchor, Dot)> {
        state
            .spans()
            .iter()
            .map(|(d, op)| {
                let (s, e) = op.anchors();
                (s, e, *d)
            })
            .collect()
    }

    #[test]
    fn stab_matches_naive_on_named_scenarios() {
        for state in [
            bold_label_fold_list(),
            span_stress(2000),
            tombstone_cluster_anchors(),
            concurrent_delete_remote_span(),
            mixed_atoms(),
        ] {
            let idx = index_of(&state, &all_span_intervals(&state));
            assert_stab_matches_naive(&state, &idx);
            assert_intersecting_matches_naive(&state, &idx);
        }
    }

    #[test]
    fn removal_excludes_only_the_removed_span() {
        let state = span_stress(300);
        let co = state.seq_checkout();
        let all = all_span_intervals(&state);
        let (s, _e, d) = all[all.len() / 2];
        let mut idx = index_of(&state, &all);
        assert!(idx.remove(co, &s, &d));
        let visible = co.visible_len();
        for p in (0..visible).step_by(3) {
            let mut want = spans_covering(p, state.spans(), co);
            want.retain(|x| *x != d);
            assert_eq!(idx.stab(co, p), want, "post-removal stab at {p}");
        }
    }

    #[test]
    #[ignore]
    fn perf_index_scaling() {
        use std::time::Instant;
        for n in [2_000usize, 20_000] {
            let mut state = ProjectedState::empty();
            let mut chars: Vec<Dot> = Vec::new();
            for i in 0..n {
                chars.push(
                    state
                        .apply(seq_char(1 + i, char::from(b'a' + (i % 26) as u8)))
                        .unwrap()
                        .id,
                );
            }
            for k in 0..n {
                let a = (k * 7) % (n - 16);
                let width = 1 + k % 16;
                state
                    .apply(EditOp::Span(SpanOp::AddSpan {
                        start: Anchor {
                            id: chars[a],
                            bias: Bias::Before,
                        },
                        end: Anchor {
                            id: chars[a + width],
                            bias: Bias::After,
                        },
                        modifier: if k % 2 == 0 {
                            Modifier::Bold
                        } else {
                            Modifier::Italic
                        },
                    }))
                    .unwrap();
            }
            let co = state.seq_checkout();
            let t = Instant::now();
            let mut idx = AnchorIntervalIndex::build(co, all_span_intervals(&state));
            idx.flush_pending(co);
            let build = t.elapsed();
            let visible = co.visible_len();
            let queries: Vec<usize> = (0..64).map(|q| (q * visible) / 64).collect();
            let mut hits = 0usize;
            let t = Instant::now();
            for _ in 0..32 {
                for &p in &queries {
                    hits += idx.stab(co, p).len();
                }
            }
            let stab = t.elapsed();
            let t = Instant::now();
            for _ in 0..32 {
                for &p in &queries {
                    hits += idx.intersecting(co, p, (p + 16).min(visible)).len();
                }
            }
            let inter = t.elapsed();
            eprintln!(
                "n={n}: build {build:.2?}, stab x{} {stab:.2?}, intersecting x{} {inter:.2?}, avg hits/query {:.1}",
                64 * 32,
                64 * 32,
                hits as f64 / (2.0 * 64.0 * 32.0)
            );
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: std::env::var("PROPTEST_CASES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(128),
            ..proptest::prelude::ProptestConfig::default()
        })]
        #[test]
        fn stab_matches_naive_at_every_corpus_boundary(
            suffix in proptest::collection::vec(proptest::prelude::any::<CorpusStep>(), 0..48),
        ) {
            let mut steps = mandatory_prefix();
            steps.extend(suffix);
            let mut idx_a = AnchorIntervalIndex::new();
            let mut idx_b = AnchorIntervalIndex::new();
            let mut seen_a = 0usize;
            let mut seen_b = 0usize;
            let run = run_corpus(&steps, &mut |a, b, spans| {
                for (state, idx, seen, side) in [
                    (a, &mut idx_a, &mut seen_a, &spans.a),
                    (b, &mut idx_b, &mut seen_b, &spans.b),
                ] {
                    let co = state.seq_checkout();
                    for (s, e, d) in &side[*seen..] {
                        let _ = idx.insert(co, *s, *e, *d);
                    }
                    *seen = side.len();
                    idx.flush_pending(co);
                    let visible = co.visible_len();
                    let stride = (visible / 32).max(1);
                    for p in (0..visible).step_by(stride) {
                        let got = idx.stab(co, p);
                        let want = spans_covering(p, state.spans(), co);
                        assert_eq!(got, want, "stab/naive diverge at pos {p}");
                    }
                    assert_intersecting_matches_naive(state, idx);
                }
            });

            for (state, idx_owned, side) in [
                (&run.a, idx_a, &run.spans.a),
                (&run.b, idx_b, &run.spans.b),
            ] {
                let co = state.seq_checkout();
                let mut idx = idx_owned;
                let mut removed: std::collections::BTreeSet<Dot> = Default::default();
                let mut remaining = side.clone();
                let mut k = 0usize;
                while k < 8 && !remaining.is_empty() {
                    let pick = (k * 37 + 11) % remaining.len();
                    let (s, _e, d) = remaining.remove(pick);
                    proptest::prop_assert!(idx.remove(co, &s, &d), "indexed span must be removable");
                    removed.insert(d);
                    let visible = co.visible_len();
                    let stride = (visible / 16).max(1);
                    for p in (0..visible).step_by(stride) {
                        let mut want = spans_covering(p, state.spans(), co);
                        want.retain(|x| !removed.contains(x));
                        proptest::prop_assert_eq!(idx.stab(co, p), want, "post-removal stab at {}", p);
                    }
                    k += 1;
                }
            }
        }
    }

    #[test]
    fn corpus_run_converges_replicas_on_final_sync() {
        let mut steps = mandatory_prefix();
        steps.extend((0u8..60).map(|i| (i, i.wrapping_mul(7), i.wrapping_mul(13), i % 4, i % 26)));
        let mut calls = 0usize;
        let run = run_corpus(&steps, &mut |_, _, _| calls += 1);
        assert!(calls > steps.len() / 2, "boundaries fire for applied ops");
        assert_eq!(
            run.a.projected(),
            run.b.projected(),
            "final bidirectional sync must converge"
        );
    }

    #[test]
    fn corpus_applies_span_ops_and_reports_them() {
        let mut steps = mandatory_prefix();
        steps.extend((0u8..40).map(|i| {
            if i % 3 == 0 {
                (8, i, i, i % 4, 0)
            } else {
                (0, i, i, 0, i % 26)
            }
        }));
        let run = run_corpus(&steps, &mut |_, _, _| {});
        assert!(!run.spans.a.is_empty());
        for (_, _, d) in &run.spans.a {
            assert!(run.a.spans().get(*d).is_some());
        }
    }

    #[test]
    fn corpus_redelivery_applies_exactly_once() {
        let mut steps = mandatory_prefix();
        steps.push((11, 0, 1, 1, 0));
        let run = run_corpus(&steps, &mut |a, b, _| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
        assert!(
            run.stats.redeliveries >= 1,
            "the explicit redelivery step must actually redeliver"
        );
    }

    fn straddle_fixture(chars: usize) -> (ProjectedState, ProjectedState) {
        let mut a = ProjectedState::empty();
        let mut ids = Vec::new();
        for i in 0..chars {
            ids.push(
                a.apply(seq_char(1 + i, char::from(b'a' + (i % 26) as u8)))
                    .unwrap()
                    .id,
            );
        }
        a.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: ids[0],
                bias: Bias::Before,
            },
            end: Anchor {
                id: *ids.last().unwrap(),
                bias: Bias::After,
            },
            modifier: Modifier::Bold,
        }))
        .unwrap();
        a.commit();
        let b = ProjectedState::from_graph(OpGraph::with_actor(2)).expect("empty graph projects");
        let heads: HashSet<Dot> = b.graph().current_heads().copied().collect();
        let css = a.graph().missing_changesets_tolerant(&heads);
        let (b, _) = b.receive_changesets(css).expect("initial delivery applies");
        (a, b)
    }

    fn deliver_char_and_span(mut a: ProjectedState, b: ProjectedState) -> ProjectedState {
        let pos = a.seq_checkout().visible_len();
        let c = a.apply(seq_char(pos, 'z')).unwrap().id;
        a.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: c,
                bias: Bias::Before,
            },
            end: Anchor {
                id: c,
                bias: Bias::After,
            },
            modifier: Modifier::Italic,
        }))
        .unwrap();
        a.commit();
        let heads: HashSet<Dot> = b.graph().current_heads().copied().collect();
        let css = a.graph().missing_changesets_tolerant(&heads);
        let novel: usize = css.iter().map(|cs| cs.ops.len()).sum();
        assert_eq!(novel, 2, "exactly one char + one span must be novel");
        let (b, _) = b.receive_changesets(css).expect("second delivery applies");
        b
    }

    #[test]
    fn bulk_threshold_exact_boundary_stays_incremental() {
        use crate::projected_state::BULK_NOVEL_FACTOR;
        let (a, b) = straddle_fixture(2 * BULK_NOVEL_FACTOR - 1);
        assert_eq!(
            b.seq_checkout().visible_len(),
            2 * BULK_NOVEL_FACTOR,
            "fixture accounting drifted — adjust the char count"
        );
        let before = b.incremental_leaf_inserts;
        let reprojects_before = b.full_reprojects;
        let b = deliver_char_and_span(a, b);
        assert_eq!(
            b.incremental_leaf_inserts,
            before + 1,
            "novel×factor == visible must stay incremental"
        );
        assert_eq!(
            b.full_reprojects, reprojects_before,
            "the incremental leg must not reproject"
        );
        assert_matches_cold_rebuild(&b);
    }

    #[test]
    fn bulk_threshold_one_past_boundary_goes_bulk() {
        use crate::projected_state::BULK_NOVEL_FACTOR;
        let (a, b) = straddle_fixture(2 * BULK_NOVEL_FACTOR - 2);
        assert_eq!(
            b.seq_checkout().visible_len(),
            2 * BULK_NOVEL_FACTOR - 1,
            "fixture accounting drifted — adjust the char count"
        );
        let before = b.incremental_leaf_inserts;
        let reprojects_before = b.full_reprojects;
        let b = deliver_char_and_span(a, b);
        assert_eq!(
            b.incremental_leaf_inserts, before,
            "novel×factor > visible must take the bulk path"
        );
        assert_eq!(
            b.full_reprojects,
            reprojects_before + 1,
            "the bulk leg must reproject exactly once"
        );
        assert_matches_cold_rebuild(&b);
    }

    #[test]
    fn cloned_states_diverge_independently() {
        let mut base = ProjectedState::empty();
        let mut chars = Vec::new();
        for i in 0..12 {
            chars.push(
                base.apply(seq_char(1 + i, char::from(b'a' + i as u8)))
                    .unwrap()
                    .id,
            );
        }
        for k in 0..6 {
            base.apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: chars[k],
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: chars[k + 4],
                    bias: Bias::After,
                },
                modifier: if k % 2 == 0 {
                    Modifier::Bold
                } else {
                    Modifier::Italic
                },
            }))
            .unwrap();
        }
        let mut left = base.clone();
        let mut right = base;
        left.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: chars[0],
                bias: Bias::Before,
            },
            end: Anchor {
                id: chars[3],
                bias: Bias::After,
            },
            modifier: Modifier::Bold,
        }))
        .unwrap();
        right
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: chars[8],
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: chars[11],
                    bias: Bias::After,
                },
                modifier: Modifier::Italic,
            }))
            .unwrap();
        right
            .apply(EditOp::Seq(ListOp::Del { pos: 2, len: 2 }))
            .unwrap();
        assert_matches_cold_rebuild(&left);
        assert_matches_cold_rebuild(&right);
    }

    #[test]
    fn mandatory_prefix_exercises_required_categories() {
        let run = run_corpus(&mandatory_prefix(), &mut |_, _, _| {});
        assert!(
            run.stats.partial_deliveries >= 1,
            "a real partial delivery ran"
        );
        assert!(run.stats.undeletes >= 1, "the consuming undelete ran");
        assert!(run.stats.b_local_spans >= 1, "B authored a span");
        assert!(
            run.stats.tombstone_anchor_spans >= 1,
            "a span anchored on tombstoned chars"
        );
        assert!(
            run.stats.reversed_spans >= 1,
            "a reversed/empty span was applied"
        );
        assert!(
            run.spans.a.len() >= 4,
            "A accumulated local + received spans"
        );
        assert!(
            !run.spans.b.is_empty(),
            "B accumulated local + received spans"
        );
        assert!(run.stats.b_local_edits >= 1, "B edited locally");
        assert!(run.stats.add_spans >= 1, "an AddSpan ran");
        assert!(run.stats.remove_spans >= 1, "a RemoveSpan ran");
        assert!(run.stats.before_anchors >= 1, "a Before-bias anchor ran");
        assert!(run.stats.after_anchors >= 1, "an After-bias anchor ran");
        assert!(run.stats.fold_templates >= 1, "a fold template ran");
        assert!(run.stats.list_templates >= 1, "a list template ran");
        assert!(run.stats.wrap_templates >= 1, "a wrap template ran");
    }

    #[test]
    fn table_template_projects_and_matches_cold() {
        let mut steps = mandatory_prefix();
        steps.push((12, 1, 0, 0, 7));
        let run = run_corpus(&steps, &mut |a, b, _| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
        assert!(run.stats.table_templates >= 1, "the table template ran");
        assert_table_grid_padded(&run.a);
    }

    fn assert_table_grid_padded(state: &ProjectedState) {
        let tree = &state.projected().tree;
        let mut found = false;
        let mut stack = vec![tree.root_id()];
        while let Some(id) = stack.pop() {
            let Some(node) = tree.get(id) else { continue };
            if node.node_type == NodeType::Table {
                let widths: Vec<usize> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        Child::Block(r) => tree.get(*r),
                        Child::Leaf { .. } => None,
                    })
                    .filter(|r| r.node_type == NodeType::TableRow)
                    .map(|r| {
                        r.children
                            .iter()
                            .filter(|c| match c {
                                Child::Block(b) => tree
                                    .get(*b)
                                    .is_some_and(|n| n.node_type == NodeType::TableCell),
                                Child::Leaf { .. } => false,
                            })
                            .count()
                    })
                    .collect();
                if widths.len() >= 2 && widths.iter().all(|w| *w == widths[0]) {
                    found = true;
                }
            }
            for c in node.children.iter() {
                if let Child::Block(b) = c {
                    stack.push(*b);
                }
            }
        }
        assert!(
            found,
            "a projected table with >=2 equal-width rows (grid padding applied) must exist"
        );
    }

    fn assert_projected_type_at_depth(
        state: &ProjectedState,
        node_type: NodeType,
        min_depth: usize,
    ) {
        let tree = &state.projected().tree;
        let mut stack: Vec<(Dot, usize)> = vec![(tree.root_id(), 0)];
        while let Some((id, depth)) = stack.pop() {
            let Some(node) = tree.get(id) else { continue };
            if node.node_type == node_type && depth >= min_depth {
                return;
            }
            for c in node.children.iter() {
                if let Child::Block(b) = c {
                    stack.push((*b, depth + 1));
                }
            }
        }
        panic!("no {node_type:?} at depth >= {min_depth} in the projection");
    }

    #[test]
    fn nested_fold_template_matches_cold() {
        let mut steps = mandatory_prefix();
        steps.push((12, 2, 0, 0, 9));
        let run = run_corpus(&steps, &mut |a, b, _| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
        assert!(run.stats.nested_fold_templates >= 1);
        assert_projected_type_at_depth(&run.a, NodeType::Fold, 2);
    }

    #[test]
    fn nested_attach_template_matches_cold() {
        let mut steps = mandatory_prefix();
        steps.push((12, 2, 0, 0, 9));
        steps.push((13, 0, 2, 200, 5));
        let run = run_corpus(&steps, &mut |a, b, _| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
        assert!(run.stats.nested_attach_templates >= 1);
        assert_projected_type_at_depth(&run.a, NodeType::Blockquote, 2);
    }

    #[test]
    fn container_kill_steps_match_cold() {
        for x in [200u8, 201] {
            let mut steps = mandatory_prefix();
            steps.push((12, 0, 0, 0, 3));
            steps.push((13, x, 0, 0, 0));
            let run = run_corpus(&steps, &mut |a, b, _| {
                assert_matches_cold_rebuild(a);
                assert_matches_cold_rebuild(b);
            });
            assert!(run.stats.container_kill_steps >= 1, "x={x}");
        }
    }

    #[test]
    fn table_width_recompute_on_cell_delete_matches_cold() {
        let mut s = ProjectedState::empty();
        let mut pos = 1;
        let table = s
            .apply(seq_block(pos, NodeType::Table, vec![Dot::ROOT]))
            .unwrap()
            .id;
        pos += 1;
        let row1 = s
            .apply(seq_block(pos, NodeType::TableRow, vec![Dot::ROOT, table]))
            .unwrap()
            .id;
        pos += 1;
        let c11 = s
            .apply(seq_block(
                pos,
                NodeType::TableCell,
                vec![Dot::ROOT, table, row1],
            ))
            .unwrap()
            .id;
        pos += 1;
        s.apply(seq_block(
            pos,
            NodeType::Paragraph,
            vec![Dot::ROOT, table, row1, c11],
        ))
        .unwrap();
        pos += 1;
        let row2 = s
            .apply(seq_block(pos, NodeType::TableRow, vec![Dot::ROOT, table]))
            .unwrap()
            .id;
        pos += 1;
        let c21 = s
            .apply(seq_block(
                pos,
                NodeType::TableCell,
                vec![Dot::ROOT, table, row2],
            ))
            .unwrap()
            .id;
        pos += 1;
        s.apply(seq_block(
            pos,
            NodeType::Paragraph,
            vec![Dot::ROOT, table, row2, c21],
        ))
        .unwrap();
        pos += 1;
        let c22 = s
            .apply(seq_block(
                pos,
                NodeType::TableCell,
                vec![Dot::ROOT, table, row2],
            ))
            .unwrap()
            .id;
        pos += 1;
        s.apply(seq_block(
            pos,
            NodeType::Paragraph,
            vec![Dot::ROOT, table, row2, c22],
        ))
        .unwrap();
        assert_matches_cold_rebuild(&s);
        let del_pos = s
            .seq_checkout()
            .resolve_boundary(c22, Bias::Before.into())
            .unwrap()
            .position;
        s.apply(EditOp::Seq(ListOp::Del {
            pos: del_pos,
            len: 2,
        }))
        .unwrap();
        assert_matches_cold_rebuild(&s);
    }

    #[test]
    fn named_scenarios_project() {
        let a = bold_label_fold_list();
        assert!(a.spans().iter().count() >= 1);
        let b = span_stress(500);
        assert_eq!(b.spans().iter().count(), 500);
        let c = tombstone_cluster_anchors();
        assert!(c.spans().iter().count() >= 2);
        let d = concurrent_delete_remote_span();
        assert!(d.spans().iter().count() >= 1);
        let e = mixed_atoms();
        assert!(e.spans().iter().count() >= 1);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: std::env::var("PROPTEST_CASES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
            ..proptest::prelude::ProptestConfig::default()
        })]
        #[test]
        fn warm_matches_cold_at_every_corpus_boundary(
            suffix in proptest::collection::vec(proptest::prelude::any::<CorpusStep>(), 0..32),
        ) {
            let mut steps = mandatory_prefix();
            steps.extend(suffix);
            run_corpus(&steps, &mut |a, b, _spans| {
                assert_matches_cold_rebuild(a);
                assert_matches_cold_rebuild(b);
                editor_model::assert_flat_index_consistent(&a.projected().tree);
                editor_model::assert_flat_index_consistent(&b.projected().tree);
            });
        }
    }

    #[test]
    fn container_kill_undelete_reparents_hoisted_sibling_matches_cold() {
        let mut steps = mandatory_prefix();
        steps.extend([
            (13u8, 0u8, 66u8, 0u8, 0u8),
            (28, 115, 0, 0, 0),
            (42, 138, 0, 0, 0),
            (84, 75, 0, 0, 0),
            (14, 113, 0, 0, 0),
            (14, 182, 0, 0, 0),
            (132, 0, 0, 0, 0),
            (187, 9, 244, 0, 0),
            (41, 0, 38, 0, 0),
            (13, 215, 0, 0, 0),
            (34, 0, 0, 0, 0),
        ]);
        run_corpus(&steps, &mut |a, b, _spans| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
    }

    #[test]
    fn remote_insert_after_tombstoned_end_anchor_matches_cold() {
        let mut steps = mandatory_prefix();
        steps.extend([
            (42u8, 8u8, 0u8, 0u8, 0u8),
            (70, 181, 0, 0, 0),
            (14, 22, 0, 0, 0),
            (221, 0, 19, 4, 0),
            (41, 0, 58, 0, 0),
            (133, 54, 45, 0, 0),
            (41, 0, 146, 0, 0),
            (193, 0, 0, 0, 0),
            (42, 11, 0, 0, 0),
            (221, 0, 62, 0, 0),
            (14, 239, 0, 0, 0),
            (243, 39, 62, 0, 0),
            (0, 0, 0, 0, 0),
        ]);
        let run = run_corpus(&steps, &mut |a, b, _| {
            assert_matches_cold_rebuild(a);
            assert_matches_cold_rebuild(b);
        });
        assert!(run.a.incremental_leaf_inserts > 0);
        assert!(run.b.incremental_leaf_inserts > 0);
    }
}
