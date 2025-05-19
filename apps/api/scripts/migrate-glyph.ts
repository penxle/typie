#!/usr/bin/env bun

import { stdin, stdout } from 'node:process';
import * as readline from 'node:readline/promises';
import { GetObjectCommand, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { TinyColor } from '@ctrl/tinycolor';
import { findChildren } from '@tiptap/core';
import { and, desc, eq, isNull } from 'drizzle-orm';
import postgres from 'postgres';
import qs from 'query-string';
import { match } from 'ts-pattern';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { db, Embeds, Entities, Files, first, firstOrThrow, Folders, Images, PostContents, Posts, PostSnapshots, Sites, Users } from '@/db';
import { EntityType } from '@/enums';
import { env } from '@/env';
import * as aws from '@/external/aws';
import * as iframely from '@/external/iframely';
import { schema } from '@/pm';
import { generateEntityOrder, generatePermalink, generateSlug, makeText, makeYDoc } from '@/utils';
import type { JSONContent } from '@tiptap/core';
import type { Node } from '@tiptap/pm/model';
import type { Transaction } from '@/db';

const rawColors = [
  { label: '블랙', value: 'black', hex: '#09090b' },
  { label: '그레이', value: 'gray', hex: '#71717a' },
  { label: '화이트', value: 'white', hex: '#ffffff' },
  { label: '레드', value: 'red', hex: '#ef4444' },
  { label: '오렌지', value: 'orange', hex: '#f97316' },
  { label: '앰버', value: 'amber', hex: '#f59e0b' },
  { label: '옐로', value: 'yellow', hex: '#eab308' },
  { label: '라임', value: 'lime', hex: '#84cc16' },
  { label: '그린', value: 'green', hex: '#22c55e' },
  { label: '에메랄드', value: 'emerald', hex: '#10b981' },
  { label: '틸', value: 'teal', hex: '#14b8a6' },
  { label: '시안', value: 'cyan', hex: '#06b6d4' },
  { label: '스카이', value: 'sky', hex: '#0ea5e9' },
  { label: '블루', value: 'blue', hex: '#3b82f6' },
  { label: '인디고', value: 'indigo', hex: '#6366f1' },
  { label: '바이올렛', value: 'violet', hex: '#8b5cf6' },
  { label: '퍼플', value: 'purple', hex: '#a855f7' },
  { label: '마젠타', value: 'fuchsia', hex: '#d946ef' },
  { label: '핑크', value: 'pink', hex: '#ec4899' },
  { label: '로즈', value: 'rose', hex: '#f43f5e' },
];
const hexes = Object.fromEntries(rawColors.map(({ value, hex }) => [value, hex]));
const textColors = rawColors.map(({ value }) => value);

const normalize = (color: TinyColor) => {
  const input = color.toRgb();

  return textColors.reduce(
    (closest, value) => {
      const target = new TinyColor(hexes[value]).toRgb();
      const d = Math.hypot(input.r - target.r, input.g - target.g, input.b - target.b);
      return d < closest.d ? { value, d } : closest;
    },
    { value: textColors[0], d: Number.MAX_VALUE },
  ).value;
};

const getCharacterCount = (text: string) => {
  return [...text.replaceAll(/\s+/g, ' ').trim()].length;
};

const getBlobSize = (node: Node) => {
  const sizes = findChildren(node, (node) => node.type.name === 'file' || node.type.name === 'image').map(
    ({ node }) => Number(node.attrs.size) || 0,
  );
  return sizes.reduce((acc, size) => acc + size, 0);
};

/* eslint-disable @typescript-eslint/no-non-null-assertion */
const sqlGlyph = postgres(process.env.GLYPH_DATABASE_URL!);

const s3Glyph = new S3Client({
  credentials: {
    accessKeyId: process.env.GLYPH_AWS_ACCESS_KEY_ID!,
    secretAccessKey: process.env.GLYPH_AWS_SECRET_ACCESS_KEY!,
  },
  region: 'ap-northeast-2',
});

/* eslint-enable @typescript-eslint/no-non-null-assertion */

const rl = readline.createInterface(stdin, stdout);

type MigrateNodeParams = {
  node: JSONContent;
  userId: string;
  tx: Transaction;
};
const migrateNode = async ({ node, userId, tx }: MigrateNodeParams): Promise<JSONContent> => {
  // Migrate Marks

  const textStyles: { textColor?: string; fontFamily?: string; fontSize?: string } = {};
  let hasTextStyle = false;
  const migratedMarks =
    node.marks
      ?.map((mark) => {
        if (mark.type === 'font_color' && mark.attrs?.fontColor) {
          textStyles.textColor = normalize(new TinyColor(mark.attrs.fontColor));
          hasTextStyle = true;
          return null;
        }
        if (mark.type === 'font_family' && mark.attrs?.fontFamily) {
          textStyles.fontFamily = mark.attrs.fontFamily;
          hasTextStyle = true;
          return null;
        }
        if (mark.type === 'font_size' && mark.attrs?.fontSize) {
          textStyles.fontSize = mark.attrs.fontColor;
          hasTextStyle = true;
          return null;
        }
        return mark;
      })
      .filter((mark) => mark !== null) ?? [];

  if (hasTextStyle) {
    migratedMarks.push({
      type: 'text_style',
      attrs: textStyles,
    });
  }

  // Migrate Node
  const newNode = await match(node.type)
    .with('document', () => {
      return {
        type: 'body',
        attrs: {
          paragraphIndent: node.attrs?.documentParagraphIndent,
          blockGap: node.attrs?.documentParagraphSpacing,
        },
      };
    })
    .with('blockquote', () => {
      if (node.attrs?.kind === 2 || node.attrs?.kind === 3) {
        return {
          attrs: { type: 'left-quote' },
        };
      } else {
        return {
          attrs: { type: 'left-line' },
        };
      }
    })
    .with('horizontal_rule', () => {
      if (node.attrs?.kind === 1) {
        return {
          attrs: { type: 'dashed-line' },
        };
      } else if (node.attrs?.kind === 2 || node.attrs?.kind === 3) {
        return {
          attrs: { type: 'light-line' },
        };
      } else if (node.attrs?.kind === 8) {
        return {
          attrs: { type: 'zigzag' },
        };
      } else {
        return {
          attrs: { type: 'circle' },
        };
      }
    })
    .with('access_barrier', () => {
      return [];
    })
    .with('html', () => {
      return { type: 'html_block' };
    })
    .with('embed', async () => {
      const url = node.attrs?.url;

      if (!url) {
        return [];
      }

      return await tx
        .select({
          id: Embeds.id,
          url: Embeds.url,
          title: Embeds.title,
          description: Embeds.description,
          thumbnailUrl: Embeds.thumbnailUrl,
          html: Embeds.html,
        })
        .from(Embeds)
        .where(eq(Embeds.url, url))
        .then(first)
        .then(async (embed) => {
          if (embed) {
            return embed;
          }

          const meta = await iframely.unfurl(url).catch(() => null);

          if (!meta) {
            return [];
          }

          return await tx
            .insert(Embeds)
            .values({
              userId,
              type: meta.type,
              url,
              title: meta.title,
              description: meta.description,
              thumbnailUrl: meta.thumbnailUrl,
              html: meta.html,
            })
            .returning({
              id: Embeds.id,
              url: Embeds.url,
              title: Embeds.title,
              description: Embeds.description,
              thumbnailUrl: Embeds.thumbnailUrl,
              html: Embeds.html,
            })
            .onConflictDoUpdate({
              target: [Embeds.url],
              set: {
                userId,
                type: meta.type,
                title: meta.title,
                description: meta.description,
                thumbnailUrl: meta.thumbnailUrl,
                html: meta.html,
              },
            })
            .then(firstOrThrow);
        })
        .then((result) => ({ attrs: result }));
    })
    .with('file', async () => {
      const originalFile = await sqlGlyph<
        { name: string; format: string; size: number; path: string }[]
      >`SELECT name, format, size, path FROM files WHERE id = ${node.attrs?.id}`.then(firstOrThrow);

      const { Body: fileBody } = await s3Glyph.send(
        new GetObjectCommand({
          Bucket: 'penxle-data',
          Key: originalFile.path,
        }),
      );

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const arrayBuffer = await fileBody!.transformToByteArray();

      await aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-usercontents',
          Key: originalFile.path,
          Body: Buffer.from(arrayBuffer),
          ContentType: originalFile.format,
          Tagging: qs.stringify({
            UserId: userId,
            Environment: env.PUBLIC_PULUMI_STACK ?? 'local',
          }),
        }),
      );

      const file = await tx
        .insert(Files)
        .values({
          userId,
          name: originalFile.name,
          format: originalFile.format,
          size: originalFile.size,
          path: originalFile.path,
        })
        .returning({
          id: Files.id,
          name: Files.name,
          size: Files.size,
          path: Files.path,
        })
        .then(firstOrThrow);

      return {
        attrs: {
          id: file.id,
          name: file.name,
          size: file.size,
          url: `https://typie.net/${file.path}`,
        },
      };
    })
    .with('image', async () => {
      const originalImage = await sqlGlyph<
        { name: string; format: string; width: number; height: number; size: number; path: string; placeholder: string }[]
      >`SELECT name, format, width, height, size, path, placeholder FROM images WHERE id = ${node.attrs?.id}`.then(firstOrThrow);

      const { Body: imageBody } = await s3Glyph.send(
        new GetObjectCommand({
          Bucket: 'penxle-data',
          Key: originalImage.path,
        }),
      );

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const arrayBuffer = await imageBody!.transformToByteArray();

      await aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-usercontents',
          Key: originalImage.path,
          Body: Buffer.from(arrayBuffer),
          ContentType: originalImage.format,
          Tagging: qs.stringify({
            UserId: userId,
            Environment: env.PUBLIC_PULUMI_STACK ?? 'local',
          }),
        }),
      );

      const image = await tx
        .insert(Images)
        .values({
          userId,
          name: originalImage.name,
          format: originalImage.format,
          width: originalImage.width,
          height: originalImage.height,
          size: originalImage.size,
          path: originalImage.path,
          placeholder: originalImage.placeholder,
        })
        .returning({
          id: Images.id,
          name: Images.name,
          width: Images.width,
          height: Images.height,
          size: Images.size,
          path: Images.path,
          placeholder: Images.placeholder,
        })
        .then(firstOrThrow);

      return {
        attrs: {
          id: image.id,
          url: `https://typie.net/${image.path}`,
          ratio: image.width / image.height,
          placeholder: image.placeholder,
          proportion: node.attrs?.size === 'full' ? 1 : Math.min(image.width / 800, 1),
          size: image.size,
        },
      };
    })
    .with('gallery', async () => {
      const ids: string[] | undefined = node.attrs?.ids;

      if (!ids || ids.length === 0) {
        return [];
      }

      const originalImages = await sqlGlyph<
        { id: string; name: string; format: string; width: number; height: number; size: number; path: string; placeholder: string }[]
      >`SELECT id, name, format, width, height, size, path, placeholder FROM images WHERE id IN ${sqlGlyph(ids)}`;

      return await Promise.all(
        ids.map(async (id) => {
          const originalImage = originalImages.find((image) => image.id === id);

          if (!originalImage) {
            return null;
          }

          const { Body: imageBody } = await s3Glyph.send(
            new GetObjectCommand({
              Bucket: 'penxle-data',
              Key: originalImage.path,
            }),
          );

          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          const arrayBuffer = await imageBody!.transformToByteArray();

          await aws.s3.send(
            new PutObjectCommand({
              Bucket: 'typie-usercontents',
              Key: originalImage.path,
              Body: Buffer.from(arrayBuffer),
              ContentType: originalImage.format,
              Tagging: qs.stringify({
                UserId: userId,
                Environment: env.PUBLIC_PULUMI_STACK ?? 'local',
              }),
            }),
          );

          const image = await tx
            .insert(Images)
            .values({
              userId,
              name: originalImage.name,
              format: originalImage.format,
              width: originalImage.width,
              height: originalImage.height,
              size: originalImage.size,
              path: originalImage.path,
              placeholder: originalImage.placeholder,
            })
            .returning({
              id: Images.id,
              name: Images.name,
              width: Images.width,
              height: Images.height,
              size: Images.size,
              path: Images.path,
              placeholder: Images.placeholder,
            })
            .then(firstOrThrow);

          return {
            type: 'image',
            attrs: {
              id: image.id,
              url: `https://typie.net/${image.path}`,
              ratio: image.width / image.height,
              placeholder: image.placeholder,
              proportion: 1,
              size: image.size,
            },
          };
        }),
      ).then((results) => results.filter((result) => result !== null));
    })
    .otherwise(() => ({}));

  if (Array.isArray(newNode)) {
    return newNode;
  }

  return {
    ...node,
    ...newNode,
    marks: migratedMarks.length > 0 ? migratedMarks : undefined,
    content: await Promise.all(node.content?.map((child) => migrateNode({ node: child, userId, tx })) ?? []).then((results) =>
      results.flat(),
    ),
  };
};

type MigratePostParams = {
  tx: Transaction;
  body: JSONContent;
  userId: string;
  siteId: string;
  parentId: string;
  lastOrder: string | null;
  title: string | null;
  subtitle: string | null;
};

const migratePost = async (params: MigratePostParams) => {
  const body = await migrateNode({ node: params.body, userId: params.userId, tx: params.tx });

  const text = makeText(body);
  const characterCount = getCharacterCount(text);

  const doc = makeYDoc({
    title: params.title,
    subtitle: params.subtitle,
    body,
  });

  const snapshot = Y.snapshot(doc);
  const blobSize = getBlobSize(yXmlFragmentToProseMirrorRootNode(doc.getXmlFragment('body'), schema));

  const entity = await params.tx
    .insert(Entities)
    .values({
      userId: params.userId,
      siteId: params.siteId,
      parentId: params.parentId,
      slug: generateSlug(),
      permalink: generatePermalink(),
      type: EntityType.POST,
      order: generateEntityOrder({ lower: params.lastOrder, upper: null }),
      depth: 1,
    })
    .returning({ id: Entities.id, order: Entities.order })
    .then(firstOrThrow);

  const post = await params.tx
    .insert(Posts)
    .values({
      entityId: entity.id,
      title: params.title,
      subtitle: params.subtitle,
    })
    .returning()
    .then(firstOrThrow);

  await params.tx.insert(PostContents).values({
    postId: post.id,
    body,
    text,
    update: Y.encodeStateAsUpdateV2(doc),
    vector: Y.encodeStateVector(doc),
    characterCount,
    blobSize,
  });

  await params.tx.insert(PostSnapshots).values({
    userId: params.userId,
    postId: post.id,
    snapshot: Y.encodeSnapshotV2(snapshot),
  });

  return entity;
};

while (true) {
  const glyphEmail = await rl.question('Glyph Email: ');

  if (!glyphEmail) {
    process.exit(0);
  }

  const typieEmail = (await rl.question(`Typie Email(${glyphEmail}): `)) || glyphEmail;

  await db.transaction(async (tx) => {
    const user = await tx.select().from(Users).where(eq(Users.email, typieEmail)).then(firstOrThrow);
    const site = await tx.select().from(Sites).where(eq(Sites.userId, user.id)).then(firstOrThrow);
    let lastFolderOrder = await tx
      .select({ order: Entities.order })
      .from(Entities)
      .where(and(eq(Entities.siteId, site.id), isNull(Entities.parentId)))
      .orderBy(desc(Entities.order))
      .limit(1)
      .then((rows) => rows[0]?.order ?? null);

    const glyphUser = await sqlGlyph<{ id: string }[]>`SELECT id FROM users WHERE email = ${glyphEmail}`.then(firstOrThrow);
    const glyphSpaces = await sqlGlyph<
      { id: string; name: string }[]
    >`SELECT spaces.id, spaces.name FROM spaces INNER JOIN space_members ON spaces.id = space_members.space_id WHERE space_members.user_id = ${glyphUser.id}`;
    for (const glyphSpace of glyphSpaces) {
      const parentEntity = await tx
        .insert(Entities)
        .values({
          userId: user.id,
          siteId: site.id,
          parentId: null,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.FOLDER,
          order: generateEntityOrder({ lower: lastFolderOrder, upper: null }),
          depth: 0,
        })
        .returning({ id: Entities.id, order: Entities.order })
        .then(firstOrThrow);

      await tx
        .insert(Folders)
        .values({
          entityId: parentEntity.id,
          name: `${glyphSpace.name} 이전 포스트`,
        })
        .returning()
        .then(firstOrThrow);

      lastFolderOrder = parentEntity.order;

      const glyphPosts = await sqlGlyph<
        {
          title: string | null;
          subtitle: string | null;
          postrevisionfreecontent: JSONContent[] | null;
          postrevisionpaidcontent: JSONContent[] | null;
          attributes: Record<string, unknown>;
        }[]
      >`SELECT post_revisions.title, post_revisions.subtitle, post_revision_free_contents.data as postRevisionFreeContent, post_revision_paid_contents.data as postRevisionPaidContent, post_revisions.attributes
  FROM posts 
  INNER JOIN post_revisions ON posts.published_revision_id = post_revisions.id 
  LEFT JOIN post_revision_contents as post_revision_free_contents ON post_revisions.free_content_id = post_revision_free_contents.id
  LEFT JOIN post_revision_contents as post_revision_paid_contents ON post_revisions.paid_content_id = post_revision_paid_contents.id
  WHERE posts.space_id = ${glyphSpace.id} AND posts.state = 'PUBLISHED'
  ORDER BY post_revisions.created_at`;

      let lastPostOrder: string | null = null;

      for (const glyphPost of glyphPosts) {
        const entity = await migratePost({
          tx,
          body: {
            type: 'doc',
            content: [
              {
                type: 'document',
                attrs: glyphPost.attributes,
                content: [...(glyphPost.postrevisionfreecontent ?? []), ...(glyphPost.postrevisionpaidcontent ?? [])],
              },
            ],
          },
          userId: user.id,
          siteId: site.id,
          parentId: parentEntity.id,
          lastOrder: lastPostOrder,
          title: glyphPost.title,
          subtitle: glyphPost.subtitle,
        });

        console.log(`Migrated post ${glyphPost.title} (${entity.id})`);

        lastPostOrder = entity.order;
      }
    }
  });
}
