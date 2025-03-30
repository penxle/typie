import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const lines = ['User-agent: *'];

  if (env.PUBLIC_PULUMI_STACK === 'prod') {
    lines.push('Disallow:');
  } else {
    lines.push('Disallow: /');
  }

  lines.push(`Sitemap: https://${event.url.host}/sitemap.xml`);

  return new Response(lines.join('\n'), {
    headers: {
      'Content-Type': 'text/plain',
    },
  });
};
