import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, url, cookies, getClientAddress }) => {
  const accessToken = cookies.get('typie-at');
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
    cookies.delete('typie-at', { path: '/', domain: url.hostname });
    cookies.delete('typie-at', { path: '/', domain: `.${url.hostname}` });
  }

  return response;
};
