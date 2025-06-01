import { createNodeWebSocket } from '@hono/node-ws';
import { app } from '@/app';

export const { upgradeWebSocket, injectWebSocket } = createNodeWebSocket({ app });
