/* eslint-disable @typescript-eslint/no-non-null-assertion */

import type { WebViewCanvasPage_Query_Variables } from './$graphql';

export const _WebViewCanvasPage_Query_Variables: WebViewCanvasPage_Query_Variables = ({ url }) => ({
  slug: url.searchParams.get('slug')!,
  siteId: url.searchParams.get('siteId')!,
});
