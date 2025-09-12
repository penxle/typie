import { logger } from '@typie/lib';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

const log = logger.getChild('graphql');

const truncateVariables = (variables: Record<string, unknown> | null | undefined) => {
  if (!variables) return variables;

  const truncated: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(variables)) {
    if (typeof value === 'string' && value.length > 100) {
      truncated[key] = value.slice(0, 100) + `... (${value.length} bytes)`;
    } else if (value && typeof value === 'object') {
      truncated[key] = truncateVariables(value as Record<string, unknown>);
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
