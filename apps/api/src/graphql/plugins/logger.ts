import { logger } from '@typie/lib';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

export const useLogger = (): Plugin<Context> => ({
  onExecute: ({ args }) => {
    logger.info('Executed operation: {operation_name} ({variables}), by user {user} from {ip}', {
      ip: args.contextValue.ip,
      user: args.contextValue.session?.userId,
      operation_name: args.operationName,
      variables: args.variableValues,
    });
  },
});
