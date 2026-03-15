import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { compressZstd } from '#/utils/compression.ts';
import { processFont } from '#/utils/font.ts';
import { wasm } from '#/utils/wasm.ts';

const S3_BUCKET = 'typie-cdn';
const S3_PREFIX = 'editor/fonts';

// spell-checker:disable
const DEFAULT_FONTS = [
  {
    familyName: 'Pretendard',
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
    fonts: [
      { weight: 300, path: 'KoPubWorldDotum-Light' },
      { weight: 500, path: 'KoPubWorldDotum-Medium' },
      { weight: 700, path: 'KoPubWorldDotum-Bold' },
    ],
  },
  {
    familyName: 'NanumBarunGothic',
    fonts: [
      { weight: 200, path: 'NanumBarunGothic-UltraLight' },
      { weight: 300, path: 'NanumBarunGothic-Light' },
      { weight: 400, path: 'NanumBarunGothic-Regular' },
      { weight: 700, path: 'NanumBarunGothic-Bold' },
    ],
  },
  {
    familyName: 'RIDIBatang',
    fonts: [{ weight: 400, path: 'RIDIBatang-Regular' }],
  },
  {
    familyName: 'KoPubWorldBatang',
    fonts: [
      { weight: 300, path: 'KoPubWorldBatang-Light' },
      { weight: 500, path: 'KoPubWorldBatang-Medium' },
      { weight: 700, path: 'KoPubWorldBatang-Bold' },
    ],
  },
  {
    familyName: 'NanumMyeongjo',
    fonts: [
      { weight: 400, path: 'NanumMyeongjo-Regular' },
      { weight: 700, path: 'NanumMyeongjo-Bold' },
      { weight: 800, path: 'NanumMyeongjo-ExtraBold' },
    ],
  },
];
// spell-checker:enable

const sourceDir = process.argv[2];
if (!sourceDir) {
  throw new Error('Usage: bun run build-default-fonts.ts <source-dir>');
}

const allFonts = DEFAULT_FONTS.flatMap((f) => f.fonts);
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

// Process fonts and upload to S3
type DefaultFontEntry = {
  id: string;
  postScriptName: string;
  weight: number;
  path: string;
  names: { nameId: number; platformId: number; languageId: number; value: string }[];
};

const defaultsData: { id: string; familyName: string; fonts: DefaultFontEntry[] }[] = [];
let uploaded = 0;
let skipped = 0;
let done = 0;

for (const family of DEFAULT_FONTS) {
  const familyId = `!${family.familyName.toUpperCase()}`;
  const fonts: DefaultFontEntry[] = [];

  for (const font of family.fonts) {
    done++;
    const ttfPath = path.resolve(sourceDir, `${font.path}.ttf`);
    const ttfData = new Uint8Array(await readFile(ttfPath));

    console.log(`[${done}/${total}] Processing ${font.path}...`);
    const { manifest, strategy, base, chunks } = await processFont(font.path, ttfData);

    const baseKB = (base.length / 1024).toFixed(1);
    const chunksKB = (chunks.reduce((s, c) => s + c.length, 0) / 1024).toFixed(1);
    console.log(`  base: ${baseKB}KB, ${chunks.length} chunks: ${chunksKB}KB, strategy: ${strategy ?? 'sequential'}`);

    // Extract font metadata
    const metadata = await wasm.getFontMetadata(ttfData);
    const findName = (nameId: number) =>
      metadata.names.find((n) => n.nameId === nameId && n.platformId === 3 && n.languageId === 0x04_09)?.value ??
      metadata.names.find((n) => n.nameId === nameId)?.value;

    const postScriptName = findName(6) ?? font.path;

    fonts.push({
      id: `${familyId}${font.weight}`,
      postScriptName,
      weight: font.weight,
      path: font.path,
      names: metadata.names.map((n) => ({ nameId: n.nameId, platformId: n.platformId, languageId: n.languageId, value: n.value })),
    });

    // Upload manifest.json (always overwrite - no hash in key)
    const manifestKey = `${S3_PREFIX}/${font.path}/manifest.json`;
    console.log(`  PUT ${manifestKey}`);
    await s3.send(
      new PutObjectCommand({
        Bucket: S3_BUCKET,
        Key: manifestKey,
        Body: JSON.stringify(manifest),
        ContentType: 'application/json',
      }),
    );

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

    const s3Base = `${S3_PREFIX}/${font.path}/${manifest.hash}`;
    const filesToUpload: { key: string; body: Uint8Array | string; contentType: string }[] = [
      { key: `${s3Base}/base.bin`, body: base, contentType: 'application/octet-stream' },
      ...chunks.map((chunk, i) => ({ key: `${s3Base}/chunks/${i}.bin`, body: chunk, contentType: 'application/octet-stream' })),
    ];

    for (const { key, body, contentType } of filesToUpload) {
      if (existingKeys.has(key)) {
        console.log(`  SKIP ${key}`);
        skipped++;
      } else {
        console.log(`  UPLOAD ${key}`);
        await s3.send(new PutObjectCommand({ Bucket: S3_BUCKET, Key: key, Body: body, ContentType: contentType }));
        uploaded++;
      }
    }
  }

  defaultsData.push({ id: familyId, familyName: family.familyName, fonts });
}

console.log(`\nS3: ${uploaded} uploaded, ${skipped} skipped`);

// Write defaults.json
const workspaceDir = path.resolve(import.meta.dirname, '../../..');
const defaultsPath = path.join(workspaceDir, 'crates/editor/assets/defaults.json');
await writeFile(defaultsPath, JSON.stringify(defaultsData));
console.log(`Written: ${defaultsPath}`);
