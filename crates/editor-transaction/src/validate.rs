use editor_model::{Doc, ModifierType, NodeId};
use editor_schema::{ContextExpr, ModifierSpecExt, NodeSpecExt};

use crate::StepError;

/// Validates that `node_id`'s children satisfy its content expression.
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

/// Validates that `node_id` is placed in an allowed context. Path is ordered `[Root, ..., Parent, Self]`.
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

/// Validates context for `node_id` and all its descendants. Use when a subtree's ancestor path changes (e.g. MoveNode).
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

/// Validates modifier context by type only, without a Modifier instance. Used by `Validation::Modifier`.
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
