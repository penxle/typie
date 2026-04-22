import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { compressZstd } from '#/utils/compression.ts';
import { processFont } from '#/utils/font.ts';
import { wasm } from '#/utils/wasm-ffi.ts';

const S3_BUCKET = 'typie-cdn';
const S3_PREFIX = 'editor/fonts';

type FontEntry = { weight: number; path: string };
type FamilyDef = {
  familyName: string;
  source: 'DEFAULT' | 'FALLBACK';
  fonts: FontEntry[];
};

const FONTS: FamilyDef[] = [
  // spell-checker:disable
  {
    familyName: 'Pretendard',
    source: 'DEFAULT',
    fonts: [
      { weight: 100, path: 'Pretendard-Thin' },
      { weight: 200, path: 'Pretendard-ExtraLight' },
      { weight: 300, path: 'Pretendard-Light' },
      { weight: 400, path: 'Pretendard-Regular' },
      { weight: 500, path: 'Pretendard-Medium' },
      { weight: 600, path: 'Pretendard-SemiBold' },
      { weight: 700, path: 'Pretendard-Bold' },
      { weight: 800, path: 'Pretendard-ExtraBold' },
      { weight: 900, path: 'Pretendard-Black' },
    ],
  },
  {
    familyName: 'KoPubWorldDotum',
    source: 'DEFAULT',
    fonts: [
      { weight: 300, path: 'KoPubWorldDotum-Light' },
      { weight: 500, path: 'KoPubWorldDotum-Medium' },
      { weight: 700, path: 'KoPubWorldDotum-Bold' },
    ],
  },
  {
    familyName: 'NanumBarunGothic',
    source: 'DEFAULT',
    fonts: [
      { weight: 200, path: 'NanumBarunGothic-UltraLight' },
      { weight: 300, path: 'NanumBarunGothic-Light' },
      { weight: 400, path: 'NanumBarunGothic-Regular' },
      { weight: 700, path: 'NanumBarunGothic-Bold' },
    ],
  },
  {
    familyName: 'RIDIBatang',
    source: 'DEFAULT',
    fonts: [{ weight: 400, path: 'RIDIBatang-Regular' }],
  },
  {
    familyName: 'KoPubWorldBatang',
    source: 'DEFAULT',
    fonts: [
      { weight: 300, path: 'KoPubWorldBatang-Light' },
      { weight: 500, path: 'KoPubWorldBatang-Medium' },
      { weight: 700, path: 'KoPubWorldBatang-Bold' },
    ],
  },
  {
    familyName: 'NanumMyeongjo',
    source: 'DEFAULT',
    fonts: [
      { weight: 400, path: 'NanumMyeongjo-Regular' },
      { weight: 700, path: 'NanumMyeongjo-Bold' },
      { weight: 800, path: 'NanumMyeongjo-ExtraBold' },
    ],
  },
  // spell-checker:enable
  {
    familyName: 'Twemoji',
    source: 'FALLBACK',
    fonts: [{ weight: 400, path: 'Twemoji' }],
  },
  {
    familyName: 'Noto Sans',
    source: 'FALLBACK',
    fonts: [
      { weight: 400, path: 'NotoSans-Regular' },
      { weight: 700, path: 'NotoSans-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans KR',
    source: 'FALLBACK',
    fonts: [
      { weight: 400, path: 'NotoSansKR-Regular' },
      { weight: 700, path: 'NotoSansKR-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans JP',
    source: 'FALLBACK',
    fonts: [
      { weight: 400, path: 'NotoSansJP-Regular' },
      { weight: 700, path: 'NotoSansJP-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans SC',
    source: 'FALLBACK',
    fonts: [
      { weight: 400, path: 'NotoSansSC-Regular' },
      { weight: 700, path: 'NotoSansSC-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans Math',
    source: 'FALLBACK',
    fonts: [{ weight: 400, path: 'NotoSansMath-Regular' }],
  },
  {
    familyName: 'Noto Sans Symbols',
    source: 'FALLBACK',
    fonts: [{ weight: 400, path: 'NotoSansSymbols-Regular' }],
  },
  {
    familyName: 'Noto Sans Symbols 2',
    source: 'FALLBACK',
    fonts: [{ weight: 400, path: 'NotoSansSymbols2-Regular' }],
  },
];

const sourceDir = process.argv[2];
if (!sourceDir) {
  throw new Error('Usage: bun run build-fonts.ts <source-dir>');
}

const allFonts = FONTS.flatMap((f) => f.fonts);
const total = allFonts.length;
console.log(`${total} fonts to process\n`);

const s3 = new S3Client();

// List existing S3 keys for dedup
console.log('Listing existing S3 keys...');
const existingKeys = new Set<string>();
let token: string | undefined;
do {
  const resp = await s3.send(
    new ListObjectsV2Command({
      Bucket: S3_BUCKET,
      Prefix: `${S3_PREFIX}/`,
      ContinuationToken: token,
    }),
  );
  for (const obj of resp.Contents ?? []) {
    if (obj.Key) existingKeys.add(obj.Key);
  }
  token = resp.NextContinuationToken;
} while (token);
console.log(`Found ${existingKeys.size} existing keys\n`);

type FontRecord = {
  id: string;
  postScriptName: string;
  weight: number;
  path: string;
  hash: string;
  /** chunk별 flat 페어 `[start0, end0, start1, end1, ...]` (inclusive). */
  chunks: number[][];
  names: { nameId: number; platformId: number; languageId: number; value: string }[];
};

type FamilyRecord = {
  id: string;
  familyName: string;
  source: 'DEFAULT' | 'FALLBACK';
  fonts: FontRecord[];
};

const fontsData: FamilyRecord[] = [];
let uploaded = 0;
let skipped = 0;
let done = 0;

for (const family of FONTS) {
  const familyId = `!${family.source}:${family.familyName}`;
  const fonts: FontRecord[] = [];

  for (const font of family.fonts) {
    done++;
    const ttfPath = path.resolve(sourceDir, `${font.path}.ttf`);
    const ttfData = new Uint8Array(await readFile(ttfPath));

    console.log(`[${done}/${total}] Processing ${font.path}...`);
    const { hash, strategy, coverages, base, chunks } = await processFont(font.path, ttfData);

    const totalKB = ((base.length + chunks.reduce((s, c) => s + c.length, 0)) / 1024).toFixed(1);
    console.log(`  ${chunks.length} chunks: ${totalKB}KB, strategy: ${strategy ?? 'sequential'}`);

    // Extract font metadata
    const metadata = await wasm.get_font_metadata(ttfData);
    const findName = (nameId: number) =>
      metadata.names.find((n) => n.nameId === nameId && n.platformId === 3 && n.languageId === 0x04_09)?.value ??
      metadata.names.find((n) => n.nameId === nameId)?.value;

    const postScriptName = findName(6) ?? font.path;

    const fontId = `${familyId}:${font.weight}`;

    fonts.push({
      id: fontId,
      postScriptName,
      weight: font.weight,
      path: font.path,
      hash,
      chunks: coverages,
      names: metadata.names.map((n) => ({ nameId: n.nameId, platformId: n.platformId, languageId: n.languageId, value: n.value })),
    });

    // Upload the raw TTF (compressed) for server-side specimen rendering.
    const originalKey = `${S3_PREFIX}/${font.path}/original.bin`;
    const compressed = await compressZstd(ttfData);
    console.log(`  PUT ${originalKey}`);
    await s3.send(
      new PutObjectCommand({
        Bucket: S3_BUCKET,
        Key: originalKey,
        Body: compressed,
        ContentType: 'application/octet-stream',
      }),
    );

    // Upload encoded base + chunks keyed by hash.
    const hashBase = `${S3_PREFIX}/${font.path}/${hash}`;
    {
      const key = `${hashBase}/base`;
      if (existingKeys.has(key)) {
        console.log(`  SKIP ${key}`);
        skipped++;
      } else {
        console.log(`  UPLOAD ${key}`);
        await s3.send(new PutObjectCommand({ Bucket: S3_BUCKET, Key: key, Body: base, ContentType: 'application/octet-stream' }));
        uploaded++;
      }
    }
    for (const [id, chunk] of chunks.entries()) {
      const key = `${hashBase}/chunks/${id}`;
      if (existingKeys.has(key)) {
        console.log(`  SKIP ${key}`);
        skipped++;
      } else {
        console.log(`  UPLOAD ${key}`);
        await s3.send(new PutObjectCommand({ Bucket: S3_BUCKET, Key: key, Body: chunk, ContentType: 'application/octet-stream' }));
        uploaded++;
      }
    }
  }

  fontsData.push({ id: familyId, familyName: family.familyName, source: family.source, fonts });
}

console.log(`\nS3: ${uploaded} uploaded, ${skipped} skipped`);

const workspaceDir = path.resolve(import.meta.dirname, '../../..');
const fontsPath = path.join(workspaceDir, 'assets/fonts.json');
await writeFile(fontsPath, JSON.stringify(fontsData));
console.log(`Written: ${fontsPath}`);
