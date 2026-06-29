use crate::Dot;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, editor_macros::Wire)]
pub enum ListOp<P = char> {
    #[wire(n(0))]
    Ins {
        #[wire(n(0))]
        pos: usize,
        #[wire(n(1))]
        item: P,
    },
    #[wire(n(1))]
    Del {
        #[wire(n(0))]
        pos: usize,
        #[wire(n(1))]
        len: usize,
    },
    #[wire(n(2))]
    Undel {
        #[wire(n(0))]
        del: Dot,
    },
}

#[derive(Clone, Debug)]
pub struct InputEvent<P = char> {
    pub id: Dot,
    pub parents: Vec<Dot>,
    pub op: ListOp<P>,
}

#[derive(Clone, Debug)]
pub struct OpLog<P = char> {
    pub ops: Vec<ListOp<P>>,
    pub dots: Vec<Dot>,
    pub parents: Vec<Vec<usize>>,
    pub lv_of: HashMap<Dot, usize>,
}

pub(crate) const NYI: i32 = -1;
pub(crate) fn item_width(state: i32) -> usize {
    if state == 0 { 1 } else { 0 }
}

pub fn build_oplog<P: Clone>(events: &[InputEvent<P>]) -> OpLog<P> {
    let by_dot: HashMap<Dot, &InputEvent<P>> = events.iter().map(|e| (e.id, e)).collect();
    enum F {
        Enter(Dot),
        Emit(Dot),
    }
    let mut roots: Vec<Dot> = events.iter().map(|e| e.id).collect();
    roots.sort();
    let mut stack: Vec<F> = roots.into_iter().rev().map(F::Enter).collect();
    let mut seen = HashSet::new();
    let mut order: Vec<Dot> = Vec::new();
    while let Some(f) = stack.pop() {
        match f {
            F::Enter(d) => {
                if !seen.insert(d) {
                    continue;
                }
                let e = by_dot[&d];
                stack.push(F::Emit(d));
                let mut ps = e.parents.clone();
                ps.sort();
                for p in ps.into_iter().rev() {
                    stack.push(F::Enter(p));
                }
            }
            F::Emit(d) => order.push(d),
        }
    }
    let lv: HashMap<Dot, usize> = order.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let mut ops = Vec::new();
    let mut dots = Vec::new();
    let mut parents = Vec::new();
    for d in &order {
        let e = by_dot[d];
        ops.push(e.op.clone());
        dots.push(*d);
        parents.push(e.parents.iter().map(|p| lv[p]).collect());
    }
    OpLog {
        ops,
        dots,
        parents,
        lv_of: lv,
    }
}

pub(crate) fn lv_cmp<P>(log: &OpLog<P>, a: usize, b: usize) -> std::cmp::Ordering {
    let da = &log.dots[a];
    let db = &log.dots[b];
    da.actor.cmp(&db.actor).then(da.clock.cmp(&db.clock))
}
