import dayjs from 'dayjs';
import { nanoid } from 'nanoid';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
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

  const response = await fetch(`${env.PRIVATE_API_URL}/graphql`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Client-IP': getClientAddress(),
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
      ...(deviceId ? { 'X-Device-Id': deviceId } : {}),
    },
    body: request.body,
    // @ts-expect-error Node type issues
    duplex: 'half',
  });

  if (response.status === 401) {
    cookies.delete('typie-at', { path: '/' });
  }

  const responseBody = response.body;
  const responseHeaders = new Headers(response.headers);

  return new Response(responseBody, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};
