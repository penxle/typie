import { createHash } from 'node:crypto';
import { DocumentContentRating, DocumentViewBodyUnavailableReason } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { redis } from '#/cache.ts';
import { db, first, UserPersonalIdentities } from '#/db/index.ts';
import { getKoreanAge } from './date.ts';
import type { Context } from '#/context.ts';
import type { Documents } from '#/db/index.ts';

export async function checkDocumentViewAccess(
  document: Pick<typeof Documents.$inferSelect, 'id' | 'contentRating' | 'password'>,
  ctx: Context,
): Promise<{ accessible: true } | { accessible: false; reason: DocumentViewBodyUnavailableReason }> {
  if (document.contentRating !== DocumentContentRating.ALL) {
    if (!ctx.session) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    const identity = await db
      .select({
        birthday: UserPersonalIdentities.birthDate,
        expiresAt: UserPersonalIdentities.expiresAt,
      })
      .from(UserPersonalIdentities)
      .where(eq(UserPersonalIdentities.userId, ctx.session.userId))
      .then(first);

    if (!identity) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    if (identity.expiresAt.isBefore(dayjs())) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_IDENTITY_VERIFICATION };
    }

    const minAge = match(document.contentRating)
      .with(DocumentContentRating.R15, () => 15)
      .with(DocumentContentRating.R19, () => 19)
      .exhaustive();

    if (getKoreanAge(identity.birthday) < minAge) {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_MINIMUM_AGE };
    }
  }

  if (document.password !== null) {
    const passwordUnlock = await redis.get(
      getDocumentViewUnlockKey({
        documentId: document.id,
        deviceId: ctx.deviceId,
        password: document.password,
      }),
    );

    if (passwordUnlock !== 'true') {
      return { accessible: false, reason: DocumentViewBodyUnavailableReason.REQUIRE_PASSWORD };
    }
  }

  return { accessible: true };
}

export function getDocumentViewUnlockKey({
  documentId,
  deviceId,
  password,
}: {
  documentId: string;
  deviceId: string;
  password: string;
}): string {
  const passwordHash = createHash('sha256').update(password).digest('hex');

  return `documentview:unlock:${documentId}:${deviceId}:${passwordHash}`;
}

export const RESTRICTED_EXCERPT = '(미리보기가 제한된 문서입니다)';
