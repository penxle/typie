import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, cookies, getClientAddress }) => {
  const accessToken = cookies.get('typie-at2');
  const deviceId = cookies.get('typie-did');

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
    cookies.delete('typie-at2', { path: '/' });
  }

  const headers = new Headers(response.headers);
  headers.delete('Transfer-Encoding');

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
};
