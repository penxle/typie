import { error, redirect } from '@sveltejs/kit';
import { fetch } from '@tauri-apps/plugin-http';
import { cacheExchange, createClient, errorExchange, fetchExchange, GraphQLError, NetworkError, wsExchange } from '@typie/sark';
import ky from 'ky';
import { TypieError } from '@/errors';
import { browser } from '$app/environment';
import { PUBLIC_API_URL, PUBLIC_WS_URL } from '$env/static/public';
import { store } from '$lib/store';

// eslint-disable-next-line import/no-default-export
export default createClient({
  url: `${PUBLIC_API_URL}/graphql`,
  fetchFn: fetch,
  fetchOptions: async () => {
    const accessToken = await store.get('access_token');
    if (!accessToken) {
      return {};
    }

    return {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    };
  },
  exchanges: [
    errorExchange((error) => {
      if (error instanceof GraphQLError && error.extensions?.type === 'TypieError') {
        return new TypieError({
          code: error.extensions.code as string,
          message: error.message,
          status: error.extensions.status as number,
        });
      }

      return error;
    }),
    cacheExchange(),
    fetchExchange(),
    ...(browser
      ? [
          wsExchange(`${PUBLIC_WS_URL}/graphql`, {
            connectionParams: async () => {
              const resp = await ky
                .post(`/graphql`, {
                  json: {
                    operationName: 'WsExchange_CreateWsSession_Mutation',
                    query: /* GraphQL */ `
                      mutation WsExchange_CreateWsSession_Mutation {
                        createWsSession
                      }
                    `,
                  },
                })
                .json<{ data: { createWsSession: string } }>();

              return {
                session: resp.data.createWsSession,
              };
            },
          }),
        ]
      : []),
  ],
  onError: (err, event) => {
    if (err instanceof TypieError) {
      error(err.status, {
        message: err.message,
        code: err.code,
      });
    }

    if (err instanceof NetworkError) {
      if (err.statusCode === 401) {
        redirect(302, event.url.href);
      }

      error(err.statusCode ?? 500, {
        message: err.message,
      });
    }

    if (err instanceof GraphQLError) {
      error(500, {
        message: err.message,
        code: err.extensions?.code as string | undefined,
        eventId: err.extensions?.eventId as string | undefined,
      });
    }
  },
});
