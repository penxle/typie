use crate::{ContextExpr, Doc, ModelError, ModifierType, NodeId};

pub fn validate_content(doc: &Doc, node_id: NodeId) -> Result<(), ModelError> {
    let node = doc.node(node_id).ok_or(ModelError::NodeNotFound(node_id))?;
    let spec = node.spec();
    let child_types: Vec<_> = node.children().map(|c| c.as_type()).collect();
    spec.content
        .validate(&child_types)
        .map_err(|e| ModelError::ContentViolation {
            node_id,
            detail: e.to_string(),
        })
}

// Path passed to context.matches is ordered [Root, ..., Parent, Self].
pub fn validate_context(doc: &Doc, node_id: NodeId) -> Result<(), ModelError> {
    let node = doc.node(node_id).ok_or(ModelError::NodeNotFound(node_id))?;
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
        return Err(ModelError::ContextViolation {
            node_id,
            detail: format!("{:?} not allowed in context {:?}", node.as_type(), path),
        });
    }

    Ok(())
}

pub fn validate_context_deep(doc: &Doc, node_id: NodeId) -> Result<(), ModelError> {
    validate_context(doc, node_id)?;

    let node = doc.node(node_id).ok_or(ModelError::NodeNotFound(node_id))?;
    for desc in node.descendants() {
        let spec = desc.spec();
        if spec.context != ContextExpr::Any {
            validate_context(doc, desc.id())?;
        }
    }

    Ok(())
}

pub fn validate_modifier_context_by_type(
    doc: &Doc,
    node_id: NodeId,
    modifier_type: ModifierType,
) -> Result<(), ModelError> {
    let node = doc.node(node_id).ok_or(ModelError::NodeNotFound(node_id))?;
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
        return Err(ModelError::ModifierContextViolation {
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
