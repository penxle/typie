use crate::schema::{DurableSchema, TypeSchema};
use crate::types::*;

pub fn all_type_schemas() -> Vec<TypeSchema> {
    vec![
        DurableBias::schema(),
        DurableAnchor::schema(),
        DurableAlignment::schema(),
        DurableBlockquoteVariant::schema(),
        DurableCalloutVariant::schema(),
        DurableHorizontalRuleVariant::schema(),
        DurableLayoutMode::schema(),
        DurableTableBorderStyle::schema(),
        DurableModifier::schema(),
        DurableModifierKind::schema(),
        DurableAttr::schema(),
        DurableNodeType::schema(),
        DurableItem::schema(),
        DurableAliasRun::schema(),
        DurableOp::schema(),
    ]
}
