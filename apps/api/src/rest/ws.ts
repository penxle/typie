import { eq } from 'drizzle-orm';
import { Hono } from 'hono';
import Redis from 'ioredis';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { PostDocumentSyncMessageKind, WsMessageKind } from '@/const';
import { db, Entities, firstOrThrow, PostContents, Posts } from '@/db';
import { env } from '@/env';
import { enqueueJob } from '@/mq';
import { decode, encode } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { upgradeWebSocket } from '@/ws';
import type { ServerWebSocket } from 'bun';
import type { WSContext } from 'hono/ws';
import type { Env } from '@/context';

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
  upgradeWebSocket(() => {
    let timer: NodeJS.Timeout | null = null;

    let currentPostId: string | null = null;
    let currentUserId: string | null = null;

    return {
      onMessage: async (event, ws) => {
        if (!(event.data instanceof ArrayBuffer)) {
          return;
        }

        const data = new Uint8Array(event.data);
        const type = data[0];
        const payload = data.slice(1);

        if (type === WsMessageKind.ESTABLISH) {
          const token = decode(payload);
          const value = await redis.getdel(`user:ws:${token}`);
          if (!value) {
            ws.close(3401, 'Unauthorized');
            return;
          }

          const { userId } = JSON.parse(value);
          if (!userId) {
            ws.close(3401, 'Unauthorized');
            return;
          }

          currentUserId = userId;
          send(ws, WsMessageKind.ESTABLISH, new Uint8Array(encode(userId)));

          send(ws, WsMessageKind.HEARTBEAT, new Uint8Array());
          timer = setInterval(() => {
            send(ws, WsMessageKind.HEARTBEAT, new Uint8Array());
          }, 1000);
        }

        if (!currentUserId) {
          return;
        }

        if (type === PostDocumentSyncMessageKind.INIT) {
          const postId = decode(payload);

          try {
            const entity = await db
              .select({ siteId: Entities.siteId })
              .from(Entities)
              .innerJoin(Posts, eq(Entities.id, Posts.entityId))
              .where(eq(Posts.id, postId))
              .then(firstOrThrow);

            await assertSitePermission({
              userId: currentUserId,
              siteId: entity.siteId,
            });

            currentPostId = postId;
          } catch {
            return;
          }
          send(ws, PostDocumentSyncMessageKind.INIT, new Uint8Array());

          if (!clients.has(currentPostId)) {
            clients.set(currentPostId, new Set());
          }

          clients.get(currentPostId)?.add(ws);

          clients.get(currentPostId)?.forEach((client) => {
            send(client, PostDocumentSyncMessageKind.PRESENCE, new Uint8Array());
          });
        }

        if (!currentPostId) {
          return;
        }

        if (type === PostDocumentSyncMessageKind.UPDATE) {
          await publisher.publish('post:document:sync', Buffer.from([...encode(currentPostId), 0, ...data]));
          await redis.sadd(`post:document:updates:${currentPostId}`, Buffer.from([...encode(currentUserId), 0, ...payload]));
          await enqueueJob('post:document:update', currentPostId);
        } else if (type === PostDocumentSyncMessageKind.VECTOR) {
          const state = await getPostDocument(currentPostId);
          const update = Y.diffUpdateV2(state.update, payload);
          send(ws, PostDocumentSyncMessageKind.UPDATE, update);
          send(ws, PostDocumentSyncMessageKind.VECTOR, state.vector);
        } else if (type === PostDocumentSyncMessageKind.AWARENESS) {
          await publisher.publish('post:document:sync', Buffer.from([...encode(currentPostId), 0, ...data]));
        }
      },
      onClose: (_, ws) => {
        if (currentPostId) {
          clients.get(currentPostId)?.delete(ws);
          if (clients.get(currentPostId)?.size === 0) {
            clients.delete(currentPostId);
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
