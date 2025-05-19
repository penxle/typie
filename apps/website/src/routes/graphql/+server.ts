import dayjs from 'dayjs';
import { nanoid } from 'nanoid';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, cookies, getClientAddress }) => {
  const accessToken = cookies.get('typie-at');
  const deviceId = cookies.get('typie-did');

  if (!deviceId) {
    cookies.set('typie-did', nanoid(32), {
      path: '/',
      httpOnly: true,
      secure: !dev,
      sameSite: 'lax',
      expires: dayjs().add(1, 'year').toDate(),
    });
  }

  const response = await fetch(`${env.PUBLIC_API_URL}/graphql`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Client-IP': getClientAddress(),
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
      ...(deviceId ? { 'X-Device-Id': deviceId } : {}),
    },
    body: await request.blob(),
  });

  if (response.status === 401) {
    cookies.delete('typie-at', { path: '/' });
  }

  const responseHeaders = new Headers(response.headers);
  responseHeaders.delete('Content-Encoding');
  responseHeaders.delete('Transfer-Encoding');

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};
