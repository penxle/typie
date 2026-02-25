import { cacheExchange, createClient, dedupExchange, httpExchange, subscriptionExchange } from '@mearie/svelte';
import { createClient as createWsClient } from 'graphql-ws';
import ky from 'ky';
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { schema } from '$mearie';
import { errorExchange } from './error';

export const scalars = {
  JSON: { parse: (v: unknown) => v, serialize: (v: unknown) => v },
  Binary: { parse: (v: unknown) => v as string, serialize: (v: string) => v },
  DateTime: { parse: (v: unknown) => v as string, serialize: (v: string) => v },
  BigInt: { parse: (v: unknown) => v as string, serialize: (v: string) => v },
};

export const mearieClient = createClient({
  schema,
  exchanges: [
    errorExchange(),
    dedupExchange(),
    cacheExchange({ fetchPolicy: 'cache-and-network' }),
    ...(browser
      ? [
          subscriptionExchange({
            client: createWsClient({
              url: `${env.PUBLIC_WS_URL}/graphql`,
              connectionParams: async () => {
                const resp = await ky
                  .post('/graphql', {
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
          }),
        ]
      : []),
    httpExchange({ url: '/graphql', credentials: 'include' }),
  ],
  scalars,
});

export const cache = mearieClient.extension('cache');
