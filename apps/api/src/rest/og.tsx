import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import { renderAsync } from '@resvg/resvg-js';
import { EntityState, EntityType } from '@typie/lib/enums';
import { and, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { HTTPException } from 'hono/http-exception';
import ky from 'ky';
import satori from 'satori';
import sharp from 'sharp';
import { match } from 'ts-pattern';
import twemoji from 'twemoji';
import { db, Documents, Entities, first, Folders, Images, Posts } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import type { Env } from '#/context.ts';

export const og = new Hono<Env>();

const loadFonts = async (filenames: string[]) => {
  const load = async (filename: string) => {
    const ext = path.extname(filename).slice(1);
    const filePath = path.join('/tmp/fonts', filename);

    try {
      return await readFile(filePath);
    } catch {
      const url = `https://cdn.typie.net/fonts/${ext}/${filename}`;
      const resp = await ky.get(url).arrayBuffer();

      await mkdir(path.dirname(filePath), { recursive: true });
      await writeFile(filePath, new Uint8Array(resp));

      return resp;
    }
  };

  return Object.fromEntries(await Promise.all(filenames.map(async (filename) => [filename, await load(filename)]))) as Record<
    string,
    ArrayBuffer
  >;
};

const fonts = await loadFonts([
  'KoPubWorldDotum-Medium.otf',
  'KoPubWorldDotum-Bold.otf',
  'Pretendard-Medium.otf',
  'Pretendard-ExtraBold.otf',
  'SUIT-Medium.otf',
  'SUIT-ExtraBold.otf',
  'NotoSansKR-Medium.ttf',
  'NotoSansKR-ExtraBold.ttf',
  'Paperlogy-4Regular.ttf',
  'Paperlogy-7Bold.ttf',
  'DeepMindSans-Regular.ttf',
]);

const colors = {
  white: '#FFFFFF',

  gray: {
    50: '#f9fafd',
    100: '#f3f4f9',
    200: '#e3e4eb',
    300: '#d3d4dd',
    400: '#9e9fa9',
    500: '#70717b',
    600: '#51525b',
    700: '#3e3f47',
    800: '#26272c',
    900: '#17181c',
    950: '#09090c',
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
    .with(EntityType.DOCUMENT, () => renderDocument(entityId))
    .exhaustive();

  const svg = await satori(node, {
    width: 1200,
    height: 630,
    fonts: [
      { name: 'KoPubWorldDotum', data: fonts['KoPubWorldDotum-Medium.otf'], weight: 500 },
      { name: 'KoPubWorldDotum', data: fonts['KoPubWorldDotum-Bold.otf'], weight: 800 },
      { name: 'Pretendard', data: fonts['Pretendard-Medium.otf'], weight: 500 },
      { name: 'Pretendard', data: fonts['Pretendard-ExtraBold.otf'], weight: 800 },
      { name: 'SUIT', data: fonts['SUIT-Medium.otf'], weight: 500 },
      { name: 'SUIT', data: fonts['SUIT-ExtraBold.otf'], weight: 800 },
      { name: 'NotoSansKR', data: fonts['NotoSansKR-Medium.ttf'], weight: 500 },
      { name: 'NotoSansKR', data: fonts['NotoSansKR-ExtraBold.ttf'], weight: 800 },
      { name: 'Paperlogy', data: fonts['Paperlogy-4Regular.ttf'], weight: 400 },
      { name: 'Paperlogy', data: fonts['Paperlogy-7Bold.ttf'], weight: 700 },
      { name: 'DeepMindSans', data: fonts['DeepMindSans-Regular.ttf'], weight: 400 },
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

  return c.body(Uint8Array.from(img.asPng()), {
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
        fontFamily: 'SUIT, Pretendard, NotoSansKR, KoPubWorldDotum',
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
      thumbnailPath: Images.path,
    })
    .from(Entities)
    .innerJoin(Folders, eq(Folders.entityId, Entities.id))
    .leftJoin(Images, eq(Images.id, Folders.thumbnailId))
    .where(eq(Entities.id, entityId))
    .then(first);

  if (!folder) {
    throw new HTTPException(404);
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '1200px',
        height: '630px',
        fontFamily: 'Paperlogy',
        backgroundColor: colors.white,
      }}
    >
      <div style={{ width: '1200px', height: '10px', backgroundColor: '#6c6fc8' }} />

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          flex: 1,
          padding: '80px 100px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: '60px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '24px', flex: 1 }}>
            <div
              style={{
                display: 'block',
                fontSize: '56px',
                fontWeight: 700,
                color: colors.gray[950],
                lineHeight: '1.3',
                lineClamp: 2,
                wordBreak: 'break-all',
              }}
            >
              {folder.name}
            </div>
          </div>

          {folder.thumbnailPath && (
            <img
              src={await toDataUri(folder.thumbnailPath)}
              width={200}
              height={200}
              style={{ objectFit: 'cover', borderRadius: '12px' }}
            />
          )}
        </div>

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            fontSize: '28px',
            fontWeight: 400,
            color: colors.gray[400],
          }}
        >
          <span>TYPIE &mdash; 작가를 위한 글쓰기 도구</span>
          <span style={{ fontFamily: 'DeepMindSans' }}>typie.co</span>
        </div>
      </div>
    </div>
  );
};

const renderDocument = async (entityId: string) => {
  const document = await db
    .select({
      title: Documents.title,
      subtitle: Documents.subtitle,
      thumbnailPath: Images.path,
    })
    .from(Entities)
    .innerJoin(Documents, eq(Documents.entityId, Entities.id))
    .leftJoin(Images, eq(Images.id, Documents.thumbnailId))
    .where(eq(Entities.id, entityId))
    .then(first);

  if (!document) {
    throw new HTTPException(404);
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '1200px',
        height: '630px',
        fontFamily: 'Paperlogy',
        backgroundColor: colors.white,
      }}
    >
      <div style={{ width: '1200px', height: '10px', backgroundColor: '#6c6fc8' }} />

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          flex: 1,
          padding: '80px 100px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: '60px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '24px', flex: 1 }}>
            <div
              style={{
                display: 'block',
                fontSize: '56px',
                fontWeight: 700,
                color: colors.gray[950],
                lineHeight: '1.3',
                lineClamp: 2,
                wordBreak: 'break-all',
              }}
            >
              {document.title ?? '(제목 없음)'}
            </div>

            {document.subtitle && (
              <div
                style={{
                  display: 'block',
                  fontSize: '30px',
                  fontWeight: 400,
                  color: colors.gray[500],
                  lineClamp: 1,
                }}
              >
                {document.subtitle}
              </div>
            )}
          </div>

          {document.thumbnailPath && (
            <img
              src={await toDataUri(document.thumbnailPath)}
              width={200}
              height={200}
              style={{ objectFit: 'cover', borderRadius: '12px' }}
            />
          )}
        </div>

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            fontSize: '28px',
            fontWeight: 400,
            color: colors.gray[400],
          }}
        >
          <span>TYPIE &mdash; 작가를 위한 글쓰기 도구</span>
          <span style={{ fontFamily: 'DeepMindSans' }}>typie.co</span>
        </div>
      </div>
    </div>
  );
};

const toDataUri = async (path: string) => {
  const object = await aws.s3.send(new GetObjectCommand({ Bucket: 'typie-usercontents', Key: `images/${path}` }));
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const source = await object.Body!.transformToByteArray();
  const buffer = await sharp(source, { failOn: 'none' }).png().toBuffer();

  return `data:image/png;base64,${Uint8Array.from(buffer).toBase64()}`;
};
