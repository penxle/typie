import dayjs from 'dayjs';
import { nanoid } from 'nanoid';
import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, cookies, getClientAddress }) => {
  const accessToken = cookies.get('typie-at');

  let deviceId = cookies.get('typie-did');
  if (!deviceId) {
    deviceId = nanoid(32);
    cookies.set('typie-did', deviceId, {
      path: '/',
      httpOnly: true,
      secure: !dev,
      sameSite: 'lax',
      expires: dayjs().add(1, 'year').toDate(),
    });
  }

  const bootstrapBypass = cookies.get('typie-bb');

  const userAgent = request.headers.get('user-agent') ?? '';

  const platform = /iPhone|iPad|iPod/.test(userAgent) ? 'IOS' : /Android/.test(userAgent) ? 'ANDROID' : 'WEB';

  const browser = /Edg\//.test(userAgent)
    ? 'Edge'
    : /Chrome\//.test(userAgent)
      ? 'Chrome'
      : /Firefox\//.test(userAgent)
        ? 'Firefox'
        : /Safari\//.test(userAgent)
          ? 'Safari'
          : 'Web';

  const os = /Mac OS X/.test(userAgent)
    ? 'macOS'
    : /Windows/.test(userAgent)
      ? 'Windows'
      : /Linux/.test(userAgent)
        ? 'Linux'
        : /Android/.test(userAgent)
          ? 'Android'
          : /iPhone|iPad|iPod/.test(userAgent)
            ? 'iOS'
            : 'Web';

  const deviceName = `${browser} on ${os}`;

  const response = await fetch(`${env.PRIVATE_API_URL}/graphql`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Client-IP': getClientAddress(),
      'X-Device-Id': deviceId,
      'X-Device-Name': deviceName,
      'X-Device-Platform': platform,
      ...(accessToken && { Authorization: `Bearer ${accessToken}` }),
      ...(bootstrapBypass && { 'X-Bootstrap-Bypass': bootstrapBypass }),
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
