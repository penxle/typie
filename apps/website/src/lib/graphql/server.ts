import { AggregatedError, cacheExchange, createClient, httpExchange, isExchangeError, isGraphQLError } from '@mearie/svelte';
import { error, redirect } from '@sveltejs/kit';
import { TypieError } from '@typie/lib/errors';
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { schema } from '$mearie';
import { mearieClient, scalars } from './client';
import { errorExchange } from './error';
import type { Artifact, CacheSnapshot, DataOf, VariablesOf } from '@mearie/svelte';

export type HydratableQuery<T extends Artifact<'query'>> = {
  data: DataOf<T>;
  ' $hydration': {
    artifact: T;
    variables: VariablesOf<T>;
    cacheSnapshot?: CacheSnapshot;
  };
};

export async function loadQuery<T extends Artifact<'query'>>(
  event: { fetch: typeof fetch; url: URL },
  query: T,
  variables?: VariablesOf<T>,
): Promise<HydratableQuery<T>> {
  const client = browser
    ? mearieClient
    : createClient({
        schema,
        exchanges: [
          errorExchange(),
          cacheExchange(),
          httpExchange({
            url: '/graphql',
            fetch: event.fetch,
            credentials: 'include',
          }),
        ],
        scalars,
      });

  try {
    const data: DataOf<T> = await (client.query as (q: T, v?: VariablesOf<T>) => Promise<DataOf<T>>)(query, variables);
    const cacheSnapshot = browser ? undefined : client.extension('cache').extract();

    return {
      data,
      ' $hydration': { artifact: query, variables: variables ?? ({} as VariablesOf<T>), cacheSnapshot },
    };
  } catch (err) {
    if (err instanceof AggregatedError) {
      for (const inner of err.errors) {
        if (inner instanceof TypieError) {
          if (inner.status === 401) {
            redirect(302, `${env.PUBLIC_AUTH_URL}/login`);
          }

          error(inner.status, { message: inner.message, code: inner.code });
        }

        if (isExchangeError(inner, 'http')) {
          const status = inner.extensions?.statusCode;
          if (status === 401) {
            redirect(302, event.url.href);
          }

          if (inner.cause instanceof TypeError) {
            error(503, { code: 'network_error', message: '서버에 연결할 수 없어요.' });
          }

          if (status === 502 || status === 503 || status === 504) {
            error(status, { code: 'service_unavailable', message: '서버를 일시적으로 사용할 수 없어요.' });
          }
        }

        if (isGraphQLError(inner)) {
          error(500, {
            message: inner.message,
            code: inner.extensions?.code as string | undefined,
            eventId: inner.extensions?.eventId as string | undefined,
          });
        }
      }
    }

    throw err;
  }
}
