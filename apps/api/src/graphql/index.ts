import { getClientAddress } from '@typie/lib';
import { GraphQLError } from 'graphql';
import { CloseCode, makeServer } from 'graphql-ws';
import { createYoga, useExecutionCancellation } from 'graphql-yoga';
import { Hono } from 'hono';
import { checkBootstrap } from '#/bootstrap.ts';
import { redis } from '#/cache.ts';
import { upgradeWebSocket } from '#/ws.ts';
import { useError } from './plugins/error.ts';
import { useLogger } from './plugins/logger.ts';
import { useRateLimit } from './plugins/rate-limit.ts';
import { schema } from './schema.ts';
import type { Env, ServerContext, UserContext } from '#/context.ts';

export const graphql = new Hono<Env>();

const app = createYoga<{ c: ServerContext }, UserContext>({
  schema,
  context: ({ c }) => ({ c, ...c.get('context') }),
  graphqlEndpoint: '/graphql',
  batching: true,
  cors: false,
  maskedErrors: false,
  landingPage: false,
  plugins: [
    useExecutionCancellation(),
    useLogger(),
    useError(),
    useRateLimit({
      default: { max: 300, refillRate: 5 },
    }),
  ],
});

type Extra = { c: ServerContext; bootstrapBypassKeyHash?: string };

const server = makeServer<{ session: string }, Extra>({
  /* eslint-disable @typescript-eslint/no-explicit-any */
  execute: (args: any) => args.rootValue.execute(args),
  subscribe: (args: any) => args.rootValue.subscribe(args),
  /* eslint-enable @typescript-eslint/no-explicit-any */
  onConnect: async (ctx) => {
    if (!ctx.connectionParams?.session) {
      return false;
    }

    const session = await redis.getdel(`user:ws:${ctx.connectionParams.session}`);
    if (!session) {
      return false;
    }

    const { userId, bootstrapBypassKeyHash } = JSON.parse(session);
    if (!userId) {
      return false;
    }

    ctx.extra.c.var.context.session = {
      id: ctx.connectionParams.session,
      userId,
    };

    ctx.extra.bootstrapBypassKeyHash = bootstrapBypassKeyHash;
  },
  onSubscribe: async (ctx, _, payload) => {
    try {
      const { schema, parse, validate, execute, subscribe, contextFactory } = app.getEnveloped({
        ...ctx.extra,
      });

      const document = parse(payload.query);
      const errors = validate(schema, document);

      if (errors.length > 0) {
        return errors;
      }

      return {
        schema,
        operationName: payload.operationName,
        document,
        variableValues: payload.variables,
        contextValue: await contextFactory(),
        rootValue: {
          execute,
          subscribe,
        },
      };
    } catch (err) {
      if (err instanceof GraphQLError) {
        return [err];
      }

      return [new GraphQLError(String(err))];
    }
  },
});

graphql.get(
  '/',
  upgradeWebSocket((c) => {
    const protocol = c.req.header('sec-websocket-protocol') ?? '';
    const ip = getClientAddress(c);
    const extra: Extra = { c };

    let handleMessage: ((data: string) => Promise<void>) | undefined;
    let handleClose: ((code?: number, reason?: string) => Promise<void>) | undefined;
    let bootstrapInterval: ReturnType<typeof setInterval> | undefined;

    return {
      onOpen: (_, ws) => {
        handleClose = server.opened(
          {
            protocol,
            send: (data) => ws.send(data, { compress: true }),
            close: (code, reason) => ws.close(code, reason),
            onMessage: (cb) => {
              handleMessage = cb;
            },
          },
          extra,
        );

        bootstrapInterval = setInterval(async () => {
          const { maintenance } = await checkBootstrap(ip, extra.bootstrapBypassKeyHash);
          if (maintenance) {
            ws.close(1001, 'Service under maintenance');
          }
        }, 60_000);
      },
      onClose: async (event) => {
        clearInterval(bootstrapInterval);
        await handleClose?.(event.code, event.reason);
      },
      onMessage: async (event, ws) => {
        if (typeof event.data === 'string') {
          try {
            await handleMessage?.(event.data);
          } catch {
            ws.close(CloseCode.InternalServerError, 'Internal server error');
          }
        }
      },
    };
  }),
);

graphql.on(['GET', 'POST'], '/', async (c) => {
  const response = await app.handle(c.req.raw, { c });
  return c.newResponse(response.body, response);
});
