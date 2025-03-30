import { createYoga, useExecutionCancellation } from 'graphql-yoga';
import { Hono } from 'hono';
import { createContext } from '@/context';
import { useError } from './plugins/error';
import { useLogger } from './plugins/logger';
import { schema } from './schema';

export const yoga = new Hono();

const app = createYoga({
  schema,
  context: createContext,
  graphqlEndpoint: '/graphql',
  batching: true,
  cors: false,
  maskedErrors: false,
  landingPage: false,
  plugins: [useExecutionCancellation(), useLogger(), useError()],
});

yoga.on(['GET', 'POST'], '/', async (c) => {
  const response = await app.handle(c.req.raw, { c });
  return c.newResponse(response.body, response);
});
