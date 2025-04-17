import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import dayjs from 'dayjs';
import { eq, inArray } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, firstOrThrow } from '@/db';
import { PostContents, PostOptions, UserPersonalIdentities } from '@/db/schemas/tables';
import { PostAgeRating, PostViewHiddenReason } from '@/enums';
import { schema, textSerializers } from '@/pm';
import { checkEntityPermission } from './entity';
import type { JSONContent } from '@tiptap/core';
import type { Context } from '@/context';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  maxWidth?: number;
  body: JSONContent;
};
export const makeYDoc = ({ title, subtitle, maxWidth, body }: MakeYDocParams) => {
  const node = Node.fromJSON(schema, body);
  const doc = new Y.Doc();

  const map = doc.getMap('attrs');
  map.set('title', title ?? '');
  map.set('subtitle', subtitle ?? '');
  map.set('maxWidth', maxWidth ?? 800);

  const fragment = doc.getXmlFragment('body');
  prosemirrorToYXmlFragment(node, fragment);

  return doc;
};

export const makeText = (body: JSONContent) => {
  const node = Node.fromJSON(schema, body);

  return getText(node, {
    blockSeparator: '\n',
    textSerializers,
  }).trim();
};

export const getPostDocument = async (postId: string) => {
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

type CheckPostHiddenReasonParams = {
  postId: string;
  userId: string | undefined;
  deviceId: string;
  entityId: string;
  ctx?: Context;
};
export const checkPostHiddenReason = async ({ postId, userId, deviceId, entityId, ctx }: CheckPostHiddenReasonParams) => {
  const postOptionLoader = ctx?.loader({
    name: 'PostOptions(postId)',
    load: async (ids) => {
      return await db.select().from(PostOptions).where(inArray(PostOptions.postId, ids));
    },
    key: ({ postId }) => postId,
  });

  const option = await (postOptionLoader
    ? postOptionLoader.load(postId)
    : db.select().from(PostOptions).where(eq(PostOptions.postId, postId)).then(firstOrThrow));

  if (option.ageRating !== PostAgeRating.ALL) {
    if (!userId) {
      return PostViewHiddenReason.INVALID_IDENTITY;
    }

    const userPersonalIdentityLoader = ctx?.loader({
      name: 'UserPersonalIdentities(userId)',
      load: async (ids) => {
        return await db.select().from(UserPersonalIdentities).where(inArray(UserPersonalIdentities.userId, ids));
      },
      key: ({ userId }) => userId,
    });

    const userPersonalIdentity = await (userPersonalIdentityLoader
      ? userPersonalIdentityLoader.load(userId)
      : db.select().from(UserPersonalIdentities).where(eq(UserPersonalIdentities.userId, userId)).then(firstOrThrow));

    if (userPersonalIdentity.expiresAt.isBefore(dayjs())) {
      return PostViewHiddenReason.INVALID_IDENTITY;
    }

    const availableBirthdayAfter = match(option.ageRating)
      .with(PostAgeRating.R15, () => {
        return dayjs().subtract(15, 'year').endOf('year');
      })
      .with(PostAgeRating.R19, () => {
        return dayjs().subtract(19, 'year').endOf('year');
      })
      .exhaustive();

    if (userPersonalIdentity.birthday.isAfter(availableBirthdayAfter)) {
      return PostViewHiddenReason.AGE_RATING;
    }
  }

  if (option.password && !(await checkEntityPermission({ entityId, userId, ctx }))) {
    const passwordUnlock = await redis.get(`post:password-unlock:${postId}:${deviceId}`);

    if (!passwordUnlock) {
      return PostViewHiddenReason.PASSWORD;
    }
  }

  return null;
};
