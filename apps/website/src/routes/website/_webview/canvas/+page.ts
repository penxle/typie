import { error } from '@sveltejs/kit';
import type { WebViewCanvasPage_Query_Variables } from './$graphql';

export const _WebViewCanvasPage_Query_Variables: WebViewCanvasPage_Query_Variables = ({ url }) => {
  const slug = url.searchParams.get('slug');
  const siteId = url.searchParams.get('siteId');

  if (!slug || !siteId) {
    error(404);
  }

  return {
    slug,
    siteId,
  };
};
