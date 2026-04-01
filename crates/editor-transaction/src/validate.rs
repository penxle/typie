use editor_model::{Doc, ModifierType, NodeId};
use editor_schema::{ContextExpr, ModifierSpecExt, NodeSpecExt};

use crate::StepError;

/// node_id의 children이 해당 노드의 content expression을 만족하는지 검증.
pub(crate) fn validate_content(doc: &Doc, node_id: NodeId) -> Result<(), StepError> {
    let node = doc.node(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    let spec = node.spec();
    let child_types: Vec<_> = node.children().map(|c| c.as_type()).collect();
    spec.content
        .validate(&child_types)
        .map_err(|e| StepError::ContentViolation {
            node_id,
            detail: e.to_string(),
        })
}

/// node_id의 위치가 해당 노드의 context expression을 만족하는지 검증.
/// path는 [Root, ..., Parent, Self] 순서.
pub(crate) fn validate_context(doc: &Doc, node_id: NodeId) -> Result<(), StepError> {
    let node = doc.node(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    let spec = node.spec();

    if spec.context == ContextExpr::Any {
        return Ok(());
    }

    let path: Vec<_> = node
        .ancestors()
        .map(|n| n.as_type())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if !spec.context.matches(&path) {
        return Err(StepError::ContextViolation {
            node_id,
            detail: format!("{:?} not allowed in context {:?}", node.as_type(), path),
        });
    }

    Ok(())
}

/// node_id와 그 하위 모든 노드의 context를 검증.
/// MoveNode 등 서브트리의 ancestor path가 변경되는 경우 사용.
pub(crate) fn validate_context_deep(doc: &Doc, node_id: NodeId) -> Result<(), StepError> {
    validate_context(doc, node_id)?;

    let node = doc.node(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    for desc in node.descendants() {
        let spec = desc.spec();
        if spec.context != ContextExpr::Any {
            validate_context(doc, desc.id())?;
        }
    }

    Ok(())
}

/// ModifierType으로 modifier context를 검증.
/// Validation::Modifier에서 사용 — Modifier 인스턴스 없이 타입만으로 검증.
pub(crate) fn validate_modifier_context_by_type(
    doc: &Doc,
    node_id: NodeId,
    modifier_type: ModifierType,
) -> Result<(), StepError> {
    let node = doc.node(node_id).ok_or(StepError::NodeNotFound(node_id))?;
    let spec = modifier_type.spec();

    if spec.context == ContextExpr::Any {
        return Ok(());
    }

    let path: Vec<_> = node
        .ancestors()
        .map(|n| n.as_type())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if !spec.context.matches(&path) {
        return Err(StepError::ModifierContextViolation {
            node_id,
            modifier_type,
            detail: format!(
                "{:?} not allowed on {:?} in context {:?}",
                modifier_type,
                node.as_type(),
                path
            ),
        });
    }

    Ok(())
}
