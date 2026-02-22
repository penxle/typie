import { createHash } from 'node:crypto';
import { fail, redirect } from '@sveltejs/kit';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, cookies }) => {
    const data = await request.formData();
    const key = data.get('key');

    if (!env.PRIVATE_BOOTSTRAP_BYPASS_KEY || key !== env.PRIVATE_BOOTSTRAP_BYPASS_KEY) {
      return fail(400, { error: 'Invalid key' });
    }

    const hash = createHash('sha256').update(env.PRIVATE_BOOTSTRAP_BYPASS_KEY).digest('hex');

    cookies.set('typie-bb', hash, {
      path: '/',
      httpOnly: true,
      secure: !dev,
      sameSite: 'lax',
      maxAge: 60 * 60,
    });

    redirect(302, '/');
  },
};
