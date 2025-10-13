import type { Source } from 'wonka';
import type { $ArtifactSchema, ArtifactSchema } from '../types';

export class NetworkError extends Error {
  override readonly name = 'NetworkError';
  readonly statusCode?: number;

  constructor({ message, statusCode }: { message: string; statusCode?: number }) {
    super(message);
    this.statusCode = statusCode;
  }
}

export class GraphQLError extends Error {
  override readonly name = 'GraphQLError';
  readonly path?: readonly (string | number)[];
  readonly extensions?: Record<string, unknown>;

  constructor({
    message,
    path,
    extensions,
  }: {
    message: string;
    path?: readonly (string | number)[];
    extensions?: Record<string, unknown>;
  }) {
    super(message);
    this.path = path;
    this.extensions = extensions;
  }
}

export type OperationContext = {
  url: string;
  fetch?: typeof globalThis.fetch;
  fetchOptions?: RequestInit;
  fetchOptionsFn?: () => Promise<RequestInit>;
  requestPolicy: 'cache-only' | 'network-only' | 'cache-first';
  transport: 'fetch' | 'sse' | 'ws';
  extensions?: Record<string, unknown>;
  optimistic?: Record<string, unknown>;
  _meta?: Record<string, unknown>;
};

export type GraphQLOperation<T extends $ArtifactSchema = $ArtifactSchema> = {
  key: string;
  type: 'query' | 'mutation' | 'subscription';
  schema: ArtifactSchema<T>;
  variables: T['$input'];
  context: OperationContext;
};

export type ExchangeOperation = {
  key: string;
  type: 'teardown';
};

export type Operation = GraphQLOperation | ExchangeOperation;

export type OperationError = NetworkError | GraphQLError | unknown;

export type OperationResult<T extends $ArtifactSchema = $ArtifactSchema> =
  | {
      type: 'data';
      operation: GraphQLOperation<T>;
      data: T['$output'];
    }
  | {
      type: 'error';
      operation: GraphQLOperation;
      error: OperationError;
    };

export type ExchangeInput = {
  forward: ExchangeIO;
};

export type ExchangeIO = (ops$: Source<Operation>) => Source<OperationResult>;

export type Exchange = (input: ExchangeInput) => ExchangeIO;
