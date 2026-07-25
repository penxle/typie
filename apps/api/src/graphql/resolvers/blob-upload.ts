import path from 'node:path';
import { GetObjectCommand, HeadObjectCommand } from '@aws-sdk/client-s3';
import { createPresignedPost } from '@aws-sdk/s3-presigned-post';
import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createDbId, db, Files, first, Images, TableCode, validateDbId } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import {
  abandonBlobLease,
  claimBlobLeaseForFinalize,
  createBlobLeases,
  deleteBlobLease,
  isValidAssetId,
  readBlobLeases,
  releaseFinalizeClaim,
} from '#/utils/blob-lease.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { builder } from '../builder.ts';
import { File, Image } from '../objects.ts';
import { persistFileObject, persistImageBuffer } from './blob.ts';
import type { BlobLease, BlobLeaseKind } from '#/utils/blob-lease.ts';

const MAX_RESERVE_ITEMS = 50;
const MAX_UPLOAD_SIZE = 1024 * 1024 * 1024; // 1GB
const UPLOAD_URL_EXPIRES_IN = 60 * 5; // 5 minutes

export const normalizeContentType = (contentType: string | null | undefined): string => {
  return contentType?.trim() || 'application/octet-stream';
};

const isValidModification = (modification: unknown): boolean =>
  modification === undefined || modification === null || (typeof modification === 'object' && !Array.isArray(modification));

const validationError = () => new TypieError({ code: 'validation_error', status: 400 });

/**
 * * Reservation
 */

export type ReserveBlobUploadItem = {
  assetId?: string | null;
  kind: BlobLeaseKind;
  name: string;
  format: string;
  size: number;
};

export type BlobUploadReservation = {
  assetId: string;
  nonce: string;
  uploadUrl: string;
  uploadFields: Record<string, string>;
  path: string;
};

export type ReserveBlobUploadsDeps = {
  assertSubscription: (userId: string) => Promise<void>;
  hasCompletedAsset: (assetId: string, kind: BlobLeaseKind) => Promise<boolean>;
  readLeases: (assetIds: string[]) => Promise<(BlobLease | null)[]>;
  createLeases: (entries: [assetId: string, lease: BlobLease][]) => Promise<boolean>;
  createAssetId: (kind: BlobLeaseKind) => string;
  createNonce: () => string;
  createObjectKey: () => string;
  presign: (args: { key: string; name: string; contentType: string; size: number; userId: string }) => Promise<{
    url: string;
    fields: Record<string, string>;
  }>;
  publishAssetsChanged: (documentId: string, ids: string[]) => Promise<void>;
  now: () => number;
};

export const reserveBlobUploads = async ({
  documentId,
  userId,
  items,
  deps,
}: {
  documentId: string;
  userId: string;
  items: ReserveBlobUploadItem[];
  deps: ReserveBlobUploadsDeps;
}): Promise<BlobUploadReservation[]> => {
  await deps.assertSubscription(userId);

  if (items.length === 0 || items.length > MAX_RESERVE_ITEMS) {
    throw validationError();
  }

  const reclaimed = items.filter((item) => !!item.assetId) as (ReserveBlobUploadItem & { assetId: string })[];
  if (new Set(reclaimed.map((item) => item.assetId)).size !== reclaimed.length) {
    throw validationError();
  }

  for (const item of items) {
    const { size } = item;
    if (!Number.isSafeInteger(size) || size < 0 || size > MAX_UPLOAD_SIZE) {
      throw validationError();
    }

    if (item.assetId && !isValidAssetId(item.assetId, item.kind)) {
      throw validationError();
    }
  }

  if (reclaimed.length > 0) {
    const completed = await Promise.all(reclaimed.map((item) => deps.hasCompletedAsset(item.assetId, item.kind)));
    if (completed.some(Boolean)) {
      throw new TypieError({ code: 'asset_already_exists', status: 409 });
    }

    const leases = await deps.readLeases(reclaimed.map((item) => item.assetId));
    if (leases.some((lease) => lease !== null)) {
      throw new TypieError({ code: 'asset_upload_in_progress', status: 409 });
    }
  }

  const createdAt = deps.now();

  const reserved = await Promise.all(
    items.map(async (item) => {
      const assetId = item.assetId ?? deps.createAssetId(item.kind);
      const format = normalizeContentType(item.format);
      const key = `${deps.createObjectKey()}${path.extname(item.name)}`;
      const nonce = deps.createNonce();

      const presigned = await deps.presign({ key, name: item.name, contentType: format, size: item.size, userId });

      const lease: BlobLease = {
        documentId,
        userId,
        kind: item.kind,
        name: item.name,
        format,
        size: item.size,
        path: key,
        nonce,
        state: 'pending',
        createdAt,
      };

      return {
        lease,
        reservation: { assetId, nonce, uploadUrl: presigned.url, uploadFields: presigned.fields, path: key },
      };
    }),
  );

  const created = await deps.createLeases(reserved.map(({ reservation, lease }) => [reservation.assetId, lease]));
  if (!created) {
    throw new TypieError({ code: 'asset_upload_in_progress', status: 409 });
  }

  const reservations = reserved.map(({ reservation }) => reservation);
  await deps.publishAssetsChanged(
    documentId,
    reservations.map((reservation) => reservation.assetId),
  );

  return reservations;
};

/**
 * * Finalization
 */

export type UploadedObject = {
  userId: string | undefined;
  contentType: string | undefined;
  contentLength: number | undefined;
  body: Uint8Array | undefined;
};

export type FinalizeBlobUploadDeps<TRow> = {
  loadCompletedAsset: (assetId: string) => Promise<TRow | null>;
  readLease: (assetId: string) => Promise<BlobLease | null>;
  claimLease: (assetId: string, nonce: string, modification?: unknown) => Promise<BlobLease | null>;
  releaseClaim: (assetId: string, nonce: string) => Promise<void>;
  deleteLease: (assetId: string, nonce: string) => Promise<void>;
  readUploadedObject: (path: string) => Promise<UploadedObject>;
  createOutputKey: () => string;
  persist: (args: { assetId: string; lease: BlobLease; object: UploadedObject; outputKey: string }) => Promise<TRow>;
  isUniqueViolation: (error: unknown) => boolean;
  publishAssetsChanged: (documentId: string, ids: string[]) => Promise<void>;
};

// completed row가 이미 있는 경우의 뒷정리 — 앞선 run이 lease DEL·무효화를 남기고 죽었을 수 있으므로
// nonce가 일치하는 lease가 남아 있으면 지우고 무효화를 다시 발행한다.
const settleCompletedLease = async <TRow>({
  assetId,
  nonce,
  deps,
}: {
  assetId: string;
  nonce: string;
  deps: FinalizeBlobUploadDeps<TRow>;
}) => {
  const lease = await deps.readLease(assetId);
  if (!lease || lease.nonce !== nonce) {
    return;
  }

  await deps.deleteLease(assetId, nonce);
  await deps.publishAssetsChanged(lease.documentId, [assetId]);
};

const rejectFinalize = async <TRow>({
  assetId,
  nonce,
  deps,
}: {
  assetId: string;
  nonce: string;
  deps: FinalizeBlobUploadDeps<TRow>;
}): Promise<never> => {
  await deps.releaseClaim(assetId, nonce);
  throw new TypieError({ code: 'asset_finalize_rejected', status: 400 });
};

export const finalizeBlobUpload = async <TRow>({
  assetId,
  nonce,
  kind,
  userId,
  modification,
  deps,
}: {
  assetId: string;
  nonce: string;
  kind: BlobLeaseKind;
  userId: string;
  modification?: unknown;
  deps: FinalizeBlobUploadDeps<TRow>;
}): Promise<TRow> => {
  const settled = await deps.loadCompletedAsset(assetId);
  if (settled) {
    await settleCompletedLease({ assetId, nonce, deps });
    return settled;
  }

  // 선점은 상태 변경(pending→finalizing + modification 고정)이므로 소유자 확인이 그보다 앞서야 한다.
  // 남의 lease를 선점해 놓고 던지면 TTL 동안 finalizing으로 굳어 abandon(=reselect)까지 막힌다.
  const claimable = await deps.readLease(assetId);
  if (claimable && claimable.nonce === nonce && claimable.userId !== userId) {
    throw new TypieError({ code: 'forbidden', status: 403 });
  }

  const lease = await deps.claimLease(assetId, nonce, modification);
  if (!lease) {
    const raced = await deps.loadCompletedAsset(assetId);
    if (!raced) {
      throw new TypieError({ code: 'asset_upload_expired', status: 409 });
    }

    await settleCompletedLease({ assetId, nonce, deps });
    return raced;
  }

  if (lease.userId !== userId) {
    await deps.releaseClaim(assetId, nonce);
    throw new TypieError({ code: 'forbidden', status: 403 });
  }

  if (lease.kind !== kind || !isValidModification(lease.modification)) {
    await rejectFinalize({ assetId, nonce, deps });
  }

  let object: UploadedObject;
  try {
    object = await deps.readUploadedObject(lease.path);
  } catch {
    throw new TypieError({ code: 'asset_finalize_retryable', status: 503 });
  }

  if (
    object.userId !== lease.userId ||
    normalizeContentType(object.contentType) !== normalizeContentType(lease.format) ||
    object.contentLength !== lease.size
  ) {
    await rejectFinalize({ assetId, nonce, deps });
  }

  // run마다 새 키를 뽑되 확장자는 업로드 path와 맞춰, 우리가 쓰는 다른 모든 키와 같은 모양을 유지한다.
  const outputKey = `${deps.createOutputKey()}${path.extname(lease.path)}`;

  // 여기서부터는 helper가 낸 예외를 종류로 판별하지 않는다 — PK 위반(경합 패배)만 걸러내고 나머지는 전부 retryable.
  let row: TRow;
  try {
    row = await deps.persist({ assetId, lease, object, outputKey });
  } catch (err) {
    const raced = deps.isUniqueViolation(err) ? await deps.loadCompletedAsset(assetId).catch(() => null) : null;
    if (!raced) {
      throw new TypieError({ code: 'asset_finalize_retryable', status: 503 });
    }

    row = raced;
  }

  await deps.deleteLease(assetId, nonce);
  await deps.publishAssetsChanged(lease.documentId, [assetId]);

  return row;
};

/**
 * * Abandon
 */

export type AbandonBlobUploadDeps = {
  abandonLease: (assetId: string, nonce: string, userId: string) => Promise<string | null>;
  publishAssetsChanged: (documentId: string, ids: string[]) => Promise<void>;
};

export const abandonBlobUpload = async ({
  assetId,
  nonce,
  userId,
  deps,
}: {
  assetId: string;
  nonce: string;
  userId: string;
  deps: AbandonBlobUploadDeps;
}): Promise<boolean> => {
  const documentId = await deps.abandonLease(assetId, nonce, userId);
  if (!documentId) {
    return false;
  }

  await deps.publishAssetsChanged(documentId, [assetId]);

  return true;
};

/**
 * * Production dependencies
 */

// pubsub은 import 시점에 redis 소켓을 여는 모듈이라 첫 발행 시점에만 동적으로 로드한다.
const publishAssetsChanged = async (documentId: string, ids: string[]) => {
  const { pubsub } = await import('#/pubsub.ts');
  pubsub.publish('document:assets', documentId, { ids });
};

// drizzle는 드라이버 오류를 DrizzleQueryError로 감싸므로 cause 체인을 따라 SQLSTATE(23505)를 찾는다.
export const isUniqueViolation = (error: unknown): boolean => {
  let cursor = error;

  for (let depth = 0; depth < 5; depth += 1) {
    if (typeof cursor !== 'object' || cursor === null) {
      return false;
    }

    if ((cursor as { code?: unknown }).code === '23505') {
      return true;
    }

    cursor = (cursor as { cause?: unknown }).cause;
  }

  return false;
};

const reserveDeps: ReserveBlobUploadsDeps = {
  assertSubscription: (userId) => assertActiveSubscription({ userId }),
  hasCompletedAsset: async (assetId, kind) => {
    const rows =
      kind === 'image'
        ? await db.select({ id: Images.id }).from(Images).where(eq(Images.id, assetId))
        : await db.select({ id: Files.id }).from(Files).where(eq(Files.id, assetId));

    return rows.length > 0;
  },
  readLeases: readBlobLeases,
  createLeases: createBlobLeases,
  createAssetId: (kind) => createDbId(kind === 'image' ? TableCode.IMAGES : TableCode.FILES),
  createNonce: () => nanoid(),
  createObjectKey: aws.createFragmentedS3ObjectKey,
  presign: async ({ key, name, contentType, size, userId }) => {
    const req = await createPresignedPost(aws.s3, {
      Bucket: 'typie-uploads',
      Key: key,
      Conditions: [['content-length-range', size, size]],
      Fields: {
        'Content-Type': contentType,
        'x-amz-meta-name': encodeURIComponent(name),
        'x-amz-meta-user-id': userId,
      },
      Expires: UPLOAD_URL_EXPIRES_IN,
    });

    return { url: req.url, fields: req.fields };
  },
  publishAssetsChanged,
  now: () => Date.now(),
};

const finalizeLeaseDeps = {
  readLease: async (assetId: string) => {
    const leases = await readBlobLeases([assetId]);
    return leases[0];
  },
  claimLease: claimBlobLeaseForFinalize,
  releaseClaim: releaseFinalizeClaim,
  deleteLease: deleteBlobLease,
  createOutputKey: aws.createFragmentedS3ObjectKey,
  isUniqueViolation,
  publishAssetsChanged,
};

const imageFinalizeDeps: FinalizeBlobUploadDeps<typeof Images.$inferSelect> = {
  ...finalizeLeaseDeps,
  loadCompletedAsset: async (assetId) => {
    const image = await db.select().from(Images).where(eq(Images.id, assetId)).then(first);
    return image ?? null;
  },
  readUploadedObject: async (key) => {
    const object = await aws.s3.send(new GetObjectCommand({ Bucket: 'typie-uploads', Key: key }));
    const body = await object.Body?.transformToByteArray();

    return {
      userId: object.Metadata?.['user-id'],
      contentType: object.ContentType,
      contentLength: object.ContentLength,
      body,
    };
  },
  persist: async ({ assetId, lease, object, outputKey }) => {
    if (!object.body) {
      throw new Error('Uploaded object has no body');
    }

    return await persistImageBuffer({
      buffer: object.body,
      name: lease.name,
      outputKey,
      userId: lease.userId,
      originalContentType: lease.format,
      modification: lease.modification,
      explicitId: assetId,
    });
  },
};

const fileFinalizeDeps: FinalizeBlobUploadDeps<typeof Files.$inferSelect> = {
  ...finalizeLeaseDeps,
  loadCompletedAsset: async (assetId) => {
    const file = await db.select().from(Files).where(eq(Files.id, assetId)).then(first);
    return file ?? null;
  },
  readUploadedObject: async (key) => {
    const head = await aws.s3.send(new HeadObjectCommand({ Bucket: 'typie-uploads', Key: key }));

    return {
      userId: head.Metadata?.['user-id'],
      contentType: head.ContentType,
      contentLength: head.ContentLength,
      body: undefined,
    };
  },
  persist: async ({ assetId, lease, outputKey }) =>
    await persistFileObject({
      sourcePath: lease.path,
      outputKey,
      name: lease.name,
      contentType: lease.format,
      contentLength: lease.size,
      userId: lease.userId,
      explicitId: assetId,
    }),
};

/**
 * * Types
 */

const BlobUploadKind = builder.enumType('BlobUploadKind', {
  values: { IMAGE: { value: 'image' }, FILE: { value: 'file' } } as const,
});

const ReserveBlobUploadItemInput = builder.inputType('ReserveBlobUploadItemInput', {
  fields: (t) => ({
    assetId: t.id({ required: false }),
    kind: t.field({ type: BlobUploadKind, required: true }),
    name: t.string({ required: true }),
    format: t.string({ required: true }),
    size: t.field({ type: 'BigInt', required: true }),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  reserveBlobUploads: t.withAuth({ session: true }).fieldWithInput({
    type: [
      t.builder.simpleObject('BlobUploadReservation', {
        fields: (t) => ({
          assetId: t.id(),
          nonce: t.string(),
          uploadUrl: t.string(),
          uploadFields: t.field({ type: 'JSON' }),
          path: t.string(),
        }),
      }),
    ],
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      items: t.input.field({ type: [ReserveBlobUploadItemInput] }),
    },
    resolve: async (_, { input }, ctx) => {
      return await reserveBlobUploads({
        documentId: input.documentId,
        userId: ctx.session.userId,
        items: input.items,
        deps: reserveDeps,
      });
    },
  }),

  finalizeBlobUploadAsImage: t.withAuth({ session: true }).fieldWithInput({
    type: Image,
    input: {
      assetId: t.input.id({ validate: validateDbId(TableCode.IMAGES) }),
      nonce: t.input.string(),
      modification: t.input.field({ type: 'JSON', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      return await finalizeBlobUpload({
        assetId: input.assetId,
        nonce: input.nonce,
        kind: 'image',
        userId: ctx.session.userId,
        modification: input.modification ?? undefined,
        deps: imageFinalizeDeps,
      });
    },
  }),

  finalizeBlobUploadAsFile: t.withAuth({ session: true }).fieldWithInput({
    type: File,
    input: {
      assetId: t.input.id({ validate: validateDbId(TableCode.FILES) }),
      nonce: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      return await finalizeBlobUpload({
        assetId: input.assetId,
        nonce: input.nonce,
        kind: 'file',
        userId: ctx.session.userId,
        deps: fileFinalizeDeps,
      });
    },
  }),

  abandonBlobUpload: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      assetId: t.input.id(),
      nonce: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      return await abandonBlobUpload({
        assetId: input.assetId,
        nonce: input.nonce,
        userId: ctx.session.userId,
        deps: { abandonLease: abandonBlobLease, publishAssetsChanged },
      });
    },
  }),
}));
