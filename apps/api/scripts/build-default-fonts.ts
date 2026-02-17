import path from 'node:path';
import { ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { DEFAULT_FONT_FAMILIES } from '@/const';
import { compressZstd } from '@/utils/compression';
import { processFont } from '@/utils/font';

const S3_BUCKET = 'typie-cdn';
const S3_PREFIX = 'editor/fonts';

const sourceDir = process.argv[2];
if (!sourceDir) {
  throw new Error('Usage: bun run build-default-fonts.ts <source-dir>');
}

const allFonts = DEFAULT_FONT_FAMILIES.flatMap((f) => f.fonts);
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
let uploaded = 0;
let skipped = 0;
let done = 0;

for (const family of DEFAULT_FONT_FAMILIES) {
  for (const font of family.fonts) {
    done++;
    const ttfPath = path.resolve(sourceDir, `${font.path}.ttf`);
    const ttfData = new Uint8Array(await Bun.file(ttfPath).bytes());

    console.log(`[${done}/${total}] Processing ${font.path}...`);
    const { manifest, strategy, base, chunks } = await processFont(font.path, ttfData);

    const baseKB = (base.length / 1024).toFixed(1);
    const chunksKB = (chunks.reduce((s, c) => s + c.length, 0) / 1024).toFixed(1);
    console.log(`  base: ${baseKB}KB, ${chunks.length} chunks: ${chunksKB}KB, strategy: ${strategy ?? 'sequential'}`);

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
}

console.log(`\nS3: ${uploaded} uploaded, ${skipped} skipped`);
