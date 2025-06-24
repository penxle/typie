import { logger } from '@typie/lib';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

const log = logger.getChild('graphql');

export const useLogger = (): Plugin<Context> => ({
  onExecute: ({ args }) => {
    log.info('Executed operation {*}', {
      operationName: args.operationName,
      ip: args.contextValue.ip,
      userId: args.contextValue.session?.userId,
    });
  },
});
