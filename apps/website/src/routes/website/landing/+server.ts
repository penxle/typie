import type { RequestHandler } from './$types';

const handler: RequestHandler = async ({ request, url, fetch }) => {
  const targetUrl = new URL('https://typie.framer.ai');
  targetUrl.search = url.search;

  const requestHeaders = new Headers(request.headers);
  requestHeaders.delete('Host');
  requestHeaders.delete('Accept-Encoding');

  const response = await fetch(targetUrl, {
    method: request.method,
    headers: requestHeaders,
    body: request.method === 'POST' ? await request.blob() : undefined,
    redirect: 'manual',
  });

  const responseHeaders = new Headers(response.headers);
  responseHeaders.delete('Content-Length');
  responseHeaders.delete('Content-Encoding');
  responseHeaders.delete('Transfer-Encoding');

  let body = null;
  if ((response.status >= 200 && response.status <= 299) || (response.status >= 400 && response.status <= 599)) {
    const responseBody = await response.text();
    body = responseBody.replaceAll('typie.framer.ai', 'typie.co');
  }

  return new Response(body, {
    status: response.status,
    statusText: response.statusText,
    headers: responseHeaders,
  });
};

export { handler as GET, handler as POST };
