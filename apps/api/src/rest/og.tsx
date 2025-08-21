import fs from 'node:fs/promises';
import path from 'node:path';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import { renderAsync } from '@resvg/resvg-js';
import { and, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { HTTPException } from 'hono/http-exception';
import ky from 'ky';
import satori from 'satori';
import sharp from 'sharp';
import { match } from 'ts-pattern';
import twemoji from 'twemoji';
import { Canvases, db, Entities, first, Folders, Images, Posts } from '@/db';
import { EntityState, EntityType } from '@/enums';
import * as aws from '@/external/aws';
import { Lazy } from '@/utils';
import type { Env } from '@/context';

export const og = new Hono<Env>();

const loadFonts = async <T extends string>(names: T[]) => {
  const load = async (name: string) => {
    const filePath = path.join('/tmp/fonts', `${name}.otf`);

    try {
      return await fs.readFile(filePath);
    } catch {
      const url = `https://cdn.typie.net/fonts/otf/${name}.otf`;
      const resp = await ky.get(url).arrayBuffer();

      await fs.mkdir(path.dirname(filePath), { recursive: true });
      await fs.writeFile(filePath, Buffer.from(resp));

      return resp;
    }
  };

  return Object.fromEntries(await Promise.all(names.map(async (name) => [name, await load(name)]))) as Record<T, ArrayBuffer>;
};

const lazyFonts = new Lazy(() =>
  loadFonts([
    'KoPubWorldDotum-Medium',
    'KoPubWorldDotum-Bold',
    'Pretendard-Medium',
    'Pretendard-ExtraBold',
    'SUIT-Medium',
    'SUIT-ExtraBold',
  ]),
);

const colors = {
  white: '#FFFFFF',

  gray: {
    50: '#FAFAFA',
    100: '#F4F4F5',
    200: '#E4E4E7',
    300: '#D4D4D8',
    400: '#A1A1AA',
    500: '#71717A',
    600: '#52525B',
    700: '#3F3F46',
    800: '#27272A',
    900: '#18181B',
    950: '#09090B',
  },
};

og.get('/:entityId', async (c) => {
  const entityId = c.req.param('entityId');

  const entity = await db
    .select({ type: Entities.type })
    .from(Entities)
    .where(and(eq(Entities.id, entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);

  if (!entity) {
    throw new HTTPException(404);
  }

  const node = await match(entity.type)
    .with(EntityType.POST, () => renderPost(entityId))
    .with(EntityType.FOLDER, () => renderFolder(entityId))
    .with(EntityType.CANVAS, () => renderCanvas(entityId))
    .exhaustive();

  const fonts = await lazyFonts.get();

  const svg = await satori(node, {
    width: 1200,
    height: 630,
    fonts: [
      { name: 'KoPubWorldDotum', data: fonts['KoPubWorldDotum-Medium'], weight: 500 },
      { name: 'KoPubWorldDotum', data: fonts['KoPubWorldDotum-Bold'], weight: 800 },
      { name: 'Pretendard', data: fonts['Pretendard-Medium'], weight: 500 },
      { name: 'Pretendard', data: fonts['Pretendard-ExtraBold'], weight: 800 },
      { name: 'SUIT', data: fonts['SUIT-Medium'], weight: 500 },
      { name: 'SUIT', data: fonts['SUIT-ExtraBold'], weight: 800 },
    ],
    loadAdditionalAsset: async (code, segment) => {
      const svg = await match(code)
        .with('emoji', () => {
          const codepoint = twemoji.convert.toCodePoint(segment);
          return ky(`https://cdnjs.cloudflare.com/ajax/libs/twemoji/14.0.2/svg/${codepoint}.svg`).text();
        })
        .otherwise(() => '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" />');

      return 'data:image/svg+xml,' + encodeURIComponent(svg);
    },
  });

  const img = await renderAsync(svg, {
    font: { loadSystemFonts: false },
    imageRendering: 0, // optimizeQuality
    shapeRendering: 2, // geometricPrecision
    textRendering: 1, // optimizeLegibility
  });

  return c.body(img.asPng(), {
    headers: {
      'Content-Type': 'image/png',
    },
  });
});

const renderPost = async (entityId: string) => {
  const post = await db
    .select({
      title: Posts.title,
      subtitle: Posts.subtitle,
      coverImagePath: Images.path,
    })
    .from(Entities)
    .innerJoin(Posts, eq(Posts.entityId, Entities.id))
    .leftJoin(Images, eq(Images.id, Posts.coverImageId))
    .where(eq(Entities.id, entityId))
    .then(first);

  if (!post) {
    throw new HTTPException(404);
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '60px',
        width: '1200px',
        height: '630px',
        fontFamily: 'SUIT, Pretendard, KoPubWorldDotum',
        color: colors.gray[950],
        lineHeight: '1.4',
        backgroundColor: colors.white,
        wordBreak: 'break-all',
      }}
    >
      {post.coverImagePath ? (
        <img src={await toDataUri(post.coverImagePath ?? '')} width={1200} height={240} style={{ objectFit: 'cover' }} />
      ) : (
        <div style={{ width: '1200px', height: '240px', backgroundColor: colors.gray[100] }} />
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '32px', width: '1000px', margin: '0 auto' }}>
        <div style={{ display: 'block', fontSize: '60px', fontWeight: 800, lineClamp: 2 }}>{post.title ?? '(제목 없음)'}</div>
        <div style={{ display: 'block', fontSize: '40px', fontWeight: 500, lineClamp: 1, color: colors.gray[500] }}>{post.subtitle}</div>
      </div>
    </div>
  );
};

const renderFolder = async (entityId: string) => {
  const folder = await db
    .select({
      name: Folders.name,
    })
    .from(Entities)
    .innerJoin(Folders, eq(Folders.entityId, Entities.id))
    .where(eq(Entities.id, entityId))
    .then(first);

  if (!folder) {
    throw new HTTPException(404);
  }

  return <div>{folder.name}</div>;
};

const renderCanvas = async (entityId: string) => {
  const canvas = await db
    .select({
      title: Canvases.title,
    })
    .from(Entities)
    .innerJoin(Canvases, eq(Canvases.entityId, Entities.id))
    .where(eq(Entities.id, entityId))
    .then(first);

  if (!canvas) {
    throw new HTTPException(404);
  }

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '1200px',
        height: '630px',
        fontFamily: 'SUIT, Pretendard, KoPubWorldDotum',
        fontSize: '60px',
        fontWeight: 800,
        color: colors.gray[950],
        backgroundColor: colors.gray[100],
      }}
    >
      {canvas.title}
    </div>
  );
};

const toDataUri = async (path: string) => {
  const object = await aws.s3.send(new GetObjectCommand({ Bucket: 'typie-usercontents', Key: `images/${path}` }));
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const source = await object.Body!.transformToByteArray();
  const buffer = await sharp(source, { failOn: 'none' }).png().toBuffer();

  return `data:image/png;base64,${buffer.toString('base64')}`;
};
