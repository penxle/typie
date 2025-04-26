import { error, redirect } from '@sveltejs/kit';
import { cacheExchange, createClient, errorExchange, fetchExchange, GraphQLError, NetworkError, wsExchange } from '@typie/sark';
import ky from 'ky';
import { TypieError } from '@/errors';
import { env } from '$env/dynamic/public';

// eslint-disable-next-line import/no-default-export
export default createClient({
  url: `/graphql`,
  fetchOptions: {
    credentials: 'include',
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
    wsExchange(`${env.PUBLIC_API_URL}/graphql`, {
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
      retryAttempts: Infinity,
    }),
  ],
  onError: (err, event) => {
    if (err instanceof TypieError) {
      error(err.status, { message: err.message });
    }

    if (err instanceof NetworkError) {
      if (err.statusCode === 401) {
        redirect(302, event.url.href);
      }

      error(err.statusCode ?? 500, { message: err.message });
    }
  },
});
