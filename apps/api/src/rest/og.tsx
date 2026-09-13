import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import { renderAsync } from '@resvg/resvg-js';
import { EntityState, EntityType } from '@typie/lib/enums';
import { titlePageColors } from '@typie/lib/title-page';
import { and, desc, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { HTTPException } from 'hono/http-exception';
import ky from 'ky';
import satori from 'satori';
import sharp from 'sharp';
import { match } from 'ts-pattern';
import twemoji from 'twemoji';
import { db, Documents, Entities, first, Folders, Images, PublicationVersions, Spaces } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import { buildPublishedPublicationByPermalinkQuery } from '#/utils/publication-view-core.ts';
import type { ReactNode } from 'react';
import type { Env, ServerContext } from '#/context.ts';

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

og.get('/p/:permalink', async (c) => {
  const permalink = c.req.param('permalink');

  const publication = await buildPublishedPublicationByPermalinkQuery(db, { permalink }).then(first);

  if (!publication) {
    throw new HTTPException(404);
  }

  const version = await db
    .select({ title: PublicationVersions.title, subtitle: PublicationVersions.subtitle, thumbnailPath: Images.path })
    .from(PublicationVersions)
    .leftJoin(Images, eq(Images.id, PublicationVersions.thumbnailId))
    .where(eq(PublicationVersions.publicationId, publication.id))
    .orderBy(desc(PublicationVersions.version))
    .limit(1)
    .then(first);

  if (!version) {
    throw new HTTPException(404);
  }

  return await respondWithCard(c, await renderDocumentCard(version));
});

const COVER_WIDTH = 1280;
const COVER_HEIGHT = 720;

og.get('/cover/:permalink', async (c) => {
  const permalink = c.req.param('permalink');

  const publication = await buildPublishedPublicationByPermalinkQuery(db, { permalink }).then(first);

  if (!publication) {
    throw new HTTPException(404);
  }

  const [version, space] = await Promise.all([
    db
      .select({ title: PublicationVersions.title })
      .from(PublicationVersions)
      .where(eq(PublicationVersions.publicationId, publication.id))
      .orderBy(desc(PublicationVersions.version))
      .limit(1)
      .then(first),
    db.select({ name: Spaces.name }).from(Spaces).where(eq(Spaces.id, publication.spaceId)).then(first),
  ]);

  if (!version || !space) {
    throw new HTTPException(404);
  }

  const title = version.title || '(제목 없음)';
  const resp = await respondWithCard(c, renderTitlePageCover({ seed: publication.id + title, title, spaceName: space.name }), {
    width: COVER_WIDTH,
    height: COVER_HEIGHT,
  });
  resp.headers.set('Cache-Control', 'public, max-age=31536000, immutable');

  return resp;
});

const PREVIEW_TEXT_LIMIT = 200;

og.get('/preview', async (c) => {
  const title = c.req.query('title')?.slice(0, PREVIEW_TEXT_LIMIT) || null;
  const subtitle = c.req.query('subtitle')?.slice(0, PREVIEW_TEXT_LIMIT) || null;
  const thumbnailId = c.req.query('thumbnailId');

  const thumbnailPath = thumbnailId
    ? await db
        .select({ path: Images.path })
        .from(Images)
        .where(eq(Images.id, thumbnailId))
        .then(first)
        .then((row) => row?.path ?? null)
    : null;

  const resp = await respondWithCard(c, await renderDocumentCard({ title, subtitle, thumbnailPath }));
  resp.headers.set('Cache-Control', 'public, max-age=300');

  return resp;
});

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
    .with(EntityType.FOLDER, () => renderFolder(entityId))
    .with(EntityType.DOCUMENT, () => renderDocument(entityId))
    .with(EntityType.DIVIDER, () => {
      throw new HTTPException(404);
    })
    .exhaustive();

  return await respondWithCard(c, node);
});

const respondWithCard = async (c: ServerContext, node: Awaited<ReactNode>, size?: { width: number; height: number }) => {
  const svg = await satori(node, {
    width: size?.width ?? 1200,
    height: size?.height ?? 630,
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
          <span>TYPIE &mdash; 언제든 이어 쓰는 글쓰기 앱</span>
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

  return await renderDocumentCard(document);
};

const renderDocumentCard = async ({
  title,
  subtitle,
  thumbnailPath,
}: {
  title: string | null;
  subtitle: string | null;
  thumbnailPath: string | null;
}) => {
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
              {title ?? '(제목 없음)'}
            </div>

            {subtitle && (
              <div
                style={{
                  display: 'block',
                  fontSize: '30px',
                  fontWeight: 400,
                  color: colors.gray[500],
                  lineClamp: 1,
                }}
              >
                {subtitle}
              </div>
            )}
          </div>

          {thumbnailPath && (
            <img src={await toDataUri(thumbnailPath)} width={200} height={200} style={{ objectFit: 'cover', borderRadius: '12px' }} />
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
          <span>TYPIE &mdash; 언제든 이어 쓰는 글쓰기 앱</span>
          <span style={{ fontFamily: 'DeepMindSans' }}>typie.co</span>
        </div>
      </div>
    </div>
  );
};

const renderTitlePageCover = ({ seed, title, spaceName }: { seed: string; title: string; spaceName: string }) => {
  const colors = titlePageColors(seed);
  const unit = COVER_WIDTH / 100;

  return (
    <div
      style={{
        display: 'flex',
        width: `${COVER_WIDTH}px`,
        height: `${COVER_HEIGHT}px`,
        padding: `${unit * 4.5}px`,
        fontFamily: 'Pretendard',
        backgroundColor: colors.background,
      }}
    >
      <div
        style={{
          display: 'flex',
          flex: 1,
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: `${unit * 2.8}px`,
          padding: `${unit * 3}px ${unit * 9}px`,
          border: `3px solid ${colors.frame}`,
        }}
      >
        <div
          style={{
            display: 'block',
            maxWidth: '100%',
            fontSize: `${unit * 7.2}px`,
            fontWeight: 800,
            lineHeight: '1.3',
            letterSpacing: `${-unit * 7.2 * 0.025}px`,
            color: colors.title,
            textAlign: 'center',
            lineClamp: 3,
            wordBreak: 'keep-all',
          }}
        >
          {title}
        </div>
        <div style={{ display: 'flex', width: `${unit * 9}px`, height: '3px', backgroundColor: colors.rule }} />
        <div
          style={{
            display: 'block',
            maxWidth: '100%',
            fontSize: `${unit * 3.9}px`,
            fontWeight: 500,
            color: colors.caption,
            textAlign: 'center',
            lineClamp: 1,
          }}
        >
          {spaceName}
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
