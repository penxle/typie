import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, cookies }) => {
  const accessToken = cookies.get('typie-at');
  const deviceId = cookies.get('typie-did');

  return await fetch(`${env.PUBLIC_API_URL}/graphql`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
      ...(deviceId ? { 'X-Device-Id': deviceId } : {}),
    },
    body: await request.blob(),
  });
};
