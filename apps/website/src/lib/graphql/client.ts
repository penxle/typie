import { cacheExchange, createClient, dedupExchange, httpExchange, setClient, subscriptionExchange } from '@mearie/svelte';
import { createClient as createWsClient } from 'graphql-ws';
import ky from 'ky';
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { schema } from '$mearie';
import { errorExchange } from './error';
import { wsStatus } from './ws-status.svelte';
import type { Client as WsClient } from 'graphql-ws';

const WS_KEEP_ALIVE_MS = 15_000;
const WS_PONG_TIMEOUT_MS = 20_000;
const WS_ACK_TIMEOUT_MS = 10_000;
const WS_RETRY_MAX_MS = 10_000;

class WsSessionError extends Error {}

const createSocketClient = () => {
  let client: WsClient | null = null;
  let pongTimer: ReturnType<typeof setTimeout> | null = null;

  const clearPongTimer = () => {
    if (pongTimer !== null) {
      clearTimeout(pongTimer);
      pongTimer = null;
    }
  };

  client = createWsClient({
    url: `${env.PUBLIC_WS_URL}/graphql`,
    keepAlive: WS_KEEP_ALIVE_MS,
    connectionAckWaitTimeout: WS_ACK_TIMEOUT_MS,
    retryAttempts: Infinity,
    retryWait: (retries) =>
      new Promise((resolve) => setTimeout(resolve, Math.min(1000 * 2 ** retries, WS_RETRY_MAX_MS) + Math.random() * 1000)),
    shouldRetry: (err) => !(err instanceof WsSessionError),
    on: {
      connecting: () => wsStatus.set('connecting'),
      connected: () => wsStatus.set('connected'),
      closed: () => {
        clearPongTimer();
        wsStatus.set('closed');
      },
      error: () => {
        clearPongTimer();
        wsStatus.set('closed');
      },
      ping: (received) => {
        if (received) {
          return;
        }

        clearPongTimer();
        pongTimer = setTimeout(() => client?.terminate(), WS_PONG_TIMEOUT_MS);
      },
      pong: (received) => {
        if (received) {
          clearPongTimer();
        }
      },
    },
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
        .json<{ data?: { createWsSession?: string } | null }>();

      const session = resp.data?.createWsSession;
      if (!session) {
        throw new WsSessionError('ws session unavailable');
      }

      return { session };
    },
  });

  return client;
};

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
    ...(browser ? [subscriptionExchange({ client: createSocketClient() })] : []),
    httpExchange({ url: '/graphql', credentials: 'include' }),
  ],
  scalars,
});

export const cache = mearieClient.extension('cache');

export function setupMearieContext() {
  if (browser) {
    setClient(mearieClient);
  } else {
    setClient(createClient({ schema, exchanges: [cacheExchange()], scalars }));
  }
}
