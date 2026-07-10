import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { eq } from 'drizzle-orm';
import qs from 'query-string';
import { db, Fonts } from '#/db/index.ts';
import { stack } from '#/env.ts';
import * as aws from '#/external/aws.ts';
import { decompressZstd } from '#/utils/compression.ts';
import { processFont } from '#/utils/font.ts';
import { processFont as processFontLegacy } from '#/utils/font-legacy.ts';

export const FONTS_BUCKET = 'typie-usercontents';

export type BackfillTarget = {
  row: { id: string; postScriptName: string; path: string; userId: string };
  needsV2: boolean;
  needsLegacy: boolean;
};

export type BackfillResult = { id: string; path: string; error: string | null };

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

export const backfillFont = async ({ row, needsV2, needsLegacy }: BackfillTarget): Promise<void> => {
  const s3Base = `fonts/${row.path}`;
  const original = await getObject(`${s3Base}/original.bin`);
  const buffer = await decompressZstd(original);
  const tagging = qs.stringify({ UserId: row.userId, Environment: stack });

  if (needsV2) {
    const { hash, coverages, base, chunks } = await processFont(row.postScriptName, buffer);
    await Promise.all([
      putObject(`${s3Base}/${hash}/base`, base, 'application/octet-stream', tagging),
      ...chunks.map((data, id) => putObject(`${s3Base}/${hash}/chunks/${id}`, data, 'application/octet-stream', tagging)),
    ]);
    // DB hash가 v2 완료 마커 — 산출물 업로드가 끝난 뒤에만 기록한다.
    await db.update(Fonts).set({ hash, chunks: coverages }).where(eq(Fonts.id, row.id));
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
};
