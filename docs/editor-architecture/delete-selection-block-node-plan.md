# delete_selection Block-Level Node Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** Do NOT run `git commit`. The user will commit manually.

**Goal:** `delete_selection`이 block-level leaf node(Image, HorizontalRule, File, Embed, Archived)를 포함한 selection을 올바르게 삭제하도록 한다.

**Architecture:** 기존 trim_from/trim_to/collect_fully_selected 3개 함수를 하나의 재귀 walk(delete_range/delete_from/delete_to)으로 교체. merge_after_delete는 시그니처만 변경(사전 기록된 textblock 수신). 커서 위치는 resolve_selection_at으로 유효한 selection으로 resolve.

**Tech Stack:** Rust, editor-commands/editor-state/editor-model/editor-transaction crates

**Spec:** `docs/editor-architecture/delete-selection-block-node-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/editor-commands/src/helpers/tree.rs` | `path_from_ancestor` 추가 |
| Modify | `crates/editor-commands/src/commands/delete_selection.rs` | 핵심 알고리즘 교체, merge 시그니처 변경, 커서 로직 |

---

### Task 1: path_from_ancestor 헬퍼

**Files:**
- Modify: `crates/editor-commands/src/helpers/tree.rs`

- [ ] **Step 1: 테스트 작성**

`crates/editor-commands/src/helpers/tree.rs`의 `mod tests` 블록 안에 추가:

```rust
#[test]
fn path_from_ancestor_same_node() {
    let (doc, _, _, _, _) = make_nested_doc();
    assert_eq!(
        path_from_ancestor(&doc, NodeId::ROOT, NodeId::ROOT),
        Some(vec![])
    );
}

#[test]
fn path_from_ancestor_direct_child() {
    let (doc, p1, _, _, _) = make_nested_doc();
    assert_eq!(
        path_from_ancestor(&doc, p1, NodeId::ROOT),
        Some(vec![0])
    );
}

#[test]
fn path_from_ancestor_grandchild() {
    let (doc, _, _, t1, _) = make_nested_doc();
    assert_eq!(
        path_from_ancestor(&doc, t1, NodeId::ROOT),
        Some(vec![0, 0])
    );
}

#[test]
fn path_from_ancestor_second_branch() {
    let (doc, _, _, _, t2) = make_nested_doc();
    assert_eq!(
        path_from_ancestor(&doc, t2, NodeId::ROOT),
        Some(vec![1, 0])
    );
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-commands path_from_ancestor`

Expected: FAIL — `path_from_ancestor` 함수가 존재하지 않음

- [ ] **Step 3: 구현**

`crates/editor-commands/src/helpers/tree.rs`에 함수 추가:

```rust
/// Compute the index path from `ancestor_id` down to `node_id`.
/// Returns None if `node_id` is not a descendant of `ancestor_id`.
pub(crate) fn path_from_ancestor(doc: &Doc, node_id: NodeId, ancestor_id: NodeId) -> Option<Vec<usize>> {
    if node_id == ancestor_id {
        return Some(Vec::new());
    }
    let mut path = Vec::new();
    let mut current = node_id;
    loop {
        let node = doc.node(current)?;
        let idx = node.index()?;
        path.push(idx);
        let parent_id = node.parent()?.id();
        if parent_id == ancestor_id {
            path.reverse();
            return Some(path);
        }
        current = parent_id;
    }
}
```

- [ ] **Step 4: 테스트 실행 — 통과 확인**

Run: `cargo test -p editor-commands path_from_ancestor`

Expected: 4 tests PASS

---

### Task 2: 재귀 Walk 핵심 함수

**Files:**
- Modify: `crates/editor-commands/src/commands/delete_selection.rs`

- [ ] **Step 1: delete_from 작성**

`trim_from` 함수 아래에 새 함수 추가 (기존 함수는 아직 삭제하지 않음):

```rust
/// Recursively delete content from path position to end of subtree.
fn delete_from(tr: &mut Transaction, path: &[usize], node_id: NodeId) -> Result<(), CommandError> {
    let doc = tr.doc();
    let node = doc.node(node_id).ok_or(CommandError::NodeNotFound(node_id))?;

    if path.len() == 1 {
        let offset = path[0];
        match node.node() {
            Node::Text(t) => {
                let text_len = t.text.char_count();
                if offset == 0 {
                    tr.remove_subtree(node_id)?;
                } else if offset < text_len {
                    tr.remove_text(node_id, offset, text_len - offset)?;
                }
            }
            _ => {
                let children: Vec<NodeId> = node.entry().children.iter().skip(offset).copied().collect();
                for child_id in children.into_iter().rev() {
                    tr.remove_subtree(child_id)?;
                }
            }
        }
    } else {
        let idx = path[0];
        let children: Vec<NodeId> = node.entry().children.iter().copied().collect();

        for i in ((idx + 1)..children.len()).rev() {
            tr.remove_subtree(children[i])?;
        }

        delete_from(tr, &path[1..], children[idx])?;
    }

    Ok(())
}
```

- [ ] **Step 2: delete_to 작성**

```rust
/// Recursively delete content from start of subtree to path position.
fn delete_to(tr: &mut Transaction, path: &[usize], node_id: NodeId) -> Result<(), CommandError> {
    let doc = tr.doc();
    let node = doc.node(node_id).ok_or(CommandError::NodeNotFound(node_id))?;

    if path.len() == 1 {
        let offset = path[0];
        match node.node() {
            Node::Text(t) => {
                let text_len = t.text.char_count();
                if offset >= text_len {
                    tr.remove_subtree(node_id)?;
                } else if offset > 0 {
                    tr.remove_text(node_id, 0, offset)?;
                }
            }
            _ => {
                let children: Vec<NodeId> = node.entry().children.iter().take(offset).copied().collect();
                for child_id in children.into_iter().rev() {
                    tr.remove_subtree(child_id)?;
                }
            }
        }
    } else {
        let idx = path[0];
        let children: Vec<NodeId> = node.entry().children.iter().copied().collect();

        for i in (0..idx).rev() {
            tr.remove_subtree(children[i])?;
        }

        delete_to(tr, &path[1..], children[idx])?;
    }

    Ok(())
}
```

- [ ] **Step 3: delete_range 작성**

```rust
/// Recursively delete the range [from_path, to_path) within the subtree rooted at node_id.
fn delete_range(
    tr: &mut Transaction,
    from_path: &[usize],
    to_path: &[usize],
    node_id: NodeId,
) -> Result<(), CommandError> {
    let from_idx = from_path[0];
    let to_idx = to_path[0];

    if from_idx == to_idx {
        let doc = tr.doc();
        let node = doc.node(node_id).ok_or(CommandError::NodeNotFound(node_id))?;
        let child_id = *node.entry().children.get(from_idx)
            .ok_or(CommandError::Corrupted("child index out of bounds".into()))?;

        match (from_path.len(), to_path.len()) {
            (1, 1) => {
                delete_within_node(tr, node_id, from_idx, to_idx)?;
            }
            (1, _) => {
                delete_to(tr, &to_path[1..], child_id)?;
            }
            (_, 1) => {
                delete_from(tr, &from_path[1..], child_id)?;
            }
            (_, _) => {
                delete_range(tr, &from_path[1..], &to_path[1..], child_id)?;
            }
        }
        return Ok(());
    }

    // from_idx < to_idx: different children
    let doc = tr.doc();
    let node = doc.node(node_id).ok_or(CommandError::NodeNotFound(node_id))?;
    let children: Vec<NodeId> = node.entry().children.iter().copied().collect();

    // From boundary
    if from_path.len() > 1 {
        delete_from(tr, &from_path[1..], children[from_idx])?;
    }

    // Fully selected middle nodes (reverse order for index stability)
    let fully_from = if from_path.len() == 1 { from_idx } else { from_idx + 1 };
    let fully_to = to_idx;
    for i in (fully_from..fully_to).rev() {
        tr.remove_subtree(children[i])?;
    }

    // To boundary
    if to_path.len() > 1 {
        delete_to(tr, &to_path[1..], children[to_idx])?;
    }

    Ok(())
}
```

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-commands`

Expected: 컴파일 성공 (새 함수들은 아직 호출되지 않으므로 dead_code warning 가능)

---

### Task 3: delete_selection 통합 + 기존 테스트 통과

**Files:**
- Modify: `crates/editor-commands/src/commands/delete_selection.rs`

- [ ] **Step 1: merge_after_delete 시그니처 변경**

기존 `merge_after_delete` 함수의 시그니처와 내부의 textblock 조회 로직을 변경한다:

```rust
/// After deletion, merge boundary textblocks and clean up containers.
fn merge_after_delete(
    tr: &mut Transaction,
    from_tb: Option<NodeId>,
    to_tb: Option<NodeId>,
    lca_id: NodeId,
) -> Result<(), CommandError> {
    let (from_tb, to_tb) = match (from_tb, to_tb) {
        (Some(a), Some(b)) if a != b => (a, b),
        _ => return Ok(()),
    };

    let doc = tr.doc();
    if doc.node(from_tb).is_none() || doc.node(to_tb).is_none() {
        return Ok(());
    }

    let to_tb_parent = doc.node(to_tb).and_then(|n| n.parent()).map(|p| p.id());

    // Paragraph-level merge
    tr.merge_node(to_tb, from_tb)?;

    // Container-level merge: walk up, merge adjacent same-type siblings
    let mut from_current = {
        let doc = tr.doc();
        doc.node(from_tb).and_then(|n| n.parent()).map(|p| p.id())
    };

    loop {
        let Some(from_id) = from_current else { break };
        if from_id == lca_id {
            break;
        }

        let doc = tr.doc();
        let Some(from_node) = doc.node(from_id) else {
            break;
        };
        let Some(next) = from_node.next_sibling() else {
            break;
        };

        if next.node().as_type() != from_node.node().as_type() {
            break;
        }

        let next_id = next.id();
        let parent_id = from_node.parent().map(|p| p.id());
        tr.merge_node(next_id, from_id)?;
        from_current = parent_id;
    }

    // Cleanup
    let doc = tr.doc();
    if let Some(parent_id) = to_tb_parent {
        if let Some(parent) = doc.node(parent_id) {
            if parent.entry().children.is_empty() {
                tr.apply_steps(prune(&parent))?;
            }
        }
    }

    let doc = tr.doc();
    if let Some(lca) = doc.node(lca_id) {
        tr.apply_steps(fulfill(&lca))?;
    }

    Ok(())
}
```

- [ ] **Step 2: delete_selection 진입부 교체**

`delete_selection` 함수의 `else` 브랜치 (multi-node path, 현재 lines 26-57)를 교체한다:

```rust
    } else {
        let lca_id = find_lowest_common_ancestor(&doc, from.node_id, to.node_id)
            .ok_or(CommandError::Corrupted("no common ancestor".into()))?;

        // Pre-compute merge targets before deletion
        let from_tb = find_ancestor_textblock(&doc, from.node_id);
        let to_tb = find_ancestor_textblock(&doc, to.node_id);

        // Record cursor fallback info for inline from that may be deleted
        let from_is_text = matches!(doc.node(from.node_id).map(|n| n.node()), Some(Node::Text(_)));
        let cursor_fallback = if from_is_text && from.offset == 0 {
            let node = doc.node(from.node_id).unwrap();
            Some((
                node.prev_sibling().map(|n| n.id()),
                node.next_sibling().map(|n| n.id()),
                node.parent().map(|p| p.id()).unwrap_or(lca_id),
                node.index().unwrap_or(0),
            ))
        } else {
            None
        };

        // Compute paths from LCA
        let mut from_path = path_from_ancestor(&doc, from.node_id, lca_id)
            .ok_or(CommandError::Corrupted("from is not a descendant of LCA".into()))?;
        from_path.push(from.offset);

        let mut to_path = path_from_ancestor(&doc, to.node_id, lca_id)
            .ok_or(CommandError::Corrupted("to is not a descendant of LCA".into()))?;
        to_path.push(to.offset);

        tr.batch::<_, CommandError>(|tr| {
            delete_range(tr, &from_path, &to_path, lca_id)?;
            merge_after_delete(tr, from_tb, to_tb, lca_id)?;
            Ok(())
        })?;

        // Cursor positioning
        let cursor = if from_is_text {
            if tr.doc().node(from.node_id).is_some() {
                from
            } else if let Some((prev_id, next_id, parent_id, removed_index)) = cursor_fallback {
                resolve_cursor_after_removal(tr, prev_id, next_id, parent_id, removed_index)
            } else {
                Position {
                    node_id: lca_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                }
            }
        } else {
            // Block from — container position
            from
        };

        tr.set_selection(Selection::collapsed(cursor))?;
    }
```

- [ ] **Step 3: import 추가 + 기존 함수 삭제**

파일 상단 import에 `path_from_ancestor` 추가:

```rust
use crate::helpers::{find_ancestor_textblock, find_lowest_common_ancestor, path_from_ancestor};
```

기존 `trim_from`, `trim_to`, `collect_fully_selected` 함수 3개를 삭제한다 (현재 lines 161-312).

- [ ] **Step 4: 컴파일 확인**

Run: `cargo check -p editor-commands`

Expected: 컴파일 성공

- [ ] **Step 5: 기존 테스트 통과 확인**

Run: `cargo test -p editor-commands delete_selection`

Expected: 기존 7개 테스트 모두 PASS:
- `collapsed_selection_returns_false`
- `delete_within_text`
- `delete_entire_text_node`
- `delete_across_two_paragraphs`
- `delete_with_middle_paragraph`
- `delete_across_blockquotes_merges_containers`
- `delete_sole_content_leaves_empty_paragraph`

실패 시: 재귀 walk 로직의 path 계산 또는 순서 문제. 실패하는 테스트의 input/expected를 트레이스하여 디버깅.

---

### Task 4: Block Boundary 통합 테스트

**Files:**
- Modify: `crates/editor-commands/src/commands/delete_selection.rs` (tests 모듈)

- [ ] **Step 1: block from + inline to 테스트 작성**

```rust
#[test]
fn delete_block_from_inline_to() {
    let (initial, ..) = state! {
        doc { r: root {
            img: image
            paragraph { t1: text("Hello") }
        } }
        selection: (r, 0) -> (t1, 3)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    let (expected, ..) = state! {
        doc { root {
            paragraph { t1: text("lo") }
        } }
        selection: (t1, 0)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p editor-commands delete_block_from_inline_to`

Expected: PASS. 만약 selection 생성 구문(`r: root`)이 지원되지 않으면, 수동으로 State를 구성해야 할 수 있음. 그 경우 `NodeId::ROOT`를 사용:

```rust
// state! 매크로가 root 바인딩을 지원하지 않는 경우 대안
use editor_model::NodeId;

let (initial_state, ..) = state! {
    doc { root {
        img: image
        paragraph { t1: text("Hello") }
    } }
    selection: (img, 0)  // dummy, 아래서 교체
};
let initial = initial_state.with_selection(Selection::new(
    Position { node_id: NodeId::ROOT, offset: 0, affinity: Affinity::Downstream },
    Position { node_id: t1, offset: 3, affinity: Affinity::Downstream },
));
```

- [ ] **Step 3: inline from + block to 테스트 작성**

```rust
#[test]
fn delete_inline_from_block_to() {
    let (initial, ..) = state! {
        doc { r: root {
            paragraph { t1: text("Hello") }
            img: image
        } }
        selection: (t1, 2) -> (r, 2)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    let (expected, ..) = state! {
        doc { root {
            paragraph { t1: text("He") }
        } }
        selection: (t1, 2)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 4: 테스트 실행**

Run: `cargo test -p editor-commands delete_inline_from_block_to`

Expected: PASS

- [ ] **Step 5: block from + block to (same parent) 테스트 작성**

```rust
#[test]
fn delete_block_from_block_to_same_parent() {
    let (initial, ..) = state! {
        doc { r: root {
            paragraph { t0: text("Before") }
            img: image
            hr: horizontal_rule
            paragraph { t1: text("After") }
        } }
        selection: (r, 1) -> (r, 3)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    let (expected, ..) = state! {
        doc { root {
            paragraph { t0: text("Before") }
            paragraph { t1: text("After") }
        } }
        selection: (t1, 0)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 6: 테스트 실행**

Run: `cargo test -p editor-commands delete_block_from_block_to_same_parent`

Expected: PASS. 이 테스트는 from.node_id == to.node_id (둘 다 root)이므로 `delete_within_node` 경로로 처리됨. 커서가 `(t1, 0)` 이 아닌 container position으로 갈 수 있음 — Task 5에서 수정.

- [ ] **Step 7: block from + inline to with middle nodes 테스트**

```rust
#[test]
fn delete_block_from_inline_to_with_middle() {
    let (initial, ..) = state! {
        doc { r: root {
            img: image
            paragraph { t1: text("Middle") }
            paragraph { t2: text("Hello") }
        } }
        selection: (r, 0) -> (t2, 3)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    let (expected, ..) = state! {
        doc { root {
            paragraph { t2: text("lo") }
        } }
        selection: (t2, 0)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 8: 테스트 실행**

Run: `cargo test -p editor-commands delete_block_from_inline_to_with_middle`

Expected: PASS

- [ ] **Step 9: 전체 테스트 실행**

Run: `cargo test -p editor-commands delete_selection`

Expected: 기존 7개 + 새 4개 = 11개 모두 PASS

---

### Task 5: resolve_selection_at + 커서 위치

**Files:**
- Modify: `crates/editor-commands/src/commands/delete_selection.rs`

- [ ] **Step 1: block 삭제 후 인접 block node 선택 테스트**

```rust
#[test]
fn delete_block_cursor_selects_adjacent_block() {
    let (initial, ..) = state! {
        doc { r: root {
            hr1: horizontal_rule
            hr2: horizontal_rule
            hr3: horizontal_rule
        } }
        selection: (r, 1) -> (r, 3)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    // hr2, hr3 삭제 → hr1만 남음 → hr1 선택
    let (expected, ..) = state! {
        doc { r: root {
            hr1: horizontal_rule
            paragraph {}
        } }
        selection: (r, 0) -> (r, 1)
    };
    assert_state_eq!(&result, &expected);
}
```

Note: 이 테스트는 `delete_within_node` 경로. fulfill이 trailing Paragraph를 추가할 수 있음. 선택 결과가 hr1의 node selection `(r, 0) -> (r, 1)`이 되어야 함.

- [ ] **Step 2: block 삭제 후 인접 textblock으로 collapsed selection 테스트**

```rust
#[test]
fn delete_block_cursor_to_textblock() {
    let (initial, ..) = state! {
        doc { r: root {
            img: image
            paragraph { t1: text("Hello") }
        } }
        selection: (r, 0) -> (r, 1)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    let (expected, ..) = state! {
        doc { root {
            paragraph { t1: text("Hello") }
        } }
        selection: (t1, 0)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 3: 테스트 실행 — 실패 확인**

Run: `cargo test -p editor-commands delete_block_cursor`

Expected: FAIL — 현재 커서가 container position `(root, offset)`으로 반환됨

- [ ] **Step 4: resolve_selection_at 구현**

```rust
use editor_schema::NodeSpecExt;

/// Resolve a container position to the nearest valid selection.
/// For block-children containers, finds either:
/// - A node selection for an adjacent block-level leaf
/// - A collapsed selection at the nearest textblock position
fn resolve_selection_at(doc: &Doc, container_id: NodeId, offset: usize) -> Selection {
    let node = match doc.node(container_id) {
        Some(n) => n,
        None => return Selection::collapsed(Position::new(container_id, offset)),
    };
    let children = &node.entry().children;

    // Try child at offset (after deletion point)
    if let Some(&child_id) = children.get(offset) {
        if let Some(sel) = resolve_selection_for_child(doc, container_id, offset, child_id) {
            return sel;
        }
    }

    // Try child before offset
    if offset > 0 {
        if let Some(&child_id) = children.get(offset - 1) {
            if let Some(sel) = resolve_selection_for_child(doc, container_id, offset - 1, child_id) {
                return sel;
            }
        }
    }

    // Fallback: container position (shouldn't reach here after fulfill)
    Selection::collapsed(Position::new(container_id, offset))
}

fn resolve_selection_for_child(
    doc: &Doc,
    container_id: NodeId,
    child_index: usize,
    child_id: NodeId,
) -> Option<Selection> {
    let child = doc.node(child_id)?;
    let spec = child.node().spec();

    if spec.external || (spec.selectable && spec.content.is_leaf()) {
        // Block-level leaf → node selection
        Some(Selection::new(
            Position {
                node_id: container_id,
                offset: child_index,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: container_id,
                offset: child_index + 1,
                affinity: Affinity::Upstream,
            },
        ))
    } else {
        // Container/textblock → walk down to first/last text position
        find_first_text_position(doc, child_id).map(Selection::collapsed)
    }
}

fn find_first_text_position(doc: &Doc, node_id: NodeId) -> Option<Position> {
    let node = doc.node(node_id)?;
    if matches!(node.node(), Node::Text(_)) {
        return Some(Position::new(node_id, 0));
    }
    // If textblock with no children (empty paragraph)
    let spec = node.node().spec();
    let all_inline = spec.content.allowed_types().iter().all(|t| t.spec().inline);
    if !spec.content.is_leaf() && all_inline {
        return Some(Position::new(node_id, 0));
    }
    // Recurse into first child
    let first_child = *node.entry().children.first()?;
    find_first_text_position(doc, first_child)
}
```

- [ ] **Step 5: delete_within_node에 적용**

`delete_within_node`의 container 브랜치 (현재 `_ =>` arm)에서 반환값을 변경한다.

현재:
```rust
_ => {
    // ... remove children ...
    Ok(Position {
        node_id,
        offset: from_offset,
        affinity: Affinity::Downstream,
    })
}
```

이 함수의 반환 타입을 `Result<Position, CommandError>`에서 변경하면 영향 범위가 크므로, 대신 호출 지점(delete_selection)에서 처리한다.

`delete_selection`의 same-node 브랜치를 수정:

```rust
if from.node_id == to.node_id {
    let cursor = delete_within_node(tr, from.node_id, from.offset, to.offset)?;
    // Check if cursor is in a block-children container
    let doc = tr.doc();
    if let Some(node) = doc.node(cursor.node_id) {
        if !matches!(node.node(), Node::Text(_)) {
            let spec = node.node().spec();
            let all_inline = spec.content.allowed_types().iter().all(|t| t.spec().inline);
            if !(!spec.content.is_leaf() && all_inline) {
                // Not a textblock — resolve to valid selection
                let sel = resolve_selection_at(&doc, cursor.node_id, cursor.offset);
                tr.set_selection(sel)?;
                return Ok(true);
            }
        }
    }
    tr.set_selection(Selection::collapsed(cursor))?;
}
```

- [ ] **Step 6: multi-node 경로에 적용**

multi-node 경로의 커서 결정 부분에서 block from일 때 `resolve_selection_at` 사용:

```rust
        // Cursor positioning
        let selection = if from_is_text {
            if tr.doc().node(from.node_id).is_some() {
                Selection::collapsed(from)
            } else if let Some((prev_id, next_id, parent_id, removed_index)) = cursor_fallback {
                Selection::collapsed(resolve_cursor_after_removal(
                    &tr, prev_id, next_id, parent_id, removed_index,
                ))
            } else {
                Selection::collapsed(Position {
                    node_id: lca_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                })
            }
        } else {
            resolve_selection_at(&tr.doc(), from.node_id, from.offset)
        };

        tr.set_selection(selection)?;
```

- [ ] **Step 7: 테스트 실행**

Run: `cargo test -p editor-commands delete_selection`

Expected: 모든 테스트 PASS (기존 + Task 4 + Task 5의 커서 테스트)

---

### Task 6: Fulfill 범위 확장 + 엣지 케이스

**Files:**
- Modify: `crates/editor-commands/src/commands/delete_selection.rs`

- [ ] **Step 1: merge 불발 확인 테스트**

```rust
#[test]
fn delete_block_from_does_not_merge_adjacent_paragraphs() {
    let (initial, ..) = state! {
        doc { r: root {
            paragraph { t0: text("Before") }
            img: image
            paragraph { t1: text("Hello") }
        } }
        selection: (r, 1) -> (t1, 3)
    };
    let (result, ..) = transact!(initial, |tr| delete_selection(&mut tr));
    // image 삭제 + "Hel" 삭제 → "Before"와 "lo"는 merge 안 됨
    let (expected, ..) = state! {
        doc { root {
            paragraph { t0: text("Before") }
            paragraph { t1: text("lo") }
        } }
        selection: (t1, 0)
    };
    assert_state_eq!(&result, &expected);
}
```

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p editor-commands delete_block_from_does_not_merge`

Expected: PASS

- [ ] **Step 3: fulfill 범위 확장**

`merge_after_delete`의 fulfill 호출 부분을 확장한다. 기존 fulfill(lca) 호출 직전에 from/to의 ancestor chain에도 fulfill을 적용:

```rust
    // Extended fulfill: also fulfill nodes on the from/to path that may be empty
    let doc = tr.doc();
    if let Some(from_tb) = from_tb_original {
        if let Some(node) = doc.node(from_tb) {
            if let Some(parent) = node.parent() {
                if parent.entry().children.is_empty() {
                    tr.apply_steps(prune(&parent))?;
                }
            }
        }
    }

    let doc = tr.doc();
    if let Some(lca) = doc.node(lca_id) {
        tr.apply_steps(fulfill(&lca))?;
    }
```

Note: `from_tb_original`을 받기 위해 merge_after_delete에 추가 파라미터가 필요할 수 있음. 또는 delete_selection에서 별도로 cleanup을 수행. 이 부분은 구현 시 정확한 구조를 결정한다.

- [ ] **Step 4: 전체 테스트 실행**

Run: `cargo test -p editor-commands delete_selection`

Expected: 모든 테스트 PASS

- [ ] **Step 5: 최종 확인**

Run: `cargo test -p editor-commands`

Expected: editor-commands crate의 모든 테스트 PASS
