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

  const bootstrapBypass = cookies.get('typie-bb');

  const response = await fetch(`${env.PRIVATE_API_URL}/graphql`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Connection: 'close',
      'X-Client-IP': getClientAddress(),
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
      ...(deviceId ? { 'X-Device-Id': deviceId } : {}),
      ...(bootstrapBypass ? { 'X-Bootstrap-Bypass': bootstrapBypass } : {}),
    },
    body: await request.arrayBuffer(),
  });

  if (response.status === 401) {
    cookies.delete('typie-at', { path: '/' });
  }

  const responseBody = await response.arrayBuffer();
  const responseHeaders = new Headers(response.headers);

  return new Response(responseBody, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};
