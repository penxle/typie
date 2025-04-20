import { redirect } from '@sveltejs/kit';
import dayjs from 'dayjs';
import { env as privateEnv } from '$env/dynamic/private';
import { env as publicEnv } from '$env/dynamic/public';
import { deserializeOAuthState } from '$lib/utils';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, cookies }) => {
  const code = url.searchParams.get('code');
  const error = url.searchParams.get('error');
  const state = url.searchParams.get('state');

  if ((!code && !error) || !state) {
    return new Response('Missing required parameters', { status: 400 });
  }

  const { redirect_uri } = deserializeOAuthState(state);

  if (!redirect_uri) {
    return new Response('Missing redirect URI', { status: 400 });
  }

  if (error === 'login_required') {
    cookies.set('typie-af', 'true', {
      path: '/',
      httpOnly: false,
      secure: true,
      sameSite: 'lax',
      expires: dayjs().add(1, 'day').toDate(),
    });

    redirect(302, redirect_uri);
  }

  if (!code) {
    return new Response('Missing code', { status: 400 });
  }

  const response = await fetch(`${publicEnv.PUBLIC_API_URL}/auth/token`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({
      code,
      grant_type: 'authorization_code',
      redirect_uri: `${url.origin}${url.pathname}`,
      client_id: publicEnv.PUBLIC_OIDC_CLIENT_ID,
      client_secret: privateEnv.PRIVATE_OIDC_CLIENT_SECRET,
    }),
  });

  if (!response.ok) {
    return response;
  }

  const data = await response.json();

  cookies.set('typie-at', data.access_token, {
    path: '/',
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
    expires: dayjs().add(1, 'year').toDate(),
  });

  redirect(302, redirect_uri);
};
