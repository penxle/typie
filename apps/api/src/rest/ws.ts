import { Hono } from 'hono';
import Redis from 'ioredis';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { PostContentSyncMessageKind } from '@/const';
import { env } from '@/env';
import { enqueueJob } from '@/mq';
import { decode, encode, getCurrentPostContentState } from '@/utils';
import { upgradeWebSocket } from '@/ws';
import type { ServerWebSocket } from 'bun';
import type { WSContext } from 'hono/ws';
import type { Context, Env } from '@/context';

export const ws = new Hono<Env>();

const publisher = new Redis(env.REDIS_URL);
const subscriber = new Redis(env.REDIS_URL);

const clients = new Map<string, Set<WSContext<ServerWebSocket>>>();

const send = (ws: WSContext<ServerWebSocket>, type: number, payload: Uint8Array) => {
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(new Uint8Array([type, ...payload]));
  }
};

subscriber.on('messageBuffer', (_, message) => {
  const data = new Uint8Array(message);
  const sepIdx = data.indexOf(0);

  const postId = decode(data.slice(0, sepIdx));
  const type = data[sepIdx + 1];
  const payload = data.slice(sepIdx + 2);

  clients.get(postId)?.forEach((client) => {
    send(client, type, payload);
  });
});

await subscriber.subscribe('post:content:sync');

ws.get(
  '/',
  upgradeWebSocket((c) => {
    const context: Context = c.var.context;
    if (!context.session) {
      return {
        onOpen: (_, ws) => {
          ws.close(3401, 'Unauthorized');
        },
      };
    }

    let postId: string | null = null;

    return {
      onMessage: async (event, ws) => {
        if (!(event.data instanceof ArrayBuffer)) {
          return;
        }

        const data = new Uint8Array(event.data);
        const type = data[0];
        const payload = data.slice(1);

        if (type === PostContentSyncMessageKind.INIT) {
          postId = decode(payload);
          send(ws, PostContentSyncMessageKind.INIT, new Uint8Array());

          if (!clients.has(postId)) {
            clients.set(postId, new Set());
          }

          clients.get(postId)?.add(ws);

          clients.get(postId)?.forEach((client) => {
            send(client, PostContentSyncMessageKind.PRESENCE, new Uint8Array());
          });
        }

        if (!postId) {
          return;
        }

        if (type === PostContentSyncMessageKind.UPDATE) {
          await publisher.publish('post:content:sync', Buffer.from([...encode(postId), 0, ...data]));
          await redis.sadd(`post:content:updates:${postId}`, Buffer.from(payload));
          await enqueueJob('post:content:update', postId);
        } else if (type === PostContentSyncMessageKind.VECTOR) {
          const state = await getCurrentPostContentState(postId);
          const update = Y.diffUpdateV2(state.update, payload);
          send(ws, PostContentSyncMessageKind.UPDATE, update);
          send(ws, PostContentSyncMessageKind.VECTOR, state.vector);
        } else if (type === PostContentSyncMessageKind.AWARENESS) {
          await publisher.publish('post:content:sync', Buffer.from([...encode(postId), 0, ...data]));
        }
      },
      onClose: (_, ws) => {
        if (postId) {
          clients.get(postId)?.delete(ws);
          if (clients.get(postId)?.size === 0) {
            clients.delete(postId);
          }
        }
      },
    };
  }),
);
