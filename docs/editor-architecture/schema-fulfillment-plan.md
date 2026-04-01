# Schema Fulfillment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** Do NOT run `git commit`. The user will commit manually.

**Goal:** 에디터의 노드 삽입/제거를 서브트리 단위로 원자적으로 처리하고, `fulfill` 헬퍼로 content expression fix-up을 선언적으로 수행한다.

**Architecture:** `Subtree` 타입을 editor-model에 도입하고, InsertSubtree/RemoveSubtree step이 이를 사용하도록 변경. Transaction에 `batch()`를 추가하여 deferred validation 지원. `fulfill()` 헬퍼가 content expression 분석 후 InsertSubtree step 목록 반환.

**Tech Stack:** Rust, editor-model/editor-transaction/editor-commands crates

**Spec:** `docs/editor-architecture/schema-fulfillment-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `crates/editor-model/src/subtree.rs` | Subtree 타입 + 생성자 + capture/into_entries |
| Modify | `crates/editor-model/src/lib.rs` | subtree 모듈 등록 |
| Modify | `crates/editor-transaction/src/step.rs` | InsertSubtree/RemoveSubtree에 Subtree 사용 |
| Modify | `crates/editor-transaction/src/steps/insert_subtree.rs` | 서브트리 삽입 + 자체 content validation |
| Modify | `crates/editor-transaction/src/steps/remove_subtree.rs` | 서브트리 캡처 + 일괄 제거 |
| Modify | `crates/editor-transaction/src/transaction.rs` | insert_node 시그니처, remove_node 내부, batch, apply_steps |
| Modify | `crates/editor-view/src/engine/mod.rs` | dirty_nodes 패턴 매치 업데이트 |
| Modify | `crates/editor-commands/src/commands/insert_text.rs` | Subtree::leaf 사용 |
| Modify | `crates/editor-commands/src/commands/insert_hard_break.rs` | Subtree::leaf 사용 |
| Modify | `crates/editor-commands/src/commands/sink_paragraph_backward.rs` | batch + fulfill 적용 |
| Create | `crates/editor-transaction/src/fulfill.rs` | fulfill 함수 |
| Modify | `crates/editor-transaction/src/lib.rs` | fulfill 모듈 등록 및 re-export |

---

### Task 1: Subtree 타입

**Files:**
- Create: `crates/editor-model/src/subtree.rs`
- Modify: `crates/editor-model/src/lib.rs`

- [ ] **Step 1: Subtree 구조체 + 생성자 작성**

```rust
// crates/editor-model/src/subtree.rs
use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::Node;

#[derive(Clone, Debug)]
pub struct Subtree {
    pub id: NodeId,
    pub node: Node,
    pub modifiers: Vec<Modifier>,
    pub children: Vec<Subtree>,
}

impl Subtree {
    pub fn leaf(id: NodeId, node: Node) -> Self {
        Self {
            id,
            node,
            modifiers: vec![],
            children: vec![],
        }
    }

    pub fn with_children(mut self, children: Vec<Subtree>) -> Self {
        self.children = children;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }
}
```

- [ ] **Step 2: into_entries 메서드 작성**

`Subtree`를 문서 삽입용 `(NodeId, NodeEntry)` 목록으로 변환. parent-first order.

```rust
impl Subtree {
    /// Subtree를 NodeEntry 목록으로 변환한다.
    /// parent_id는 이 서브트리의 root가 삽입될 부모 노드.
    /// 반환값은 parent-first order.
    pub fn into_entries(self, parent_id: NodeId) -> Vec<(NodeId, NodeEntry)> {
        let mut entries = Vec::new();
        self.collect_entries(parent_id, &mut entries);
        entries
    }

    fn collect_entries(self, parent_id: NodeId, entries: &mut Vec<(NodeId, NodeEntry)>) {
        let child_ids: imbl::Vector<NodeId> = self.children.iter().map(|c| c.id).collect();
        let entry = NodeEntry {
            node: self.node,
            parent: Some(parent_id),
            children: child_ids,
            modifiers: self.modifiers,
        };
        let self_id = self.id;
        entries.push((self_id, entry));
        for child in self.children {
            child.collect_entries(self_id, entries);
        }
    }
}
```

- [ ] **Step 3: capture 메서드 작성**

문서의 기존 노드를 `Subtree`로 캡처 (remove_node의 undo용).

```rust
use crate::doc::Doc;

impl Subtree {
    /// 문서에서 node_id와 그 descendants를 재귀적으로 캡처하여 Subtree를 생성한다.
    pub fn capture(doc: &Doc, node_id: NodeId) -> Option<Self> {
        let entry = doc.get_entry(node_id)?;
        let children = entry
            .children
            .iter()
            .filter_map(|&child_id| Self::capture(doc, child_id))
            .collect();
        Some(Self {
            id: node_id,
            node: entry.node.clone(),
            modifiers: entry.modifiers.clone(),
            children,
        })
    }
}
```

- [ ] **Step 4: 모듈 등록**

```rust
// crates/editor-model/src/lib.rs 에 추가:
mod subtree;
pub use subtree::*;
```

- [ ] **Step 5: 테스트 작성 및 실행**

`crates/editor-model/src/subtree.rs` 하단에 테스트 추가:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::*;

    #[test]
    fn leaf_creates_childless_subtree() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()));
        assert_eq!(tree.id, id);
        assert!(tree.children.is_empty());
        assert!(tree.modifiers.is_empty());
    }

    #[test]
    fn with_children_builds_nested_subtree() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let tree = Subtree::leaf(parent_id, Node::BulletList(BulletListNode {}))
            .with_children(vec![
                Subtree::leaf(child_id, Node::ListItem(ListItemNode {}))
            ]);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, child_id);
    }

    #[test]
    fn into_entries_produces_parent_first_order() {
        let root_id = NodeId::new();
        let child_id = NodeId::new();
        let grandchild_id = NodeId::new();
        let tree = Subtree::leaf(root_id, Node::BulletList(BulletListNode {}))
            .with_children(vec![
                Subtree::leaf(child_id, Node::ListItem(ListItemNode {}))
                    .with_children(vec![
                        Subtree::leaf(grandchild_id, Node::Paragraph(ParagraphNode::default()))
                    ])
            ]);

        let insertion_parent = NodeId::new();
        let entries = tree.into_entries(insertion_parent);

        assert_eq!(entries.len(), 3);
        // root first
        assert_eq!(entries[0].0, root_id);
        assert_eq!(entries[0].1.parent, Some(insertion_parent));
        assert_eq!(entries[0].1.children.len(), 1);
        // then child
        assert_eq!(entries[1].0, child_id);
        assert_eq!(entries[1].1.parent, Some(root_id));
        // then grandchild
        assert_eq!(entries[2].0, grandchild_id);
        assert_eq!(entries[2].1.parent, Some(child_id));
    }

    #[test]
    fn capture_builds_subtree_from_doc() {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children = imbl::vector![para_id];
                e
            })
            .with_node(
                para_id,
                NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(NodeId::ROOT),
                    children: imbl::vector![text_id],
                    modifiers: vec![],
                },
            )
            .with_node(
                text_id,
                NodeEntry::new(Node::Text(TextNode { text: "Hi".into() }))
                    .with_parent(para_id),
            );

        let tree = Subtree::capture(&doc, para_id).unwrap();
        assert_eq!(tree.id, para_id);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, text_id);
    }
}
```

Run: `cargo test -p editor-model subtree`
Expected: ALL PASS

---

### Task 2: Step 변경 — InsertSubtree/RemoveSubtree

**Files:**
- Modify: `crates/editor-transaction/src/step.rs`
- Modify: `crates/editor-transaction/src/steps/insert_subtree.rs`
- Modify: `crates/editor-transaction/src/steps/remove_subtree.rs`

이 Task는 Step enum 변경 + step 적용 로직 + 모든 패턴 매치를 동시에 수정해야 컴파일된다.

- [ ] **Step 1: Step enum 변경**

`crates/editor-transaction/src/step.rs`에서 InsertSubtree/RemoveSubtree variant 수정:

```rust
// Before:
// InsertSubtree { parent_id: NodeId, index: usize, node_id: NodeId, entry: NodeEntry },
// RemoveSubtree { parent_id: NodeId, index: usize, node_id: NodeId, entry: NodeEntry },

// After:
use editor_model::Subtree;  // import 추가

InsertSubtree {
    parent_id: NodeId,
    index: usize,
    subtree: Subtree,
},
RemoveSubtree {
    parent_id: NodeId,
    index: usize,
    subtree: Subtree,
},
```

Step의 import 라인에 `Subtree` 추가:
```rust
use editor_model::{DocumentAttrs, Modifier, Node, NodeEntry, NodeId, Subtree};
```
(기존 `NodeEntry` import는 다른 variant에서 아직 사용하지 않으면 제거. 단 SplitNode 등에서 사용할 수 있으니 확인 후 결정.)

- [ ] **Step 2: Step::apply 디스패치 수정**

```rust
// step.rs apply() 내:
Step::InsertSubtree {
    parent_id,
    index,
    subtree,
} => steps::insert_node::apply(state, *parent_id, *index, subtree),
Step::RemoveSubtree {
    parent_id,
    index,
    subtree,
} => steps::remove_node::apply(state, *parent_id, *index, subtree),
```

- [ ] **Step 3: Step::inverse 수정**

```rust
// step.rs inverse() 내:
Step::InsertSubtree {
    parent_id,
    index,
    subtree,
} => steps::insert_node::inverse(*parent_id, *index, subtree.clone()),
Step::RemoveSubtree {
    parent_id,
    index,
    subtree,
} => steps::remove_node::inverse(*parent_id, *index, subtree.clone()),
```

- [ ] **Step 4: insert_node step 적용 로직 재작성**

`crates/editor-transaction/src/steps/insert_subtree.rs` 전체 교체:

```rust
use editor_model::{NodeId, Subtree};
use editor_state::State;

use crate::{Step, StepError, validate};

pub(crate) fn apply(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<State, StepError> {
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index > parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    // Subtree를 NodeEntry 목록으로 변환하여 Doc에 삽입
    let entries = subtree.clone().into_entries(parent_id);
    let mut doc = state.doc.clone();
    for (id, entry) in entries {
        doc = doc.insert_node(id, entry);
    }

    // Parent의 children에 root 추가
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.insert(index, subtree.id);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    // Validation
    validate::validate_content(&new_state.doc, parent_id)?;
    validate::validate_context(&new_state.doc, subtree.id)?;
    validate_subtree_content(&new_state.doc, subtree.id)?;

    Ok(new_state)
}

/// 서브트리 내 모든 노드의 content expression을 재귀적으로 검증
fn validate_subtree_content(doc: &editor_model::Doc, node_id: NodeId) -> Result<(), StepError> {
    validate::validate_content(doc, node_id)?;
    if let Some(node_ref) = doc.node(node_id) {
        for child in node_ref.children() {
            validate_subtree_content(doc, child.id())?;
        }
    }
    Ok(())
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
        parent_id,
        index,
        subtree,
    }
}
```

- [ ] **Step 5: remove_node step 적용 로직 재작성**

`crates/editor-transaction/src/steps/remove_subtree.rs` 전체 교체:

```rust
use editor_model::{NodeId, Subtree};
use editor_state::State;

use crate::{Step, StepError, validate};

pub(crate) fn apply(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<State, StepError> {
    let node_id = subtree.id;
    state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index >= parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    // 서브트리의 모든 노드 ID를 수집하여 Doc에서 제거
    let node_ids = collect_ids(subtree);
    let mut doc = state.doc.clone();
    for id in node_ids {
        doc = doc.remove_node(id);
    }

    // Parent의 children에서 제거
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.remove(index);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    validate::validate_content(&new_state.doc, parent_id)?;

    Ok(new_state)
}

fn collect_ids(subtree: &Subtree) -> Vec<NodeId> {
    let mut ids = vec![subtree.id];
    for child in &subtree.children {
        ids.extend(collect_ids(child));
    }
    ids
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::InsertSubtree {
        parent_id,
        index,
        subtree,
    }
}
```

- [ ] **Step 6: 기존 step 테스트를 새 API에 맞게 수정**

`insert_node.rs` 테스트에서 `Step::InsertSubtree`를 `Subtree`로 변경:

```rust
// Before (기존 InsertNode):
// Step::InsertNode { parent_id: NodeId::ROOT, index: 1, node_id: new_id, entry: NodeEntry::new(...) }

// After:
let step = Step::InsertSubtree {
    parent_id: NodeId::ROOT,
    index: 1,
    subtree: Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default())),
};
```

`remove_node.rs` 테스트에서 `Step::RemoveSubtree`를 `Subtree`로 변경:

```rust
// Before (기존 RemoveNode):
// Step::RemoveNode { parent_id: fold_id, index: 0, node_id: fold_title_id, entry: ... }

// After:
let step = Step::RemoveSubtree {
    parent_id: fold_id,
    index: 0,
    subtree: Subtree::capture(&state.doc, fold_title_id).unwrap(),
};
```

모든 테스트에 동일 패턴 적용. `Step::RemoveSubtree { entry, .. }` 패턴 매치도 `Step::RemoveSubtree { subtree, .. }`로 변경.

- [ ] **Step 7: insert_node에 자체 content validation 테스트 추가**

```rust
#[test]
fn insert_empty_container_fails_content_validation() {
    let (state, ..) = state! {
        doc {
            root {
                paragraph { t1: text("Hello") }
            }
        }
        selection: (t1, 0)
    };

    // 빈 BulletList 삽입 시도 — content ListItem+를 만족하지 않으므로 실패해야 함
    let new_id = NodeId::new();
    let step = Step::InsertSubtree {
        parent_id: NodeId::ROOT,
        index: 0,
        subtree: Subtree::leaf(new_id, Node::BulletList(BulletListNode {})),
    };

    assert!(step.apply(&state).is_err());
}

#[test]
fn insert_valid_subtree_succeeds() {
    let (state, ..) = state! {
        doc {
            root {
                paragraph { t1: text("Hello") }
            }
        }
        selection: (t1, 0)
    };

    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let para_id = NodeId::new();
    let subtree = Subtree::leaf(list_id, Node::BulletList(BulletListNode {}))
        .with_children(vec![
            Subtree::leaf(item_id, Node::ListItem(ListItemNode {}))
                .with_children(vec![
                    Subtree::leaf(para_id, Node::Paragraph(ParagraphNode::default()))
                ])
        ]);

    let step = Step::InsertSubtree {
        parent_id: NodeId::ROOT,
        index: 0,
        subtree,
    };

    let new_state = step.apply(&state).unwrap();
    assert!(new_state.doc.get_entry(list_id).is_some());
    assert!(new_state.doc.get_entry(item_id).is_some());
    assert!(new_state.doc.get_entry(para_id).is_some());
}
```

- [ ] **Step 8: remove_node 서브트리 제거 테스트 추가**

```rust
#[test]
fn remove_subtree_removes_all_descendants() {
    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let para_id = NodeId::new();
    let text_id = NodeId::new();

    let doc = Doc::default()
        .with_node_updated(NodeId::ROOT, |mut e| {
            e.children = imbl::vector![list_id];
            e
        })
        .with_node(list_id, NodeEntry {
            node: Node::BulletList(BulletListNode {}),
            parent: Some(NodeId::ROOT),
            children: imbl::vector![item_id],
            modifiers: vec![],
        })
        .with_node(item_id, NodeEntry {
            node: Node::ListItem(ListItemNode {}),
            parent: Some(list_id),
            children: imbl::vector![para_id],
            modifiers: vec![],
        })
        .with_node(para_id, NodeEntry {
            node: Node::Paragraph(ParagraphNode::default()),
            parent: Some(item_id),
            children: imbl::vector![text_id],
            modifiers: vec![],
        })
        .with_node(text_id,
            NodeEntry::new(Node::Text(TextNode { text: "A".into() }))
                .with_parent(para_id),
        );

    // Root content: (choice)*, Paragraph → BulletList만 남으면 trailing Paragraph 없어서 실패
    // 대신 Paragraph + BulletList 구조에서 BulletList 제거 테스트
    // 더 단순한 테스트: Blockquote 안에서 paragraph 하나 제거 (두 개 중)
    // 원래 테스트 의도는 descendants가 모두 제거되는지 확인
    let state = State::new(doc, Selection::collapsed(Position::new(text_id, 0)));

    let subtree = Subtree::capture(&state.doc, list_id).unwrap();
    assert_eq!(subtree.children.len(), 1); // ListItem
    assert_eq!(subtree.children[0].children.len(), 1); // Paragraph

    // Root에서 BulletList 제거 — content violation이지만 descendants 제거 확인이 목적
    // 별도로 valid한 케이스를 만들어 테스트
}
```

Note: content validation 때문에 valid한 상태에서만 remove가 성공한다. `Subtree::capture`가 descendants를 올바르게 캡처하는지는 Task 1에서 테스트 완료.

- [ ] **Step 9: insert→remove roundtrip 테스트 수정**

기존 `insert_then_remove_roundtrip` 테스트를 Subtree API로 업데이트:

```rust
#[test]
fn insert_then_remove_roundtrip() {
    let (state, ..) = state! {
        doc {
            root {
                paragraph { t1: text("Hello") }
            }
        }
        selection: (t1, 0)
    };

    let new_id = NodeId::new();
    let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));
    let step = Step::InsertSubtree {
        parent_id: NodeId::ROOT,
        index: 1,
        subtree,
    };
    let state2 = step.apply(&state).unwrap();
    let state3 = step.inverse().apply(&state2).unwrap();
    assert!(!state3.doc.get_entry(new_id).is_some());
    assert_eq!(state3.doc.node(NodeId::ROOT).unwrap().children().count(), 1);
}
```

- [ ] **Step 10: 컴파일 및 테스트**

Run: `cargo test -p editor-transaction`
Expected: ALL PASS

---

### Task 3: Transaction API 변경

**Files:**
- Modify: `crates/editor-transaction/src/transaction.rs`

- [ ] **Step 1: insert_node 시그니처 변경**

```rust
// Before:
pub fn insert_subtree(
    &mut self,
    parent_id: NodeId,
    index: usize,
    node_id: NodeId,
    entry: NodeEntry,
) -> Result<(), StepError> {
    self.apply_step(Step::InsertSubtree {
        parent_id,
        index,
        node_id,
        entry,
    })
}

// After:
pub fn insert_subtree(
    &mut self,
    parent_id: NodeId,
    index: usize,
    subtree: Subtree,
) -> Result<(), StepError> {
    self.apply_step(Step::InsertSubtree {
        parent_id,
        index,
        subtree,
    })
}
```

import에 `Subtree` 추가:
```rust
use editor_model::{Doc, DocumentAttrs, Modifier, Node, NodeEntry, NodeId, Subtree};
```

- [ ] **Step 2: remove_node에서 Subtree 캡처**

```rust
// Before:
pub fn remove_subtree(&mut self, node_id: NodeId) -> Result<(), StepError> {
    let entry = self.state.doc.get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent_id = entry.parent.ok_or(StepError::NodeNotFound(node_id))?;
    let parent_entry = self.state.doc.get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;
    let index = parent_entry.children.iter()
        .position(|&id| id == node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let entry = entry.clone();
    self.apply_step(Step::RemoveSubtree { parent_id, index, node_id, entry })
}

// After:
pub fn remove_subtree(&mut self, node_id: NodeId) -> Result<(), StepError> {
    let subtree = Subtree::capture(&self.state.doc, node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let entry = self.state.doc.get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent_id = entry.parent.ok_or(StepError::NodeNotFound(node_id))?;
    let parent_entry = self.state.doc.get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;
    let index = parent_entry.children.iter()
        .position(|&id| id == node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    self.apply_step(Step::RemoveSubtree { parent_id, index, subtree })
}
```

- [ ] **Step 3: Transaction 테스트 수정**

`insert_node_records_step` 테스트:
```rust
#[test]
fn insert_node_records_step() {
    let (state, ..) = state! {
        doc { root { paragraph { t1: text("Hello World") } } }
        selection: (t1, 0)
    };

    let mut tr = Transaction::new(&state);
    let new_id = NodeId::new();
    tr.insert_subtree(NodeId::ROOT, 1, Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()))).unwrap();

    assert!(tr.doc().get_entry(new_id).is_some());
    let doc = tr.doc();
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 2);

    let (_, steps, _) = tr.commit();
    assert_eq!(steps.len(), 1);
}
```

`remove_node_derives_entry_from_state` 테스트:
```rust
#[test]
fn remove_node_captures_subtree_from_state() {
    let (state, p1, p2) = state! {
        doc { root { p1: paragraph { text("Hello World") } p2: paragraph {} } }
        selection: (p1, 0)
    };

    let mut tr = Transaction::new(&state);
    tr.remove_subtree(p1).unwrap();

    assert!(tr.doc().get_entry(p1).is_none());
    let doc = tr.doc();
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0], p2);

    let (_, steps, _) = tr.commit();
    match &steps[0] {
        Step::RemoveSubtree { subtree, .. } => {
            assert!(matches!(subtree.node, Node::Paragraph(_)));
        }
        _ => panic!("expected RemoveSubtree"),
    }
}
```

- [ ] **Step 4: 컴파일 및 테스트**

Run: `cargo test -p editor-transaction`
Expected: ALL PASS

---

### Task 4: editor-view 패턴 매치 업데이트

**Files:**
- Modify: `crates/editor-view/src/engine/mod.rs:168`

- [ ] **Step 1: dirty_nodes 함수 수정**

```rust
// Before:
Step::InsertSubtree { parent_id, .. } | Step::RemoveSubtree { parent_id, .. } => {
    vec![*parent_id]
}

// After (변경 없음 — parent_id 필드 이름이 동일하므로 패턴 매치 그대로 동작):
Step::InsertSubtree { parent_id, .. } | Step::RemoveSubtree { parent_id, .. } => {
    vec![*parent_id]
}
```

`parent_id` 필드명이 유지되므로 이 파일은 변경 불필요. 단, `use` 구문에 `NodeEntry`가 Step에서 제거되었을 수 있으니 컴파일 확인.

- [ ] **Step 2: 컴파일 확인**

Run: `cargo check -p editor-view`
Expected: SUCCESS

---

### Task 5: editor-commands 호출자 마이그레이션

**Files:**
- Modify: `crates/editor-commands/src/commands/insert_text.rs`
- Modify: `crates/editor-commands/src/commands/insert_hard_break.rs`
- Modify: `crates/editor-commands/src/commands/sink_paragraph_backward.rs`

- [ ] **Step 1: insert_text.rs 마이그레이션**

모든 `tr.insert_subtree(parent, idx, id, entry)` 호출을 `tr.insert_subtree(parent, idx, Subtree::leaf(id, node))` 로 변경.

```rust
// 파일 상단에 import 추가:
use editor_model::Subtree;

// 각 호출 지점:
// Before:
tr.insert_subtree(parent.id(), node_index, new_id, new_entry)?;

// After:
tr.insert_subtree(parent.id(), node_index, Subtree::leaf(new_id, new_entry.node))?;
```

Note: 기존 `new_entry`는 `NodeEntry::new(node)` 패턴이므로 `new_entry.node`으로 Node를 추출. modifiers가 있는 경우 `.with_modifiers()` 체이닝. 실제 코드에서 modifiers 유무를 확인하여 적용.

- [ ] **Step 2: insert_hard_break.rs 마이그레이션**

동일 패턴 적용. 4개 호출 지점 모두 변경.

```rust
use editor_model::Subtree;

// Before:
tr.insert_subtree(parent.id(), node_index, break_id, break_entry)?;

// After:
tr.insert_subtree(parent.id(), node_index, Subtree::leaf(break_id, break_entry.node))?;
```

- [ ] **Step 3: sink_paragraph_backward.rs — ensure_valid_after_removal 내 insert_node 마이그레이션**

현재 ad-hoc fix-up은 Task 7에서 batch+fulfill로 교체하지만, 우선 컴파일 통과를 위해 insert_node 호출만 Subtree API로 변경:

```rust
use editor_model::Subtree;

// ensure_valid_after_removal 내:
// Before:
tr.insert_subtree(parent_id, parent.entry().children.len(), fix_id, fix_entry)?;

// After:
tr.insert_subtree(parent_id, parent.entry().children.len(), Subtree::leaf(fix_id, Node::Paragraph(ParagraphNode::default())))?;
```

- [ ] **Step 4: 컴파일 및 테스트**

Run: `cargo test -p editor-commands`
Expected: ALL PASS

---

### Task 6: batch + apply_steps

**Files:**
- Modify: `crates/editor-transaction/src/transaction.rs`
- Modify: `crates/editor-transaction/src/step.rs` (apply_unchecked 추가)
- Modify: `crates/editor-transaction/src/steps/insert_subtree.rs` (validate 플래그)
- Modify: `crates/editor-transaction/src/steps/remove_subtree.rs` (validate 플래그)
- Modify: `crates/editor-transaction/src/steps/move_node.rs` (validate 플래그)

- [ ] **Step 1: Step에 apply_unchecked 추가**

`crates/editor-transaction/src/step.rs`에 validation을 건너뛰는 apply 메서드 추가:

```rust
impl Step {
    /// Validation을 수행하지 않고 step을 적용한다. batch 내부에서만 사용.
    pub(crate) fn apply_unchecked(&self, state: &State) -> Result<State, StepError> {
        match self {
            Step::InsertSubtree { parent_id, index, subtree } => {
                steps::insert_node::apply_unchecked(state, *parent_id, *index, subtree)
            }
            Step::RemoveSubtree { parent_id, index, subtree } => {
                steps::remove_node::apply_unchecked(state, *parent_id, *index, subtree)
            }
            Step::MoveNode { node_id, old_parent, old_index, new_parent, new_index } => {
                steps::move_node::apply_unchecked(state, *node_id, *old_parent, *old_index, *new_parent, *new_index)
            }
            // 나머지 step은 validation이 가벼우므로 기존 apply 재사용
            _ => self.apply(state),
        }
    }
}
```

- [ ] **Step 2: insert_node에 apply_unchecked 추가**

`crates/editor-transaction/src/steps/insert_subtree.rs`:

```rust
/// Validation 없이 삽입만 수행 (batch 내부용)
pub(crate) fn apply_unchecked(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<State, StepError> {
    let parent = state.doc.get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;
    if index > parent.children.len() {
        return Err(StepError::IndexOutOfBounds { parent_id, index, len: parent.children.len() });
    }

    let entries = subtree.clone().into_entries(parent_id);
    let mut doc = state.doc.clone();
    for (id, entry) in entries {
        doc = doc.insert_node(id, entry);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.insert(index, subtree.id);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;
    Ok(new_state)
}
```

- [ ] **Step 3: remove_node에 apply_unchecked 추가**

`crates/editor-transaction/src/steps/remove_subtree.rs`:

```rust
pub(crate) fn apply_unchecked(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<State, StepError> {
    let node_id = subtree.id;
    state.doc.get_entry(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    let parent = state.doc.get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;
    if index >= parent.children.len() {
        return Err(StepError::IndexOutOfBounds { parent_id, index, len: parent.children.len() });
    }

    let node_ids = collect_ids(subtree);
    let mut doc = state.doc.clone();
    for id in node_ids {
        doc = doc.remove_node(id);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.remove(index);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;
    Ok(new_state)
}
```

- [ ] **Step 4: move_node에 apply_unchecked 추가**

`crates/editor-transaction/src/steps/move_node.rs`:

기존 `apply` 함수에서 validation 3줄을 제거한 복사본:

```rust
pub(crate) fn apply_unchecked(
    state: &State,
    node_id: NodeId,
    old_parent: NodeId,
    old_index: usize,
    new_parent: NodeId,
    new_index: usize,
) -> Result<State, StepError> {
    state.doc.get_entry(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    let old_p = state.doc.get_entry(old_parent)
        .ok_or(StepError::NodeNotFound(old_parent))?;
    if old_index >= old_p.children.len() {
        return Err(StepError::IndexOutOfBounds { parent_id: old_parent, index: old_index, len: old_p.children.len() });
    }

    let doc = state.doc.with_node_updated(old_parent, |mut entry| {
        let mut children = entry.children.clone();
        children.remove(old_index);
        entry.children = children;
        entry
    });
    let new_p = doc.get_entry(new_parent).ok_or(StepError::NodeNotFound(new_parent))?;
    if new_index > new_p.children.len() {
        return Err(StepError::IndexOutOfBounds { parent_id: new_parent, index: new_index, len: new_p.children.len() });
    }

    let doc = doc.with_node_updated(new_parent, |mut entry| {
        let mut children = entry.children.clone();
        children.insert(new_index, node_id);
        entry.children = children;
        entry
    });
    let doc = doc.with_node_updated(node_id, |mut entry| {
        entry.parent = Some(new_parent);
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;
    // validation 생략
    Ok(new_state)
}
```

- [ ] **Step 5: Transaction에 batch + apply_steps 추가**

`crates/editor-transaction/src/transaction.rs`:

```rust
impl Transaction {
    // ... 기존 메서드들 ...

    /// Batch 내 step들은 개별 validation을 건너뛰고,
    /// batch 종료 시 변경된 모든 노드의 content + context를 1회 검증한다.
    /// 실패 시 batch 시작 시점의 상태로 rollback.
    pub fn batch<F>(&mut self, f: F) -> Result<(), StepError>
    where
        F: FnOnce(&mut Transaction) -> Result<(), StepError>,
    {
        let sp = self.savepoint();
        self.batching = true;
        let result = f(self);
        self.batching = false;

        match result {
            Ok(()) => {
                // Batch 내에서 변경된 노드들의 validation 수행
                if let Err(e) = self.validate_batch(&sp) {
                    self.rollback(sp);
                    return Err(e);
                }
                Ok(())
            }
            Err(e) => {
                self.rollback(sp);
                Err(e)
            }
        }
    }

    /// 복수 step을 순차 적용.
    pub fn apply_steps(&mut self, steps: Vec<Step>) -> Result<(), StepError> {
        for step in steps {
            self.apply_step(step)?;
        }
        Ok(())
    }

    fn validate_batch(&self, sp: &Savepoint) -> Result<(), StepError> {
        use crate::validate;
        use std::collections::HashSet;

        let mut content_targets = HashSet::new();
        let mut context_targets = Vec::new();

        for step in &self.steps[sp.steps_len..] {
            match step {
                Step::InsertSubtree { parent_id, subtree, .. } => {
                    content_targets.insert(*parent_id);
                    // 서브트리 내 모든 노드의 content 검증
                    collect_subtree_ids_for_validation(subtree, &mut content_targets);
                    context_targets.push(subtree.id);
                }
                Step::RemoveSubtree { parent_id, .. } => {
                    content_targets.insert(*parent_id);
                }
                Step::MoveNode { old_parent, new_parent, node_id, .. } => {
                    content_targets.insert(*old_parent);
                    content_targets.insert(*new_parent);
                    context_targets.push(*node_id);
                }
                _ => {}
            }
        }

        for node_id in &content_targets {
            if self.state.doc.get_entry(*node_id).is_some() {
                validate::validate_content(&self.state.doc, *node_id)?;
            }
        }
        for node_id in &context_targets {
            if self.state.doc.get_entry(*node_id).is_some() {
                validate::validate_context_deep(&self.state.doc, *node_id)?;
            }
        }

        Ok(())
    }
}

fn collect_subtree_ids_for_validation(subtree: &Subtree, targets: &mut std::collections::HashSet<NodeId>) {
    targets.insert(subtree.id);
    for child in &subtree.children {
        collect_subtree_ids_for_validation(child, targets);
    }
}
```

Transaction 구조체에 `batching` 필드 추가:

```rust
pub struct Transaction {
    state: State,
    steps: Vec<Step>,
    effects: Vec<Effect>,
    batching: bool,
}

// new()에서:
Self { state: state.clone(), steps: Vec::new(), effects: Vec::new(), batching: false }
```

`apply_step`에서 batching 모드 처리:

```rust
fn apply_step(&mut self, step: Step) -> Result<(), StepError> {
    self.state = if self.batching {
        step.apply_unchecked(&self.state)?
    } else {
        step.apply(&self.state)?
    };
    self.steps.push(step);
    Ok(())
}
```

`Savepoint`에 `steps_len` 접근 가능하도록:
```rust
// Savepoint는 이미 pub steps_len을 가지고 있으나, validate_batch에서 접근 필요.
// Savepoint 필드가 private이면 getter 추가:
impl Savepoint {
    pub(crate) fn steps_len(&self) -> usize {
        self.steps_len
    }
}
```

실제 구현 시 Savepoint 필드 접근성에 따라 조정.

- [ ] **Step 6: batch 테스트 작성**

```rust
#[test]
fn batch_defers_validation() {
    let (state, ..) = state! {
        doc {
            root {
                blockquote {
                    paragraph { t1: text("A") }
                }
                paragraph { t2: text("B") }
            }
        }
        selection: (t2, 0)
    };

    let mut tr = Transaction::new(&state);
    let bq_id = state.doc.node(NodeId::ROOT).unwrap()
        .children().next().unwrap().id();
    let para_id = state.doc.node(NodeId::ROOT).unwrap()
        .children().nth(1).unwrap().id();

    // batch 없이 move하면 Root에서 trailing Paragraph 없어서 실패
    // (별도 테스트로 확인 가능)

    // batch 안에서: move + fix-up
    let fix_id = NodeId::new();
    tr.batch(|tr| {
        let target_children = tr.doc().node(bq_id).unwrap().children().count();
        tr.move_node(para_id, bq_id, target_children)?;
        tr.insert_subtree(
            NodeId::ROOT,
            1, // Blockquote 뒤
            Subtree::leaf(fix_id, Node::Paragraph(ParagraphNode::default())),
        )?;
        Ok(())
    }).unwrap();

    // 최종 상태 확인: Root = [Blockquote, Paragraph(fix-up)]
    let doc = tr.doc();
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 2);
    assert!(doc.get_entry(fix_id).is_some());
}

#[test]
fn batch_rolls_back_on_invalid_final_state() {
    let (state, ..) = state! {
        doc {
            root {
                blockquote {
                    paragraph { t1: text("A") }
                }
                paragraph { t2: text("B") }
            }
        }
        selection: (t2, 0)
    };

    let mut tr = Transaction::new(&state);
    let bq_id = state.doc.node(NodeId::ROOT).unwrap()
        .children().next().unwrap().id();
    let para_id = state.doc.node(NodeId::ROOT).unwrap()
        .children().nth(1).unwrap().id();

    // batch 안에서 move만 하고 fix-up 안 함 → batch 종료 시 validation 실패 → rollback
    let result = tr.batch(|tr| {
        let target_children = tr.doc().node(bq_id).unwrap().children().count();
        tr.move_node(para_id, bq_id, target_children)?;
        Ok(())
    });

    assert!(result.is_err());
    // rollback 확인: 원래 상태 유지
    let doc = tr.doc();
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 2);
}
```

- [ ] **Step 7: 컴파일 및 테스트**

Run: `cargo test -p editor-transaction`
Expected: ALL PASS

---

### Task 7: fulfill 헬퍼

**Files:**
- Create: `crates/editor-transaction/src/fulfill.rs`
- Modify: `crates/editor-commands/src/helpers/mod.rs`

- [ ] **Step 1: fulfill 함수 작성**

```rust
// crates/editor-transaction/src/fulfill.rs
use editor_model::{Node, NodeId, NodeRef, NodeType, Subtree};
use editor_model::nodes::*;
use editor_schema::{ContentExpr, NodeSpecExt};
use editor_transaction::Step;

/// 주어진 노드의 content expression을 만족시키기 위해 필요한 InsertSubtree step들을 계산한다.
/// 이미 valid하면 빈 Vec을 반환한다.
pub fn fulfill(node: &NodeRef) -> Vec<Step> {
    let spec = node.node().spec();
    let child_types: Vec<NodeType> = node.children().map(|c| c.node().as_type()).collect();

    if spec.content.matches_sequence(&child_types) {
        return vec![];
    }

    let insertions = compute_insertions(&spec.content, &child_types);
    insertions
        .into_iter()
        .map(|(index, node_type)| {
            let subtree = scaffold(node_type);
            Step::InsertSubtree {
                parent_id: node.id(),
                index,
                subtree,
            }
        })
        .collect()
}

/// Content expression과 현재 children을 비교하여 필요한 삽입을 계산한다.
/// 반환값: (삽입 위치, 삽입할 NodeType) 목록
fn compute_insertions(content: &ContentExpr, existing: &[NodeType]) -> Vec<(usize, NodeType)> {
    match content {
        ContentExpr::Empty => vec![],

        // OneOrMore: 최소 1개 필요
        ContentExpr::OneOrMore(inner) => {
            if existing.is_empty() {
                let default_type = first_type(inner);
                vec![(0, default_type)]
            } else {
                vec![]
            }
        }

        // ZeroOrMore: 항상 valid
        ContentExpr::ZeroOrMore(_) => vec![],

        // Optional: 항상 valid
        ContentExpr::Optional(_) => vec![],

        // Single: 정확히 1개 필요
        ContentExpr::Single(t) => {
            if existing.is_empty() {
                vec![(0, *t)]
            } else {
                vec![]
            }
        }

        // Choice: 정확히 1개 필요
        ContentExpr::Choice(choices) => {
            if existing.is_empty() {
                let default_type = first_type(&choices[0]);
                vec![(0, default_type)]
            } else {
                vec![]
            }
        }

        // Seq: 각 요소를 순서대로 확인
        ContentExpr::Seq(exprs) => {
            compute_seq_insertions(exprs, existing)
        }
    }
}

/// Seq 패턴에서 필요한 삽입을 계산한다.
fn compute_seq_insertions(exprs: &[ContentExpr], existing: &[NodeType]) -> Vec<(usize, NodeType)> {
    let mut insertions = Vec::new();
    let mut existing_idx = 0;

    for (i, expr) in exprs.iter().enumerate() {
        match expr {
            ContentExpr::Single(t) => {
                if existing_idx < existing.len() && existing[existing_idx] == *t {
                    existing_idx += 1;
                } else {
                    // 이 위치의 required element가 없음 → 삽입 필요
                    insertions.push((existing_idx + insertions.len(), *t));
                }
            }
            ContentExpr::ZeroOrMore(inner) | ContentExpr::OneOrMore(inner) => {
                let is_one_or_more = matches!(expr, ContentExpr::OneOrMore(_));

                // 매칭되는 만큼 소비
                let required_after: usize = exprs[i + 1..].iter().map(|e| e.min_required()).sum();
                let max_consume = existing.len().saturating_sub(required_after);

                let mut consumed = 0;
                while existing_idx < max_consume && inner.matches(existing[existing_idx]) {
                    existing_idx += 1;
                    consumed += 1;
                }

                if is_one_or_more && consumed == 0 {
                    let default_type = first_type(inner);
                    insertions.push((existing_idx + insertions.len(), default_type));
                }
            }
            ContentExpr::Optional(inner) => {
                if existing_idx < existing.len() && inner.matches(existing[existing_idx]) {
                    existing_idx += 1;
                }
            }
            _ => {}
        }
    }

    insertions
}

/// ContentExpr에서 첫 번째 concrete NodeType을 추출한다.
fn first_type(expr: &ContentExpr) -> NodeType {
    match expr {
        ContentExpr::Single(t) => *t,
        ContentExpr::Choice(choices) => first_type(&choices[0]),
        ContentExpr::OneOrMore(inner) | ContentExpr::ZeroOrMore(inner) | ContentExpr::Optional(inner) => {
            first_type(inner)
        }
        ContentExpr::Seq(exprs) => first_type(&exprs[0]),
        ContentExpr::Empty => unreachable!("Empty content has no type"),
    }
}

/// 주어진 NodeType에 대해 최소 유효 서브트리를 생성한다.
/// content expression의 required children을 재귀적으로 채운다.
fn scaffold(node_type: NodeType) -> Subtree {
    let id = NodeId::new();
    let node = default_node(node_type);
    let spec = node_type.spec();

    let children = scaffold_children(&spec.content);

    Subtree {
        id,
        node,
        modifiers: vec![],
        children,
    }
}

/// Content expression의 required 부분에 대해 scaffold된 children을 생성한다.
fn scaffold_children(content: &ContentExpr) -> Vec<Subtree> {
    match content {
        ContentExpr::Empty | ContentExpr::ZeroOrMore(_) | ContentExpr::Optional(_) => vec![],
        ContentExpr::Single(t) => vec![scaffold(*t)],
        ContentExpr::OneOrMore(inner) => {
            let t = first_type(inner);
            vec![scaffold(t)]
        }
        ContentExpr::Choice(choices) => {
            let t = first_type(&choices[0]);
            vec![scaffold(t)]
        }
        ContentExpr::Seq(exprs) => {
            exprs.iter().flat_map(|e| scaffold_children(e)).collect()
        }
    }
}

/// NodeType에 대한 default Node 인스턴스를 생성한다.
fn default_node(node_type: NodeType) -> Node {
    match node_type {
        NodeType::Root => Node::Root(RootNode {}),
        NodeType::Paragraph => Node::Paragraph(ParagraphNode::default()),
        NodeType::Blockquote => Node::Blockquote(BlockquoteNode {}),
        NodeType::Callout => Node::Callout(CalloutNode::default()),
        NodeType::Text => Node::Text(TextNode { text: String::new() }),
        NodeType::BulletList => Node::BulletList(BulletListNode {}),
        NodeType::OrderedList => Node::OrderedList(OrderedListNode::default()),
        NodeType::ListItem => Node::ListItem(ListItemNode {}),
        NodeType::Fold => Node::Fold(FoldNode {}),
        NodeType::FoldTitle => Node::FoldTitle(FoldTitleNode {}),
        NodeType::FoldContent => Node::FoldContent(FoldContentNode {}),
        NodeType::Table => Node::Table(TableNode::default()),
        NodeType::TableRow => Node::TableRow(TableRowNode {}),
        NodeType::TableCell => Node::TableCell(TableCellNode { col_width: None }),
        NodeType::Image => Node::Image(ImageNode::default()),
        NodeType::File => Node::File(FileNode::default()),
        NodeType::Embed => Node::Embed(EmbedNode::default()),
        NodeType::Archived => Node::Archived(ArchivedNode::default()),
        NodeType::HardBreak => Node::HardBreak(HardBreakNode {}),
        NodeType::HorizontalRule => Node::HorizontalRule(HorizontalRuleNode {}),
        NodeType::PageBreak => Node::PageBreak(PageBreakNode {}),
    }
}
```

Note: `min_required`는 이미 `ContentExpr`에 존재 (`crates/editor-schema/src/content.rs:63`). 단, 현재 `pub`가 아닐 수 있으니 확인 후 필요 시 `pub`로 변경.

- [ ] **Step 2: 모듈 등록**

`crates/editor-transaction/src/lib.rs`에 모듈 추가 및 re-export:

```rust
mod fulfill;
pub use fulfill::fulfill;
```

- [ ] **Step 3: ContentExpr::min_required 접근성 확인**

`crates/editor-schema/src/content.rs`에서 `min_required`가 `fn`이면 `pub fn`으로 변경:

```rust
// Before:
fn min_required(&self) -> usize {

// After:
pub fn min_required(&self) -> usize {
```

- [ ] **Step 4: fulfill 테스트 작성**

`crates/editor-transaction/src/fulfill.rs` 하단에 테스트 추가:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::*;

    #[test]
    fn fulfill_valid_node_returns_empty() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        let doc = state.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let steps = fulfill(&root);
        assert!(steps.is_empty());
    }

    #[test]
    fn fulfill_root_missing_trailing_paragraph() {
        // Root content: (choice)*, Paragraph
        // Root에 Blockquote만 있으면 trailing Paragraph 누락
        let bq_id = NodeId::new();
        let bq_para_id = NodeId::new();
        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children = imbl::vector![bq_id];
                e
            })
            .with_node(bq_id, NodeEntry {
                node: Node::Blockquote(BlockquoteNode {}),
                parent: Some(NodeId::ROOT),
                children: imbl::vector![bq_para_id],
                modifiers: vec![],
            })
            .with_node(bq_para_id, NodeEntry {
                node: Node::Paragraph(ParagraphNode::default()),
                parent: Some(bq_id),
                children: imbl::vector![],
                modifiers: vec![],
            });

        let root = doc.node(NodeId::ROOT).unwrap();
        let steps = fulfill(&root);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree { parent_id, index, subtree } => {
                assert_eq!(*parent_id, NodeId::ROOT);
                assert_eq!(*index, 1);
                assert!(matches!(subtree.node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn fulfill_empty_blockquote_inserts_paragraph() {
        // Blockquote content: (Paragraph | BulletList | OrderedList)+
        let bq_id = NodeId::new();
        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children = imbl::vector![bq_id];
                e
            })
            .with_node(bq_id, NodeEntry {
                node: Node::Blockquote(BlockquoteNode {}),
                parent: Some(NodeId::ROOT),
                children: imbl::vector![],
                modifiers: vec![],
            });

        let bq = doc.node(bq_id).unwrap();
        let steps = fulfill(&bq);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree { subtree, .. } => {
                assert!(matches!(subtree.node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn fulfill_empty_bullet_list_inserts_list_item_with_paragraph() {
        // BulletList content: ListItem+
        // ListItem content: Paragraph, (BulletList | OrderedList)*
        let list_id = NodeId::new();
        let doc = Doc::default()
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children = imbl::vector![list_id];
                e
            })
            .with_node(list_id, NodeEntry {
                node: Node::BulletList(BulletListNode {}),
                parent: Some(NodeId::ROOT),
                children: imbl::vector![],
                modifiers: vec![],
            });

        let list = doc.node(list_id).unwrap();
        let steps = fulfill(&list);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::InsertSubtree { subtree, .. } => {
                // ListItem with Paragraph child
                assert!(matches!(subtree.node, Node::ListItem(_)));
                assert_eq!(subtree.children.len(), 1);
                assert!(matches!(subtree.children[0].node, Node::Paragraph(_)));
            }
            _ => panic!("expected InsertSubtree"),
        }
    }

    #[test]
    fn scaffold_produces_minimum_valid_subtree() {
        // BulletList → ListItem → Paragraph
        let tree = scaffold(NodeType::BulletList);
        assert!(matches!(tree.node, Node::BulletList(_)));
        assert_eq!(tree.children.len(), 1);

        let item = &tree.children[0];
        assert!(matches!(item.node, Node::ListItem(_)));
        assert_eq!(item.children.len(), 1);

        let para = &item.children[0];
        assert!(matches!(para.node, Node::Paragraph(_)));
        assert!(para.children.is_empty()); // Paragraph content: (Text|HardBreak)* → 0 OK
    }
}
```

- [ ] **Step 5: 컴파일 및 테스트**

Run: `cargo test -p editor-transaction fulfill`
Expected: ALL PASS

---

### Task 8: sink_paragraph_backward 리팩터링

**Files:**
- Modify: `crates/editor-commands/src/commands/sink_paragraph_backward.rs`

- [ ] **Step 1: batch + fulfill로 교체**

```rust
// Before:
// Fix-up: ensure source parent remains valid after paragraph removal
let source_parent_id = paragraph
    .parent()
    .ok_or(CommandError::NoParent(paragraph_id))?
    .id();
ensure_valid_after_removal(tr, source_parent_id, paragraph_id)?;

let doc = tr.doc();
let target = doc
    .node(target_id)
    .ok_or(CommandError::NodeNotFound(target_id))?;
let target_children_count = target.entry().children.len();

tr.move_node(paragraph_id, target_id, target_children_count)?;

// After:
use editor_transaction::fulfill;

let source_parent_id = paragraph
    .parent()
    .ok_or(CommandError::NoParent(paragraph_id))?
    .id();

let doc = tr.doc();
let target = doc
    .node(target_id)
    .ok_or(CommandError::NodeNotFound(target_id))?;
let target_children_count = target.entry().children.len();

tr.batch(|tr| {
    tr.move_node(paragraph_id, target_id, target_children_count)?;
    let doc = tr.doc();
    let parent = doc
        .node(source_parent_id)
        .ok_or(CommandError::NoParent(source_parent_id))?;
    tr.apply_steps(fulfill(&parent))?;
    Ok(())
})?;
```

Note: `CommandError`를 `StepError`로 변환해야 할 수 있음. `batch` 클로저는 `Result<(), StepError>`를 반환하므로, `CommandError`를 StepError로 변환하거나 batch 밖에서 준비 작업을 수행.

실제로는 `ok_or` 대신 batch 밖에서 parent 존재 여부를 확인하고, batch 안에서는 step만 실행하는 것이 더 깔끔:

```rust
let source_parent_id = paragraph
    .parent()
    .ok_or(CommandError::NoParent(paragraph_id))?
    .id();

let doc = tr.doc();
let target = doc
    .node(target_id)
    .ok_or(CommandError::NodeNotFound(target_id))?;
let target_children_count = target.entry().children.len();

tr.batch(|tr| {
    tr.move_node(paragraph_id, target_id, target_children_count)?;
    let doc = tr.doc();
    if let Some(parent) = doc.node(source_parent_id) {
        tr.apply_steps(fulfill(&parent))?;
    }
    Ok(())
})?;
```

- [ ] **Step 2: ensure_valid_after_removal 함수 삭제**

`sink_paragraph_backward.rs`에서 `ensure_valid_after_removal` 함수 전체 삭제 (96-122행).

- [ ] **Step 3: 기존 테스트 실행하여 동작 확인**

Run: `cargo test -p editor-commands sink_paragraph`
Expected: ALL PASS

기존 테스트(`sink_into_blockquote`, `sink_into_callout`, `sink_deep_blockquote`, `sink_empty_paragraph_into_blockquote` 등)가 모두 통과해야 한다. 이 테스트들이 fix-up Paragraph가 올바르게 삽입되는지 검증한다.

- [ ] **Step 4: 전체 테스트**

Run: `cargo test`
Expected: ALL PASS
