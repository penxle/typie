import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

const handler: RequestHandler = async ({ url, request, params }) => {
  const requestHeaders = new Headers(request.headers);
  requestHeaders.delete('Host');
  requestHeaders.delete('Accept-Encoding');

  const response = await fetch(`${env.PUBLIC_API_URL}/auth/${params.rest}${url.search}`, {
    method: request.method,
    headers: requestHeaders,
    body: request.method === 'POST' ? await request.blob() : undefined,
    redirect: 'manual',
  });

  const responseHeaders = new Headers(response.headers);
  responseHeaders.delete('Content-Length');
  responseHeaders.delete('Content-Encoding');
  responseHeaders.delete('Transfer-Encoding');

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};

export { handler as GET, handler as POST };
