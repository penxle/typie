import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { db, first, firstOrThrow } from '@/db';
import { UserPersonalIdentities } from '@/db/schemas/tables';
import { UserPersonalIdentityKind } from '@/enums';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';

type FinalizeIdentityVerificationByPhoneParams = {
  userId: string;
  identityVerificationId: string;
};
export const finalizeIdentityVerificationByPhone = async ({
  userId,
  identityVerificationId,
}: FinalizeIdentityVerificationByPhoneParams) => {
  const resp = await portone.getIdentityVerification({ identityVerificationId });

  if (resp.status !== 'succeeded') {
    throw new TypieError({ code: 'identity_verification_failed' });
  }

  const birthday = dayjs.kst(resp.birthDate).startOf('day');

  const existingIdentity = await db
    .select({
      id: UserPersonalIdentities.id,
      kind: UserPersonalIdentities.kind,
      birthday: UserPersonalIdentities.birthday,
      name: UserPersonalIdentities.name,
      ci: UserPersonalIdentities.ci,
    })
    .from(UserPersonalIdentities)
    .where(eq(UserPersonalIdentities.userId, userId))
    .then(first);

  if (existingIdentity) {
    if (existingIdentity.kind === UserPersonalIdentityKind.PHONE && existingIdentity.ci !== resp.ci) {
      throw new TypieError({ code: 'identity_not_match' });
    }

    return await db
      .update(UserPersonalIdentities)
      .set({
        name: resp.name,
        birthday,
        phoneNumber: resp.phoneNumber,
        ci: resp.ci,
        kind: UserPersonalIdentityKind.PHONE,
        expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
      })
      .where(eq(UserPersonalIdentities.id, existingIdentity.id))
      .returning()
      .then(firstOrThrow);
  }

  return await db
    .insert(UserPersonalIdentities)
    .values({
      userId,
      name: resp.name,
      birthday,
      phoneNumber: resp.phoneNumber,
      ci: resp.ci,
      kind: UserPersonalIdentityKind.PHONE,
      expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
    })
    .returning()
    .then(firstOrThrow);
};
