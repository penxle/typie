import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { processFont } from '#/utils/font.ts';

const S3_BUCKET = 'typie-cdn';
const S3_PREFIX = 'editor/fonts';

const FALLBACK_FONTS = [
  {
    familyName: 'Noto Sans',
    fonts: [
      { weight: 400, path: 'NotoSans-Regular' },
      { weight: 700, path: 'NotoSans-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans KR',
    fonts: [
      { weight: 400, path: 'NotoSansKR-Regular' },
      { weight: 700, path: 'NotoSansKR-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans JP',
    fonts: [
      { weight: 400, path: 'NotoSansJP-Regular' },
      { weight: 700, path: 'NotoSansJP-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans SC',
    fonts: [
      { weight: 400, path: 'NotoSansSC-Regular' },
      { weight: 700, path: 'NotoSansSC-Bold' },
    ],
  },
  {
    familyName: 'Noto Sans Math',
    fonts: [{ weight: 400, path: 'NotoSansMath-Regular' }],
  },
  {
    familyName: 'Noto Sans Symbols',
    fonts: [{ weight: 400, path: 'NotoSansSymbols-Regular' }],
  },
  {
    familyName: 'Noto Sans Symbols 2',
    fonts: [{ weight: 400, path: 'NotoSansSymbols2-Regular' }],
  },
  {
    familyName: 'NotoColorEmoji',
    fonts: [{ weight: 400, path: 'NotoColorEmoji' }],
  },
];

const sourceDir = process.argv[2];
if (!sourceDir) {
  throw new Error('Usage: bun run build-fallback-fonts.ts <source-dir>');
}

const allFonts = FALLBACK_FONTS.flatMap((f) => f.fonts);
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
type FallbackFont = { weight: number; path: string; hash: string; chunk_count: number; chunk_map: string | null; chunk_map_sup?: number[] };
const fallbacksData: { familyName: string; fonts: FallbackFont[] }[] = [];
let uploaded = 0;
let skipped = 0;
let done = 0;

for (const family of FALLBACK_FONTS) {
  const fonts: FallbackFont[] = [];

  for (const font of family.fonts) {
    done++;
    const ttfPath = path.resolve(sourceDir, `${font.path}.ttf`);
    const ttfData = new Uint8Array(await readFile(ttfPath));

    console.log(`[${done}/${total}] Processing ${font.path}...`);
    const { manifest, strategy, base, chunks } = await processFont(font.path, ttfData);

    const baseKB = (base.length / 1024).toFixed(1);
    const chunksKB = (chunks.reduce((s, c) => s + c.length, 0) / 1024).toFixed(1);
    console.log(`  base: ${baseKB}KB, ${chunks.length} chunks: ${chunksKB}KB, strategy: ${strategy ?? 'sequential'}`);

    const s3Base = `${S3_PREFIX}/${font.path}/${manifest.hash}`;
    const filesToUpload: { key: string; body: Uint8Array }[] = [
      { key: `${s3Base}/base.bin`, body: base },
      ...chunks.map((chunk, i) => ({ key: `${s3Base}/chunks/${i}.bin`, body: chunk })),
    ];

    for (const { key, body } of filesToUpload) {
      if (existingKeys.has(key)) {
        console.log(`  SKIP ${key}`);
        skipped++;
      } else {
        console.log(`  UPLOAD ${key}`);
        await s3.send(new PutObjectCommand({ Bucket: S3_BUCKET, Key: key, Body: body, ContentType: 'application/octet-stream' }));
        uploaded++;
      }
    }

    fonts.push({ weight: font.weight, path: font.path, ...manifest });
  }

  if (fonts.length > 0) {
    fallbacksData.push({ familyName: family.familyName, fonts });
  }
}

console.log(`\nS3: ${uploaded} uploaded, ${skipped} skipped`);

// Write fallbacks.json
const workspaceDir = path.resolve(import.meta.dirname, '../../..');
const fallbacksPath = path.join(workspaceDir, 'crates/editor/assets/fallbacks.json');
await writeFile(fallbacksPath, JSON.stringify(fallbacksData));
console.log(`Written: ${fallbacksPath}`);
