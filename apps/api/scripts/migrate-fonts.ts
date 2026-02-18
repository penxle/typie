import path from 'node:path';
import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { and, eq, like } from 'drizzle-orm';
import qs from 'query-string';
import { db, FontFamilies, Fonts } from '@/db';
import { stack } from '@/env';
import * as aws from '@/external/aws';
import { compressZstd } from '@/utils/compression';
import { processFont } from '@/utils/font';
import { wasm } from '@/utils/wasm';

const CONCURRENCY = 10;

const rows = await db
  .select({
    fontId: Fonts.id,
    familyId: Fonts.familyId,
    fontPath: Fonts.path,
    postScriptName: Fonts.postScriptName,
    userId: FontFamilies.userId,
  })
  .from(Fonts)
  .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
  .where(like(Fonts.path, '%.woff2'));

console.log(`${rows.length} fonts to migrate (concurrency: ${CONCURRENCY})\n`);

let completed = 0;

async function migrateFont(row: (typeof rows)[number], i: number) {
  const oldPath = row.fontPath;
  const newPath = oldPath.replace(/\.woff2$/, '');

  console.log(`[${i + 1}/${rows.length}] ${oldPath} → ${newPath}`);

  // 1. Download existing woff2 from S3
  const object = await aws.s3.send(
    new GetObjectCommand({
      Bucket: 'typie-usercontents',
      Key: `fonts/${oldPath}`,
    }),
  );
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const woff2 = await object.Body!.transformToByteArray();

  // 2. Decompress woff2 → SFNT (fonttools via Python; wawoff2 has 30MB output limit)
  const woff2Proc = Bun.spawn(['python3', path.resolve(import.meta.dir, 'woff2sfnt.py')], {
    stdin: 'pipe',
    stdout: 'pipe',
    stderr: 'pipe',
  });
  woff2Proc.stdin.write(woff2);
  woff2Proc.stdin.end();
  const sfntBuf = await new Response(woff2Proc.stdout).arrayBuffer();
  const woff2ExitCode = await woff2Proc.exited;
  if (woff2ExitCode !== 0) {
    const stderr = await new Response(woff2Proc.stderr).text();
    throw new Error(`woff2 decompress failed: ${stderr}`);
  }
  const sfnt = new Uint8Array(sfntBuf);

  // 3. Check if OTF (CFF) → convert to TTF via Python fonttools
  const isOtf = sfnt[0] === 0x4f && sfnt[1] === 0x54 && sfnt[2] === 0x54 && sfnt[3] === 0x4f; // "OTTO"

  let ttfData: Uint8Array;
  if (isOtf) {
    console.log(`  [${i + 1}] OTF detected, converting to TTF via fonttools...`);
    const proc = Bun.spawn(['python3', path.resolve(import.meta.dir, 'otf2ttf.py')], {
      stdin: 'pipe',
      stdout: 'pipe',
      stderr: 'pipe',
    });
    proc.stdin.write(sfnt);
    proc.stdin.end();
    const output = await new Response(proc.stdout).arrayBuffer();
    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      const stderr = await new Response(proc.stderr).text();
      console.error(`  [${i + 1}] fonttools error: ${stderr}`);
      return;
    }
    ttfData = new Uint8Array(output);
  } else {
    ttfData = sfnt;
  }

  // 4. Extract font metadata
  const metadata = await wasm.getFontMetadata(sfnt);
  console.log(`  [${i + 1}] metadata: ${metadata.postScriptName} weight=${metadata.weight}`);

  // 4-1. Check for duplicate postScriptName in the same family
  const duplicate = await db
    .select({ id: Fonts.id })
    .from(Fonts)
    .where(and(eq(Fonts.familyId, row.familyId), eq(Fonts.postScriptName, metadata.postScriptName)))
    .then((r) => r[0]);

  if (duplicate && duplicate.id !== row.fontId) {
    console.log(`  [${i + 1}] Duplicate postScriptName "${metadata.postScriptName}" (existing: ${duplicate.id}), deleting...`);
    await db.delete(Fonts).where(eq(Fonts.id, row.fontId));
    console.log(`  [${i + 1}] Deleted`);
    return;
  }

  // 5. Process font (chunking, manifest generation)
  const fontName = metadata.postScriptName;
  const { manifest, base, chunks } = await processFont(fontName, ttfData);
  const compressed = await compressZstd(ttfData);

  const tagging = qs.stringify({
    UserId: row.userId,
    Environment: stack,
  });

  const s3Base = `fonts/${newPath}`;

  // 6. Upload to S3
  console.log(`  [${i + 1}] Uploading ${3 + chunks.length} files...`);
  await Promise.all([
    aws.s3.send(
      new PutObjectCommand({
        Bucket: 'typie-usercontents',
        Key: `${s3Base}/web.woff2`,
        Body: woff2,
        ContentType: 'font/woff2',
        Tagging: tagging,
      }),
    ),
    aws.s3.send(
      new PutObjectCommand({
        Bucket: 'typie-usercontents',
        Key: `${s3Base}/manifest.json`,
        Body: JSON.stringify(manifest),
        ContentType: 'application/json',
        Tagging: tagging,
      }),
    ),
    aws.s3.send(
      new PutObjectCommand({
        Bucket: 'typie-usercontents',
        Key: `${s3Base}/${manifest.hash}/base.bin`,
        Body: base,
        ContentType: 'application/octet-stream',
        Tagging: tagging,
      }),
    ),
    aws.s3.send(
      new PutObjectCommand({
        Bucket: 'typie-usercontents',
        Key: `${s3Base}/original.bin`,
        Body: compressed,
        ContentType: 'application/octet-stream',
        Tagging: tagging,
      }),
    ),
    ...chunks.map((chunk, j) =>
      aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-usercontents',
          Key: `${s3Base}/${manifest.hash}/chunks/${j}.bin`,
          Body: chunk,
          ContentType: 'application/octet-stream',
          Tagging: tagging,
        }),
      ),
    ),
  ]);

  // 7. Update DB
  const baseFamilyName = metadata.familyName ?? metadata.fullName ?? metadata.postScriptName;
  await db
    .update(Fonts)
    .set({
      path: newPath,
      fullName: metadata.fullName ?? null,
      postScriptName: metadata.postScriptName,
      subfamilyDisplayName: metadata.subfamilyDisplayName ?? null,
      weight: metadata.weight,
    })
    .where(eq(Fonts.id, row.fontId));

  // Find a unique familyName for this user (append (2), (3), ... if needed)
  let familyName = baseFamilyName;
  let suffix = 1;
  while (true) {
    const conflict = await db
      .select({ id: FontFamilies.id })
      .from(FontFamilies)
      .where(and(eq(FontFamilies.userId, row.userId), eq(FontFamilies.familyName, familyName)))
      .then((r) => r[0]);
    if (!conflict || conflict.id === row.familyId) break;
    suffix++;
    familyName = `${baseFamilyName} (${suffix})`;
  }

  await db
    .update(FontFamilies)
    .set({
      familyName,
      displayName: familyName === baseFamilyName ? (metadata.displayName ?? baseFamilyName) : familyName,
    })
    .where(eq(FontFamilies.id, row.familyId));

  if (familyName !== baseFamilyName) {
    console.log(`  [${i + 1}] Family name conflict, used: ${familyName}`);
  }

  completed++;
  console.log(`  [${i + 1}] Done (${completed}/${rows.length})`);
}

const MAX_RETRIES = 3;
let errors = 0;

async function migrateFontWithRetry(row: (typeof rows)[number], i: number) {
  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    try {
      await migrateFont(row, i);
      return;
    } catch (err) {
      if (attempt < MAX_RETRIES) {
        console.error(`  [${i + 1}] Attempt ${attempt} failed, retrying... (${err})`);
        await new Promise((r) => setTimeout(r, 1000 * attempt));
      } else {
        console.error(`  [${i + 1}] Failed after ${MAX_RETRIES} attempts:`, err);
        errors++;
      }
    }
  }
}

// Process with concurrency workers sharing a single iterator
const entries = rows.entries();
await Promise.all(
  Array.from({ length: CONCURRENCY }, async () => {
    for (const [i, row] of entries) {
      await migrateFontWithRetry(row, i);
    }
  }),
);

console.log(`\nMigration complete (${completed}/${rows.length} migrated, ${errors} errors)`);
