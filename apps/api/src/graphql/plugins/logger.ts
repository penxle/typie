import { logger } from '@typie/lib';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

const log = logger.getChild('graphql');

const truncateVariables = (variables: Record<string, unknown> | null | undefined, depth = 0) => {
  if (!variables) return variables;
  if (depth >= 2) return '...';

  const truncated: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(variables)) {
    if (typeof value === 'string' && value.length > 100) {
      truncated[key] = value.slice(0, 100) + `... (${value.length} bytes)`;
    } else if (Array.isArray(value)) {
      if (depth >= 1) {
        truncated[key] = `[...${value.length} items]`;
      } else {
        truncated[key] = value.map((item) =>
          item && typeof item === 'object' ? truncateVariables(item as Record<string, unknown>, depth + 1) : item,
        );
      }
    } else if (value && typeof value === 'object') {
      truncated[key] = truncateVariables(value as Record<string, unknown>, depth + 1);
    } else {
      truncated[key] = value;
    }
  }

  return truncated;
};

export const useLogger = (): Plugin<Context> => ({
  onExecute: ({ args }) => {
    log.info('Executing operation {*}', {
      operationName: args.operationName,
      variables: truncateVariables(args.variableValues),
      ip: args.contextValue.ip,
      userId: args.contextValue.session?.userId,
    });
  },
});
