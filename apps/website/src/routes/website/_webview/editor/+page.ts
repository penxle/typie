import { error } from '@sveltejs/kit';
import type { WebViewEditorPage_Query_Variables } from './$graphql';

export const _WebViewEditorPage_Query_Variables: WebViewEditorPage_Query_Variables = ({ url }) => {
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
