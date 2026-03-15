import { createNodeWebSocket } from '@hono/node-ws';
import { app } from '#/app.ts';

export const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });
