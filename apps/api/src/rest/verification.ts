import { Hono } from 'hono';
import { env } from '@/env';
import { finalizeIdentityVerificationByPhone } from '@/utils/identity-verification';
import type { Env } from '@/context';

export const verification = new Hono<Env>();

verification.get('/callback', async (c) => {
  const { identityVerificationId } = c.req.query();
  const ctx = c.var.context;

  if (!ctx.session || !identityVerificationId) {
    return c.redirect(env.WEBSITE_URL);
  }

  await finalizeIdentityVerificationByPhone({
    userId: ctx.session.userId,
    identityVerificationId,
  });

  return c.redirect(env.WEBSITE_URL);
});
