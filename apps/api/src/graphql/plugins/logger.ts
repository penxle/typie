import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { handleStreamOrSingleExecutionResult } from 'graphql-yoga';
import { truncateVariables } from './utils.ts';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '#/context.ts';

const log = logger.getChild('graphql');

export const useLogger = (): Plugin<Context> => ({
  onExecute: ({ args }) => {
    const operationName = args.operationName ?? 'anonymous';
    Sentry.getCurrentScope().setTransactionName(`POST /graphql (${operationName})`);

    return {
      onExecuteDone: (payload) =>
        handleStreamOrSingleExecutionResult(payload, ({ result }) => {
          if (result.errors?.some((e) => e.extensions?.code === 'RATE_LIMITED')) {
            return;
          }
          log.info('Executing operation {*}', {
            operationName: args.operationName,
            variables: truncateVariables(args.variableValues),
            ip: args.contextValue.ip,
            userId: args.contextValue.session?.userId,
          });
        }),
    };
  },
});
