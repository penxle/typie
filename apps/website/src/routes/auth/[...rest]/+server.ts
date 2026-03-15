import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

const handler: RequestHandler = async ({ url, request, params }) => {
  const requestHeaders = new Headers(request.headers);
  requestHeaders.delete('Host');
  requestHeaders.delete('Accept-Encoding');

  requestHeaders.set('Connection', 'close');

  const response = await fetch(`${env.PRIVATE_API_URL}/auth/${params.rest}${url.search}`, {
    method: request.method,
    headers: requestHeaders,
    body: request.method === 'POST' ? request.body : undefined,
    redirect: 'manual',
    // @ts-expect-error -- required for streaming request bodies
    duplex: 'half',
  });

  const responseHeaders = new Headers(response.headers);

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};

export { handler as GET, handler as POST };
