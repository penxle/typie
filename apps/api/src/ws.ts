import { createBunWebSocket } from 'hono/bun';
import type { ServerWebSocket } from 'bun';

export const { websocket, upgradeWebSocket } = createBunWebSocket<ServerWebSocket>();
