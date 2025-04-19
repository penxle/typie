import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

const handler: RequestHandler = async ({ url, request, params, fetch }) => {
  const headers = new Headers(request.headers);
  headers.delete('host');

  const response = await fetch(`${env.PUBLIC_API_URL}/auth/${params.rest}${url.search}`, {
    method: request.method,
    headers,
    body: request.method === 'POST' ? await request.blob() : undefined,
    redirect: 'manual',
  });

  return new Response(response.body, {
    status: response.status,
    headers: response.headers,
  });
};

export { handler as GET, handler as POST };
