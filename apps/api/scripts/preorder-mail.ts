#!/usr/bin/env bun

import dayjs from 'dayjs';
import { eq, isNull } from 'drizzle-orm';
import { customAlphabet } from 'nanoid';
import { CreditCodes, db, first, firstOrThrow, PreorderUsers } from '@/db';
import { sendEmail } from '@/email';
import { PreorderCodeEmail } from '@/email/templates';
import { delay } from '@/utils/promise';

// cspell:disable-next-line
const generateRedeemCode = customAlphabet('ABCDEFGHJKMNPQRSTUVWXYZ1234567890', 20);

while (true) {
  const preorderUser = await db
    .select({
      id: PreorderUsers.id,
      email: PreorderUsers.email,
    })
    .from(PreorderUsers)
    .where(isNull(PreorderUsers.codeId))
    .limit(1)
    .then(first);

  if (!preorderUser) {
    break;
  }

  await db
    .transaction(async (tx) => {
      const code = await tx
        .insert(CreditCodes)
        .values({
          code: generateRedeemCode(),
          amount: 4900,
          expiresAt: dayjs().add(1, 'year'),
        })
        .returning({
          id: CreditCodes.id,
          code: CreditCodes.code,
        })
        .then(firstOrThrow);

      await tx
        .update(PreorderUsers)
        .set({
          codeId: code.id,
        })
        .where(eq(PreorderUsers.id, preorderUser.id));

      await sendEmail({
        subject: '타이피 정식 출시 안내',
        recipient: preorderUser.email,
        body: PreorderCodeEmail({
          code: code.code.match(/.{1,4}/g)?.join('-') ?? code.code,
        }),
      });

      console.log(`Sent email to ${preorderUser.email}`);
    })
    .catch((err) => {
      console.error(err);
      console.error(`Failed to send email to ${preorderUser.email}!!`);
    });

  await delay(100);
}

process.exit(0);
