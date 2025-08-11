import { nanoid } from 'nanoid';
import { filter, makeSubject, onEnd, onStart, pipe, publish, share } from 'wonka';
import { composeExchanges } from '../exchanges';
import type { LoadEvent } from '@sveltejs/kit';
import type { Source, Subject } from 'wonka';
import type { $ArtifactSchema, ArtifactSchema } from '../../types';
import type { Exchange, GraphQLOperation, Operation, OperationContext, OperationResult } from '../types';

export type ClientOptions = {
  url: string;
  fetchFn?: typeof globalThis.fetch;
  fetchOptions?: RequestInit | (() => Promise<RequestInit>);
  exchanges: Exchange[];
  onError?: (error: unknown, event: LoadEvent) => void | Promise<void>;
};

type CreateOperationParams<T extends $ArtifactSchema> = {
  schema: ArtifactSchema<T>;
  variables: T['$input'];
  context?: Partial<OperationContext>;
};

type $OperationArtifactSchema = $ArtifactSchema<'query' | 'mutation' | 'subscription'>;

export class SarkClient {
  id = nanoid();

  private url: string;
  private fetchFn: typeof globalThis.fetch | undefined;
  private fetchOptions: RequestInit | (() => Promise<RequestInit>);
  private onError?: (error: unknown, event: LoadEvent) => void | Promise<void>;

  private operations$: Subject<Operation>;
  private result$: Source<OperationResult>;

  constructor(options: ClientOptions) {
    this.url = options.url;
    this.fetchFn = options.fetchFn;
    this.fetchOptions = options.fetchOptions ?? {};
    this.onError = options.onError;

    const composedExchange = composeExchanges(options.exchanges);
    const forward = composedExchange({
      forward: (ops$) => {
        return pipe(
          ops$,
          filter((op) => op.type !== 'teardown'),
          filter((_): _ is never => false),
        );
      },
    });

    this.operations$ = makeSubject<Operation>();
    this.result$ = share(forward(this.operations$.source));

    publish(this.result$);
  }

  createOperation<T extends $OperationArtifactSchema>({ schema, variables, context }: CreateOperationParams<T>): GraphQLOperation<T> {
    return {
      key: nanoid(),
      type: schema.kind,
      schema,
      variables,
      context: {
        url: context?.url ?? this.url,
        fetch: this.fetchFn ?? context?.fetch,
        fetchOptions: { ...(typeof this.fetchOptions === 'object' ? this.fetchOptions : {}), ...context?.fetchOptions },
        fetchOptionsFn: typeof this.fetchOptions === 'function' ? this.fetchOptions : undefined,
        requestPolicy: context?.requestPolicy ?? 'cache-first',
        transport: context?.transport ?? 'fetch',
        extensions: context?.extensions,
        _meta: context?._meta ?? {},
      },
    };
  }

  executeOperation = <T extends $OperationArtifactSchema>(operation: GraphQLOperation<T>): Source<OperationResult> => {
    return pipe(
      this.result$,
      filter((result) => result.operation.key === operation.key),
      onStart(() => this.operations$.next(operation)),
      onEnd(() => this.operations$.next({ key: operation.key, type: 'teardown' as const })),
      share,
    );
  };

  handleError = async (error: unknown, event: LoadEvent) => {
    await this.onError?.(error, event);
  };
}

export const createClient = (options: ClientOptions): (() => SarkClient) => {
  return () => new SarkClient(options);
};
