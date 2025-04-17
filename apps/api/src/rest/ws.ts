import { eq } from 'drizzle-orm';
import { Hono } from 'hono';
import Redis from 'ioredis';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { PostDocumentSyncMessageKind } from '@/const';
import { db, firstOrThrow, PostContents } from '@/db';
import { env } from '@/env';
import { enqueueJob } from '@/mq';
import { decode, encode } from '@/utils';
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

await subscriber.subscribe('post:document:sync');

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
    let timer: NodeJS.Timeout | null = null;
    const userId = context.session.userId;

    return {
      onMessage: async (event, ws) => {
        if (!(event.data instanceof ArrayBuffer)) {
          return;
        }

        const data = new Uint8Array(event.data);
        const type = data[0];
        const payload = data.slice(1);

        if (type === PostDocumentSyncMessageKind.INIT) {
          postId = decode(payload);
          send(ws, PostDocumentSyncMessageKind.INIT, new Uint8Array());

          if (!clients.has(postId)) {
            clients.set(postId, new Set());
          }

          clients.get(postId)?.add(ws);

          clients.get(postId)?.forEach((client) => {
            send(client, PostDocumentSyncMessageKind.PRESENCE, new Uint8Array());
          });

          send(ws, PostDocumentSyncMessageKind.HEARTBEAT, new Uint8Array());
          timer = setInterval(() => {
            send(ws, PostDocumentSyncMessageKind.HEARTBEAT, new Uint8Array());
          }, 1000);
        }

        if (!postId) {
          return;
        }

        if (type === PostDocumentSyncMessageKind.UPDATE) {
          await publisher.publish('post:document:sync', Buffer.from([...encode(postId), 0, ...data]));
          await redis.sadd(`post:document:updates:${postId}`, Buffer.from([...encode(userId), 0, ...payload]));
          await enqueueJob('post:document:update', postId);
        } else if (type === PostDocumentSyncMessageKind.VECTOR) {
          const state = await getPostDocument(postId);
          const update = Y.diffUpdateV2(state.update, payload);
          send(ws, PostDocumentSyncMessageKind.UPDATE, update);
          send(ws, PostDocumentSyncMessageKind.VECTOR, state.vector);
        } else if (type === PostDocumentSyncMessageKind.AWARENESS) {
          await publisher.publish('post:document:sync', Buffer.from([...encode(postId), 0, ...data]));
        }
      },
      onClose: (_, ws) => {
        if (postId) {
          clients.get(postId)?.delete(ws);
          if (clients.get(postId)?.size === 0) {
            clients.delete(postId);
          }
        }

        if (timer) {
          clearInterval(timer);
        }
      },
    };
  }),
);

const getPostDocument = async (postId: string) => {
  const { update, vector } = await db
    .select({ update: PostContents.update, vector: PostContents.vector })
    .from(PostContents)
    .where(eq(PostContents.postId, postId))
    .then(firstOrThrow);

  const buffers = await redis.smembersBuffer(`post:document:updates:${postId}`);
  if (buffers.length === 0) {
    return {
      update,
      vector,
    };
  }

  const pendingUpdates = buffers.map((buffer) => {
    const data = new Uint8Array(buffer);
    const sepIdx = data.indexOf(0);

    return data.slice(sepIdx + 1);
  });

  const updatedUpdate = Y.mergeUpdatesV2([update, ...pendingUpdates]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};
