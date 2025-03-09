import { getClientAddress } from '@glitter/lib';
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

yoga.get('/', (c) => app.handleRequest(c.req.raw, { ip: getClientAddress(c) }));
yoga.post('/', (c) => app.handleRequest(c.req.raw, { ip: getClientAddress(c) }));
