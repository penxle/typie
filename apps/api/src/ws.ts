import { createNodeWebSocket } from '@hono/node-ws';
import { app } from './app';

export const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });
