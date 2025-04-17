import type { GraphQLOperation } from '../types';

export const addOperationMeta = (operation: GraphQLOperation, meta: Record<string, unknown>): GraphQLOperation => {
  return { ...operation, context: { ...operation.context, _meta: { ...operation.context._meta, ...meta } } };
};
