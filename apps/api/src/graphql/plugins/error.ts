import { isAsyncIterable } from '@envelop/core';
import * as Sentry from '@sentry/bun';
import { logger } from '@typie/lib';
import { GraphQLError } from 'graphql';
import { dev } from '@/env';
import { TypieError } from '@/errors';
import type { AsyncIterableIteratorOrValue, ExecutionResult } from '@envelop/core';
import type { Plugin } from 'graphql-yoga';

class UnexpectedError extends GraphQLError {
  public eventId: string;

  constructor(error: Error) {
    const eventId = Sentry.captureException(error);
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

const transformError = (error: unknown): GraphQLError => {
  if (error instanceof TypieError) {
    return error;
  } else if (error instanceof GraphQLError && error.originalError) {
    return transformError(error.originalError);
  } else if (error instanceof Error) {
    logger.error(error);
    return new UnexpectedError(error);
  } else {
    logger.error(error);
    return new UnexpectedError(new Error(String(error)));
  }
};

type ErrorHandlerPayload = { error: unknown; setError: (err: unknown) => void };
const errorHandler = ({ error, setError }: ErrorHandlerPayload) => {
  setError(transformError(error));
};

type ResultHandlerPayload<T> = { result: T; setResult: (result: T) => void };
const resultHandler = ({ result, setResult }: ResultHandlerPayload<AsyncIterableIteratorOrValue<ExecutionResult>>) => {
  const handler = ({ result, setResult }: ResultHandlerPayload<ExecutionResult>) => {
    if (result.errors) {
      setResult({
        ...result,
        errors: result.errors.map((error) => transformError(error)),
      });
    }
  };

  if (isAsyncIterable(result)) {
    return { onNext: handler };
  } else {
    handler({ result, setResult });
    return;
  }
};

export const useError = (): Plugin => {
  return {
    onPluginInit: ({ registerContextErrorHandler }) => {
      registerContextErrorHandler(errorHandler);
    },
    onExecute: () => ({
      onExecuteDone: resultHandler,
    }),
    onSubscribe: () => ({
      onSubscribeResult: resultHandler,
      onSubscribeError: errorHandler,
    }),
  };
};
