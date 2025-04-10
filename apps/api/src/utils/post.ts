import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { eq } from 'drizzle-orm';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, firstOrThrow } from '@/db';
import { PostContents } from '@/db/schemas/tables';
import { schema, textSerializers } from '@/pm';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  body: JSONContent;
};
export const makeYDoc = ({ title, subtitle, body }: MakeYDocParams) => {
  const node = Node.fromJSON(schema, body);
  const doc = new Y.Doc();

  const map = doc.getMap('attrs');
  map.set('title', title ?? '');
  map.set('subtitle', subtitle ?? '');
  map.set('maxWidth', 800);

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
