import type { BlockState, HorizontalRuleVariant, Message } from '@typie/editor-ffi/browser';

export const createHorizontalRuleVariantMessage = (blockState: BlockState | undefined, variant: HorizontalRuleVariant): Message => {
  const nodes = blockState?.nodes ?? [];
  const target = nodes.length === 1 && nodes[0].node.type === 'horizontal_rule' ? nodes[0] : undefined;
  if (target) {
    return { type: 'node', op: { type: 'set_attrs', id: target.id, attrs: { type: 'horizontal_rule', variant } } };
  }
  return { type: 'insertion', op: { type: 'fragment', fragment: { node: { type: 'horizontal_rule', variant } } } };
};
