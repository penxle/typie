#!/usr/bin/env node

import { eq } from 'drizzle-orm';
import { CreditCodes, db, firstOrThrow, PreorderUsers } from '@/db';
import { sendEmail } from '@/email';
import { PreorderCodeEmail } from '@/email/templates';

if (!process.argv[2]) {
  console.error('Usage: node scripts/resend-preorder-mail.ts <email>');
  process.exit(1);
}

const preorderUser = await db
  .select({ id: PreorderUsers.id, email: PreorderUsers.email, code: CreditCodes.code })
  .from(PreorderUsers)
  .innerJoin(CreditCodes, eq(PreorderUsers.codeId, CreditCodes.id))
  .where(eq(PreorderUsers.email, process.argv[2]))
  .then(firstOrThrow);

await sendEmail({
  subject: '타이피 정식 출시 안내',
  recipient: preorderUser.email,
  body: PreorderCodeEmail({
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    code: preorderUser.code.match(/.{1,4}/g)!.join('-'),
  }),
});

console.log(`Sent email to ${preorderUser.email}`);
process.exit(0);
