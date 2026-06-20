import { isAsyncIterable } from '@envelop/core';
import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { TypieError } from '@typie/lib/errors';
import { GraphQLError, print } from 'graphql';
import { dev } from '#/env.ts';
import { truncateVariables } from './utils.ts';
import type { AsyncIterableIteratorOrValue, ExecutionResult } from '@envelop/core';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '#/context.ts';

const log = logger.getChild('graphql');

type OperationInfo = {
  operationName?: string | null;
  variableValues?: unknown;
  query?: string;
  userId?: string;
  ip?: string;
};

class UnexpectedError extends GraphQLError {
  public eventId: string;

  constructor(error: Error, operation?: OperationInfo) {
    const eventId = Sentry.captureException(error, {
      tags: {
        ...(operation?.operationName && { 'graphql.operation': operation.operationName }),
      },
      contexts: {
        ...(operation && {
          graphql: {
            operationName: operation.operationName,
            variables: operation.variableValues,
            query: operation.query,
          },
        }),
      },
      user:
        operation?.userId || operation?.ip
          ? {
              ...(operation.userId && { id: operation.userId }),
              ...(operation.ip && { ip_address: operation.ip }),
            }
          : undefined,
    });
    const originalError = dev ? error : undefined;

    super(dev ? error.message : 'Unexpected error', {
      extensions: {
        type: 'UnexpectedError',
        code: dev ? 'unexpected_error_dev' : 'unexpected_error',
        eventId,
        originalError,
      },
      originalError,
    });

    this.eventId = eventId;
  }
}

const transformError = (error: unknown, operation?: OperationInfo): GraphQLError => {
  if (error instanceof TypieError) {
    return error;
  }
  if (error instanceof GraphQLError && error.extensions?.code === 'RATE_LIMITED') {
    return error;
  }
  if (error instanceof GraphQLError && error.originalError) {
    return transformError(error.originalError, operation);
  }
  if (error instanceof Error) {
    log.error('Unexpected error {*}', { error });
    return new UnexpectedError(error, operation);
  }
  log.error('Unexpected error {*}', { error });
  return new UnexpectedError(new Error(String(error)), operation);
};

type ErrorHandlerPayload = { error: unknown; setError: (err: unknown) => void };
const contextErrorHandler = ({ error, setError }: ErrorHandlerPayload) => {
  setError(transformError(error));
};

type ResultHandlerPayload<T> = { result: T; setResult: (result: T) => void };
const createResultHandler = (operation: OperationInfo) => {
  return ({ result, setResult }: ResultHandlerPayload<AsyncIterableIteratorOrValue<ExecutionResult>>) => {
    const handler = ({ result, setResult }: ResultHandlerPayload<ExecutionResult>) => {
      if (result.errors) {
        setResult({
          ...result,
          errors: result.errors.map((error) => transformError(error, operation)),
        });
      }
    };

    if (isAsyncIterable(result)) {
      return { onNext: handler };
    }
    handler({ result, setResult });
    return;
  };
};

const extractOperationInfo = (args: {
  operationName?: string | null;
  variableValues?: Readonly<Record<string, unknown>> | null;
  document: Parameters<typeof print>[0];
  contextValue?: unknown;
}): OperationInfo => {
  const context = args.contextValue as Context | undefined;
  return {
    operationName: args.operationName,
    variableValues: truncateVariables(args.variableValues as Record<string, unknown> | null | undefined),
    query: print(args.document),
    userId: context?.session?.userId,
    ip: context?.ip,
  };
};

export const useError = (): Plugin<Context> => {
  return {
    onPluginInit: ({ registerContextErrorHandler }) => {
      registerContextErrorHandler(contextErrorHandler);
    },
    onExecute: ({ args }) => {
      const operation = extractOperationInfo(args);
      return {
        onExecuteDone: createResultHandler(operation),
      };
    },
    onSubscribe: ({ args }) => {
      const operation = extractOperationInfo(args);
      return {
        onSubscribeResult: createResultHandler(operation),
        onSubscribeError: ({ error, setError }: ErrorHandlerPayload) => {
          setError(transformError(error, operation));
        },
      };
    },
  };
};
