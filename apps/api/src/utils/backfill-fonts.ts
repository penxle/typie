import { GetObjectCommand, HeadObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { eq } from 'drizzle-orm';
import qs from 'query-string';
import { db, first, Fonts } from '#/db/index.ts';
import { stack } from '#/env.ts';
import * as aws from '#/external/aws.ts';
import { decompressZstd } from '#/utils/compression.ts';
import { isNonEmptyHead, isUnsupportedFontFormat, processFont } from '#/utils/font.ts';
import { processFont as processFontLegacy } from '#/utils/font-legacy.ts';
import { wasm } from '#/utils/wasm-ffi.ts';

export const FONTS_BUCKET = 'typie-usercontents';

export type BackfillTarget = {
  row: { id: string; postScriptName: string; path: string; userId: string };
  needsV2: boolean;
  needsLegacy: boolean;
  needsManifest: boolean;
};

export type BackfillStatus = { status: 'success' | 'skipped'; reason?: string };
export type BackfillResult = { id: string; path: string; status: 'success' | 'skipped' | 'failed'; reason: string | null };

const getObject = async (key: string): Promise<Uint8Array> => {
  const object = await aws.s3.send(new GetObjectCommand({ Bucket: FONTS_BUCKET, Key: key }));
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return await object.Body!.transformToByteArray();
};

const putObject = async (key: string, body: Uint8Array | string, contentType: string, tagging: string) => {
  await aws.s3.send(
    new PutObjectCommand({
      Bucket: FONTS_BUCKET,
      Key: key,
      Body: body,
      ContentType: contentType,
      Tagging: tagging,
    }),
  );
};

export const objectExistsNonEmpty = async (key: string): Promise<boolean> => {
  try {
    const head = await aws.s3.send(new HeadObjectCommand({ Bucket: FONTS_BUCKET, Key: key }));
    return isNonEmptyHead(head);
  } catch (err) {
    if (err instanceof Error && err.name === 'NotFound') {
      return false;
    }
    throw err;
  }
};

export const backfillFont = async ({ row, needsV2, needsLegacy, needsManifest }: BackfillTarget): Promise<BackfillStatus> => {
  const s3Base = `fonts/${row.path}`;
  const tagging = qs.stringify({ UserId: row.userId, Environment: stack });
  let skippedReason: string | undefined;

  if (needsManifest && !needsV2) {
    const current = await db.select({ hash: Fonts.hash, chunks: Fonts.chunks }).from(Fonts).where(eq(Fonts.id, row.id)).then(first);
    if (current?.hash) {
      const hasChunkObjects = await objectExistsNonEmpty(`${s3Base}/${current.hash}/chunks/0`);
      if (hasChunkObjects) {
        const manifest = await wasm.build_font_manifest({ chunks: current.chunks as number[][] });
        await putObject(`${s3Base}/${current.hash}/manifest.v1`, manifest, 'application/octet-stream', tagging);
      } else {
        skippedReason = 'cff-suspect';
      }
    }
  }

  if (needsV2 || needsLegacy) {
    const original = await getObject(`${s3Base}/original.bin`);
    const buffer = await decompressZstd(original);

    if (needsV2) {
      try {
        const { hash, coverages, base, chunks, manifest } = await processFont(row.postScriptName, buffer);
        await Promise.all([
          putObject(`${s3Base}/${hash}/base`, base, 'application/octet-stream', tagging),
          putObject(`${s3Base}/${hash}/manifest.v1`, manifest, 'application/octet-stream', tagging),
          ...chunks.map((data, id) => putObject(`${s3Base}/${hash}/chunks/${id}`, data, 'application/octet-stream', tagging)),
        ]);
        // DB hash가 v2 완료 마커 — 산출물 업로드가 끝난 뒤에만 기록한다.
        await db.update(Fonts).set({ hash, chunks: coverages }).where(eq(Fonts.id, row.id));
      } catch (err) {
        if (isUnsupportedFontFormat(err)) {
          skippedReason = 'unsupported_font_format';
          await db.update(Fonts).set({ hash: '', chunks: [] }).where(eq(Fonts.id, row.id));
        } else {
          throw err;
        }
      }
    }

    if (needsLegacy) {
      const legacy = await processFontLegacy(row.postScriptName, buffer);
      await Promise.all([
        putObject(`${s3Base}/${legacy.manifest.hash}/base.bin`, legacy.base, 'application/octet-stream', tagging),
        ...legacy.chunks.map((chunk, i) =>
          putObject(`${s3Base}/${legacy.manifest.hash}/chunks/${i}.bin`, chunk, 'application/octet-stream', tagging),
        ),
      ]);
      // manifest.json이 legacy 완료 마커 — 반드시 마지막에 업로드한다.
      await putObject(`${s3Base}/manifest.json`, JSON.stringify(legacy.manifest), 'application/json', tagging);
    }
  }

  return skippedReason ? { status: 'skipped', reason: skippedReason } : { status: 'success' };
};
