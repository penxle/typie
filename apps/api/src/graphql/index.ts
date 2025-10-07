import { useOpenTelemetry } from '@envelop/opentelemetry';
import { trace } from '@opentelemetry/api';
import { GraphQLError } from 'graphql';
import { CloseCode, makeServer } from 'graphql-ws';
import { createYoga, useExecutionCancellation } from 'graphql-yoga';
import { Hono } from 'hono';
import { upgradeWebSocket } from 'hono/bun';
import { redis } from '@/cache';
import { useError } from './plugins/error';
import { useLogger } from './plugins/logger';
import { schema } from './schema';
import type { Env, ServerContext, UserContext } from '@/context';

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
    useOpenTelemetry(
      {
        document: false,
        resolvers: true,
      },
      trace.getTracerProvider(),
    ),
  ],
});

const server = makeServer<{ session: string }, { c: ServerContext }>({
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

    const { userId } = JSON.parse(session);
    if (!userId) {
      return false;
    }

    ctx.extra.c.var.context.session = {
      id: ctx.connectionParams.session,
      userId,
    };
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

    let handleMessage: ((data: string) => Promise<void>) | undefined;
    let handleClose: ((code?: number, reason?: string) => Promise<void>) | undefined;

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
          { c },
        );
      },
      onClose: async (event) => {
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
