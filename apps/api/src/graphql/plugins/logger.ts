import * as Sentry from '@sentry/bun';
import { logger } from '@typie/lib';
import { handleStreamOrSingleExecutionResult } from 'graphql-yoga';
import { truncateVariables } from './utils';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

const log = logger.getChild('graphql');

export const useLogger = (): Plugin<Context> => ({
  onExecute: ({ args }) => {
    const operationName = args.operationName ?? 'anonymous';
    Sentry.getCurrentScope().setTransactionName(`POST /graphql (${operationName})`);

    return {
      onExecuteDone(payload) {
        return handleStreamOrSingleExecutionResult(payload, ({ result }) => {
          if (result.errors?.some((e) => e.extensions?.code === 'RATE_LIMITED')) {
            return;
          }
          log.info('Executing operation {*}', {
            operationName: args.operationName,
            variables: truncateVariables(args.variableValues),
            ip: args.contextValue.ip,
            userId: args.contextValue.session?.userId,
          });
        });
      },
    };
  },
});
